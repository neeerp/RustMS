//! One party's run through an event in its own copy of the event's maps
//! (`net::events`). The directory picks the team and reserves the event on
//! the channel; this actor gives each member a ticket into the copy, keeps
//! the time, and when the event ends sends those still in it out, closes
//! the copy's fields and frees the event for the next party.
//!
//! Members' client actors do the moving: a ticket makes `change_field` use
//! the copy's fields for the event's maps, and leaving those maps, the
//! party or the game hands the ticket back (`EventMessage::Left`). A ticket
//! dropped any other way, say with a client that went away before reading
//! it, hands itself back.
//!
//! The stages cleared are shared with the tickets, so a member's client
//! actor can tell at once whether a stage's way on is open. Clearing the
//! last stage wins the event: its clock stops, the next party may start
//! their own, the team goes to the event's clear map if it has one, and it
//! goes on until the last member leaves its maps. A won event's leader may
//! take the team to its bonus, or the win takes it there at once; the
//! bonus runs on a clock of its own.
//!
//! A squad's event (`Event::squad`) is not a party's: it ends only when its
//! last member leaves, and those on its roster who left may come back
//! while it runs (`EventMessage::Join`). A party's event may end that way
//! too (`Event::until_empty`, Papulatus's): leaving the party then keeps
//! the ticket, and a new leader ends nothing. One with a gate keeps it shut
//! until it is won or over.

use crate::actor::registry::FieldRegistry;
use crate::message::{
    ClientHandle, DirectoryMessage, FieldHandle, FieldKey, FieldMessage, ServerMessage, SquadAnswer,
};
use crate::supervisor::spawn_supervised;
use net::events::{
    draw_combo, draw_question, Balloon, Cloto, Event, EventCommand, EventKind, Puzzles, Stage1,
    Team, ZPQ_DOCUMENTS,
};
use net::ClientId;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, sleep_until, Instant};
use tracing::{debug, info};

/// How long members have to get out after the event ends before its fields
/// close.
const GRACE: Duration = Duration::from_secs(2);
const EVENT_QUEUE_CAPACITY: usize = 64;

#[derive(Debug)]
pub enum EventMessage {
    /// `client_id` left the event's maps, the party, or the game.
    Left { client_id: ClientId },
    /// `client_id` talks to Cloto on `stage`; she has nothing to say on a
    /// stage without a part for her.
    Cloto {
        client_id: ClientId,
        stage: u8,
        reply: oneshot::Sender<Option<Cloto>>,
    },
    /// `client_id` talks to the NPC `stage` is handed in to.
    Balloon {
        client_id: ClientId,
        stage: u8,
        reply: oneshot::Sender<Option<Balloon>>,
    },
    /// `client_id`'s script did something the event keeps track of.
    Command {
        client_id: ClientId,
        command: EventCommand,
    },
    /// `member`, on a squad's roster, comes back into its event: answered
    /// with a ticket, or refused.
    Join {
        member: ClientHandle,
        reply: oneshot::Sender<SquadAnswer>,
    },
}

/// The stages an event's party has cleared, one bit each. Only the event
/// writes it.
#[derive(Debug, Default)]
pub struct Cleared(AtomicU16);

impl Cleared {
    pub fn has(&self, stage: u8) -> bool {
        bit(stage).is_some_and(|bit| self.0.load(Ordering::Acquire) & bit != 0)
    }

    /// Returns whether `stage` was not cleared before.
    fn set(&self, stage: u8) -> bool {
        bit(stage).is_some_and(|bit| self.0.fetch_or(bit, Ordering::Release) & bit == 0)
    }
}

fn bit(stage: u8) -> Option<u16> {
    stage
        .checked_sub(1)
        .and_then(|shift| 1u16.checked_shl(shift.into()))
}

/// A way to reach one running event.
#[derive(Clone, Debug)]
pub struct EventHandle {
    tx: mpsc::Sender<EventMessage>,
}

/// What a member holds while in the event: the event's own fields for its
/// maps, and when it ends. Dropping it leaves the event.
#[derive(Debug)]
pub struct EventTicket {
    pub id: u32,
    pub kind: EventKind,
    /// When the time shown on the event's maps runs out; none once it is
    /// won, until a bonus starts.
    pub clock: Option<Instant>,
    member: ClientId,
    event: EventHandle,
    cleared: Arc<Cleared>,
    /// Who leads the team, shared with the event, which hands it on.
    leader: Arc<AtomicI32>,
    /// The item buff the member gets on arrival and loses on leaving, if
    /// the team earned one (`Event::blessing`).
    pub blessing: Option<i32>,
    /// Whether the member took their share of the prize
    /// (`EventCommand::PrizeTaken`).
    pub prize_taken: bool,
}

impl Drop for EventTicket {
    fn drop(&mut self) {
        let left = EventMessage::Left {
            client_id: self.member,
        };
        // A squad's thirty may fill the queue at once (a stage cleared, then
        // all leaving): a member still counted could never come back.
        if let Err(mpsc::error::TrySendError::Full(left)) = self.event.tx.try_send(left) {
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                let tx = self.event.tx.clone();
                runtime.spawn(async move {
                    let _ = tx.send(left).await;
                });
            }
        }
    }
}

impl EventTicket {
    /// Whether `map_id` is one of the event's maps, played in its copy.
    pub fn holds(&self, map_id: i32) -> bool {
        self.kind.event().holds(map_id)
    }

    /// Whether the member leads the team.
    pub fn leader(&self) -> bool {
        self.leader.load(Ordering::Relaxed) == self.member
    }

    /// Whether the party has cleared `stage`.
    pub fn cleared(&self, stage: u8) -> bool {
        self.cleared.has(stage)
    }

    /// Whether the party has cleared the last stage, so has no time limit.
    pub fn won(&self) -> bool {
        self.cleared(self.kind.event().last_stage())
    }

    /// Where the member stands with Cloto on `stage`, or `None` if she has
    /// nothing to say there or the event is over.
    pub async fn cloto(&self, stage: u8) -> Option<Cloto> {
        let (reply, view) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Cloto {
                client_id: self.member,
                stage,
                reply,
            })
            .await
            .ok()?;
        view.await.ok().flatten()
    }

    /// Where the member stands with the NPC `stage` is handed in to, or
    /// `None` if the event is over.
    pub async fn balloon(&self, stage: u8) -> Option<Balloon> {
        let (reply, view) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Balloon {
                client_id: self.member,
                stage,
                reply,
            })
            .await
            .ok()?;
        view.await.ok().flatten()
    }

    /// Tell the event what the member's script did.
    pub async fn command(&self, command: EventCommand) {
        let _ = self
            .event
            .tx
            .send(EventMessage::Command {
                client_id: self.member,
                command,
            })
            .await;
    }
}

/// An event the directory reserved for a party: `id` on `channel_id`, with
/// the members who go.
#[derive(Debug)]
pub struct EventStart {
    pub id: u32,
    pub kind: EventKind,
    pub channel_id: u8,
    pub leader: ClientId,
    pub team: Vec<ClientHandle>,
    /// The item buff everyone gets on arrival, if the team earned one.
    pub blessing: Option<i32>,
    pub queue: EventQueue,
    /// A squad's whole roster, who may come in while it runs, those who
    /// stayed behind at the start too.
    pub roster: Vec<ClientId>,
}

/// A queue for an event about to start, made by the directory so it can
/// keep a way to it (`EventQueue::joiner`).
#[derive(Debug)]
pub struct EventQueue {
    tx: mpsc::Sender<EventMessage>,
    rx: mpsc::Receiver<EventMessage>,
}

impl Default for EventQueue {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(EVENT_QUEUE_CAPACITY);
        Self { tx, rx }
    }
}

impl EventQueue {
    /// A way to the event that doesn't keep it going.
    pub fn joiner(&self) -> EventJoiner {
        EventJoiner(self.tx.downgrade())
    }
}

/// A way for those on a squad's roster back into its event, while it runs.
#[derive(Clone, Debug)]
pub struct EventJoiner(mpsc::WeakSender<EventMessage>);

impl EventJoiner {
    /// Ask the event to take `member` back, answering `reply`. Returns
    /// `reply` if the event is over or busy.
    pub fn join(
        &self,
        member: ClientHandle,
        reply: oneshot::Sender<SquadAnswer>,
    ) -> Option<oneshot::Sender<SquadAnswer>> {
        let message = EventMessage::Join { member, reply };
        let Some(tx) = self.0.upgrade() else {
            return taken_back(message);
        };
        match tx.try_send(message) {
            Ok(()) => None,
            Err(error) => taken_back(error.into_inner()),
        }
    }
}

fn taken_back(message: EventMessage) -> Option<oneshot::Sender<SquadAnswer>> {
    match message {
        EventMessage::Join { reply, .. } => Some(reply),
        _ => None,
    }
}

/// Start the event's actor, which hands out the tickets.
pub fn spawn(
    start: EventStart,
    fields: Arc<FieldRegistry>,
    directory: mpsc::Sender<DirectoryMessage>,
) {
    let (id, kind, channel_id) = (start.id, start.kind, start.channel_id);
    fields.open_instance(channel_id, id);
    shut_gate(&fields, channel_id, kind);
    let EventQueue { tx, rx } = start.queue;
    let everyone = Arc::new(Mutex::new(start.team.clone()));
    let cleared: Arc<Cleared> = Arc::default();
    let won = cleared.clone();
    let actor = EventActor {
        id,
        kind,
        channel_id,
        ends: Some(Instant::now() + kind.event().duration()),
        joiner: tx.downgrade(),
        event: Some(EventHandle { tx }),
        rx,
        team: {
            let mut team = Team::new(
                kind.event(),
                start.leader,
                start.team.iter().map(|m| m.client_id).collect(),
            );
            team.enlist(&start.roster);
            team
        },
        members: start
            .team
            .into_iter()
            .map(|member| (member.client_id, member))
            .collect(),
        stage1: Stage1::default(),
        puzzles: Puzzles::default(),
        heard: HashSet::new(),
        cleared,
        leader: Arc::new(AtomicI32::new(start.leader)),
        directory: directory.clone(),
        fields: fields.clone(),
        everyone: everyone.clone(),
        failed: false,
        bonus_taken: false,
        blessing: start.blessing,
    };
    // However it ends, even by a panic, its members are sent out, the copy
    // closes and the next party may go.
    spawn_supervised(format!("event {kind:?} {id}"), actor.run(), move || {
        let everyone = everyone
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for member in everyone.iter() {
            member.send(ServerMessage::EventEnded { id });
        }
        fields.close_instance(channel_id, id);
        // A win opened it already, and the next squad may have shut it.
        let gate = (!won.has(kind.event().last_stage())).then_some(&fields);
        free_lobby(&directory, gate, channel_id, kind, id);
    });
}

/// The gates of `kind`'s event on `channel_id`, if it has any: their
/// fields and reactors' names.
fn gate(
    fields: &Arc<FieldRegistry>,
    channel_id: u8,
    kind: EventKind,
) -> Vec<(FieldHandle, &'static str)> {
    kind.event()
        .gate
        .iter()
        .map(|&(map_id, name)| (fields.field(FieldKey::new(channel_id, map_id)), name))
        .collect()
}

/// Set every reactor of `gate` to `state`.
async fn set_gate(gate: Vec<(FieldHandle, &'static str)>, state: u8) {
    for (field, name) in gate {
        let _ = field
            .tx
            .send(FieldMessage::SetReactor { name, state })
            .await;
    }
}

/// Shut the gates of `kind`'s event on `channel_id`, if it has any (Cosmic
/// `updateGateState(1)`).
fn shut_gate(fields: &Arc<FieldRegistry>, channel_id: u8, kind: EventKind) {
    let gate = gate(fields, channel_id, kind);
    if !gate.is_empty() {
        tokio::spawn(set_gate(gate, 1));
    }
}

/// Tell the directory event `id` no longer holds its lobby, so the next
/// party may go, opening the event's gates first if `open` has the fields
/// (so the next squad shuts them only after). Said again later, it changes
/// nothing.
fn free_lobby(
    directory: &mpsc::Sender<DirectoryMessage>,
    open: Option<&Arc<FieldRegistry>>,
    channel_id: u8,
    kind: EventKind,
    id: u32,
) {
    let directory = directory.clone();
    let gate = open.map_or_else(Vec::new, |fields| gate(fields, channel_id, kind));
    tokio::spawn(async move {
        set_gate(gate, 0).await;
        let _ = directory
            .send(DirectoryMessage::EventClosed {
                channel_id,
                kind,
                id,
            })
            .await;
    });
}

struct EventActor {
    id: u32,
    kind: EventKind,
    channel_id: u8,
    /// None once the event is won.
    ends: Option<Instant>,
    /// Ours until the tickets are out; then only they keep the queue open.
    event: Option<EventHandle>,
    /// For the tickets of those who come back (`EventMessage::Join`).
    joiner: mpsc::WeakSender<EventMessage>,
    rx: mpsc::Receiver<EventMessage>,
    team: Team,
    members: HashMap<ClientId, ClientHandle>,
    stage1: Stage1,
    puzzles: Puzzles,
    /// The hand-in stages whose NPC the leader has heard out.
    heard: HashSet<u8>,
    cleared: Arc<Cleared>,
    /// The team's leader, for the tickets.
    leader: Arc<AtomicI32>,
    directory: mpsc::Sender<DirectoryMessage>,
    fields: Arc<FieldRegistry>,
    /// Everyone ever in it, those who came back too, to be sent out
    /// however it ends.
    everyone: Arc<Mutex<Vec<ClientHandle>>>,
    /// The party lost what it protected, or its lead went to someone not
    /// in it, which ends the event.
    failed: bool,
    /// The leader took the team to the bonus.
    bonus_taken: bool,
    blessing: Option<i32>,
}

impl EventActor {
    async fn run(mut self) {
        let (id, kind) = (self.id, self.kind);
        info!(?kind, id, members = ?self.team.members(), blessing = ?self.blessing, "Event started");
        if let Some(event) = self.event.take() {
            for member in self.members.values() {
                // A ticket that doesn't arrive hands itself back.
                let ticket = self.ticket(member.client_id, event.clone());
                member.send(ServerMessage::EventStarted(ticket));
            }
        }
        let mut over = false;
        while !over {
            let ends = self.ends;
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(EventMessage::Left { client_id }) => over = self.team.leave(client_id),
                    Some(EventMessage::Cloto { client_id, stage, reply }) => {
                        let _ = reply.send(self.cloto(client_id, stage));
                    }
                    Some(EventMessage::Balloon { client_id, stage, reply }) => {
                        let _ = reply.send(self.balloon(client_id, stage));
                    }
                    Some(EventMessage::Command { client_id, command }) => {
                        self.command(client_id, command);
                        over = self.failed;
                    }
                    Some(EventMessage::Join { member, reply }) => {
                        let _ = reply.send(self.join(member));
                    }
                    None => over = true,
                },
                _ = until(ends) => over = self.time_up(),
            }
        }
        info!(?kind, id, "Event ended");
        // Nothing is answered from here on: a member asking Cloto hears
        // nothing at once rather than after the grace.
        drop(self.rx);
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventEnded { id });
            }
        }
        // Give them time to get out before the copy closes.
        sleep(GRACE).await;
    }

    fn ticket(&self, member: ClientId, event: EventHandle) -> EventTicket {
        EventTicket {
            id: self.id,
            kind: self.kind,
            clock: self.ends,
            member,
            event,
            cleared: self.cleared.clone(),
            leader: self.leader.clone(),
            blessing: self.blessing,
            prize_taken: false,
        }
    }

    /// `member` of a squad's roster comes back while it is not won. A
    /// ticket that doesn't arrive hands itself back.
    fn join(&mut self, member: ClientHandle) -> SquadAnswer {
        let Some(tx) = self.joiner.upgrade() else {
            return SquadAnswer::Refused;
        };
        if !self.team.rejoin(member.client_id) {
            return SquadAnswer::Refused;
        }
        info!(kind = ?self.kind, id = self.id, member = member.client_id, "Member came back");
        let ticket = self.ticket(member.client_id, EventHandle { tx });
        self.everyone
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(member.clone());
        self.members.insert(member.client_id, member);
        SquadAnswer::Rejoin(ticket)
    }

    fn cloto(&mut self, client_id: ClientId, stage: u8) -> Option<Cloto> {
        if !self.team.has(client_id) {
            return None;
        }
        let event = self.kind.event();
        if stage == 1 {
            return Some(self.stage1.talk(client_id, &self.team, draw_question));
        }
        if stage == event.last_stage() {
            let cleared = self.cleared.has(stage);
            return Some(event.last_stage_talk(client_id, &self.team, cleared));
        }
        if !event.puzzles.contains(&stage) {
            return None;
        }
        let cleared = self.cleared.has(stage);
        let draw = || draw_stage_combo(event, stage);
        Some(
            self.puzzles
                .talk(stage, client_id, &self.team, cleared, draw),
        )
    }

    /// The leader first hears what the stage asks, and only then may hand
    /// it in or answer its puzzle (Cosmic's `statusStg`); the others are
    /// sent to fetch them.
    fn balloon(&mut self, client_id: ClientId, stage: u8) -> Option<Balloon> {
        if !self.team.has(client_id) {
            return None;
        }
        let event = self.kind.event();
        if event.puzzles.contains(&stage) {
            let cleared = self.cleared.has(stage);
            let draw = || draw_stage_combo(event, stage);
            let talk = self
                .puzzles
                .talk(stage, client_id, &self.team, cleared, draw);
            return Some(match talk {
                Cloto::Cleared => Balloon::Cleared,
                Cloto::Intro { .. } => Balloon::Intro,
                Cloto::Puzzle { combo, .. } => Balloon::Answer { combo },
                _ => Balloon::NotLeader,
            });
        }
        let hand_in = event.hand_in(stage)?;
        Some(if self.cleared.has(stage) {
            Balloon::Cleared
        } else if client_id != self.team.leader() {
            Balloon::NotLeader
        } else if hand_in.intro && self.heard.insert(stage) {
            Balloon::Intro
        } else {
            Balloon::Ready
        })
    }

    fn command(&mut self, client_id: ClientId, command: EventCommand) {
        if !self.team.has(client_id) {
            return;
        }
        let leader = client_id == self.team.leader();
        let event = self.kind.event();
        let puzzles = &event.puzzles;
        let won = self.cleared.has(event.last_stage());
        match command {
            EventCommand::PassGiven => self.stage1.pass_given(client_id),
            // Handed in, by a leader who has heard the stage out.
            EventCommand::ClearStage(stage) if event.hand_in(stage).is_some() => {
                let intro = event.hand_in(stage).is_some_and(|hand_in| hand_in.intro);
                let heard = !intro || self.heard.contains(&stage);
                if !leader || !heard {
                    debug!(stage, client_id, "Clear not taken");
                } else if stage == event.last_stage() {
                    self.win();
                } else {
                    self.clear(stage);
                }
            }
            // Before stage 1's, for an event with only one.
            EventCommand::ClearStage(stage) if leader && stage == event.last_stage() => {
                self.win();
            }
            // Before Kerning City's stage 1, which takes its passes.
            EventCommand::ClearStage(stage) if leader && event.talk_clears.contains(&stage) => {
                self.clear(stage);
            }
            // By any member, whose hit broke the reactor, or whose item
            // set Minerva's statue to rights.
            EventCommand::ClearStage(stage) if event.member_clears.contains(&stage) => {
                if stage == event.last_stage() {
                    self.win();
                } else {
                    self.clear(stage);
                }
            }
            EventCommand::ClearStage(1) if leader => {
                if self.stage1.clear() {
                    self.clear(1);
                }
            }
            EventCommand::ClearStage(stage)
                if leader && puzzles.contains(&stage) && self.puzzles.set(stage) =>
            {
                self.clear(stage);
            }
            EventCommand::ClearStage(stage) => debug!(stage, client_id, "Clear not taken"),
            EventCommand::Fail if !won => {
                info!(kind = ?self.kind, id = self.id, "Event failed");
                self.failed = true;
            }
            EventCommand::Fail => {}
            EventCommand::StartBonus if leader && won && !self.bonus_taken => {
                self.start_bonus();
            }
            EventCommand::StartBonus => debug!(client_id, "Bonus not taken"),
            // Told by the old leader or the new one, whichever is first.
            // A squad's lead stays its own, whatever the parties; an event
            // that lasts until it is empty doesn't end with a lead outside.
            EventCommand::HandOver(to) if !event.squad && (leader || client_id == to) => {
                self.leader.store(to, Ordering::Relaxed);
                if self.team.hand_over(to) {
                    info!(kind = ?self.kind, id = self.id, to, "Event's lead left it");
                    self.failed = true;
                }
            }
            EventCommand::HandOver(_) => {}
            EventCommand::MoveTeam(map) if leader && event.maps.contains(&map) => {
                self.move_team(None, map, self.ends);
            }
            EventCommand::MoveTeam(map) => debug!(map, client_id, "Move not taken"),
            EventCommand::WinWithDocuments if leader && self.kind == EventKind::Zpq && !won => {
                self.cleared.set(ZPQ_DOCUMENTS);
                self.win();
            }
            EventCommand::WinWithDocuments => debug!(client_id, "Win not taken"),
            EventCommand::RestartClock if self.kind == EventKind::Horntail && !won => {
                self.restart_clock(net::events::HORNTAIL_FIGHT_MINUTES);
            }
            EventCommand::RestartClock => debug!(client_id, "Restart not taken"),
            // Kept by the member's own ticket.
            EventCommand::PrizeTaken => {}
        }
    }

    /// The leader takes the team to the bonus, on a clock of its own
    /// (Cosmic `1012113`: `startEventTimer`, `warpEventTeam`).
    fn start_bonus(&mut self) {
        let Some(bonus) = &self.kind.event().bonus else {
            return;
        };
        let ends = Instant::now() + bonus.duration();
        self.bonus_taken = true;
        self.ends = Some(ends);
        self.move_team(None, bonus.map, Some(ends));
        info!(kind = ?self.kind, id = self.id, "Bonus started");
    }

    /// The clock starts over with `minutes` left (scaled for testing), on
    /// every member's ticket and screen.
    fn restart_clock(&mut self, minutes: u64) {
        let ends = Instant::now() + net::events::scaled(Duration::from_secs(minutes * 60));
        self.ends = Some(ends);
        info!(kind = ?self.kind, id = self.id, minutes, "Clock restarted");
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventClock {
                    id: self.id,
                    clock: ends,
                });
            }
        }
    }

    /// The time ran out. A bonus's ends, taking whoever is still on its map
    /// to its exit (Cosmic `scheduledTimeout`); otherwise the event ends.
    fn time_up(&mut self) -> bool {
        let bonus = self.kind.event().bonus.as_ref();
        match bonus {
            Some(bonus) if self.bonus_taken => {
                self.ends = None;
                self.move_team(Some(bonus.map), bonus.exit, None);
                info!(kind = ?self.kind, id = self.id, "Bonus over");
                false
            }
            _ => true,
        }
    }

    /// Take the members on `from`, or everyone, to `to`, with the time
    /// shown there.
    fn move_team(&self, from: Option<i32>, to: i32, clock: Option<Instant>) {
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventMoved {
                    id: self.id,
                    from,
                    to,
                    clock,
                });
            }
        }
    }

    /// The leader handed in the last stage's passes: the event is won, its
    /// time no longer runs, and the next party may start theirs in a copy
    /// of their own (Cosmic `clearPQ`, whose `setEventCleared` frees the
    /// lobby).
    fn win(&mut self) {
        if !self.clear(self.kind.event().last_stage()) {
            return;
        }
        self.team.win();
        self.ends = None;
        free_lobby(
            &self.directory,
            Some(&self.fields),
            self.channel_id,
            self.kind,
            self.id,
        );
        info!(kind = ?self.kind, id = self.id, "Event won");
        // After the clear itself, so its EXP and effects come first (Cosmic
        // `clearPQ`).
        let event = self.kind.event();
        if let Some(map) = event.clear_map {
            self.move_team(None, map, None);
        }
        // Ludibrium's takes the team on to its bonus at once (Cosmic
        // `clearPQ`).
        if event.bonus.as_ref().is_some_and(|bonus| bonus.on_win) {
            self.start_bonus();
        }
    }

    /// The way on from `stage` opens before anyone hears of the clear, so a
    /// member who sees it can go through. A stage clears once; returns
    /// whether this cleared it.
    fn clear(&self, stage: u8) -> bool {
        if !self.cleared.set(stage) {
            return false;
        }
        info!(kind = ?self.kind, id = self.id, stage, "Stage cleared");
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::StageCleared { id: self.id, stage });
            }
        }
        true
    }
}

/// A combo for puzzle stage `stage` of `event`, from its map's areas.
fn draw_stage_combo(event: &Event, stage: u8) -> Option<u32> {
    let field = net::get_game_data().ok()?.field(event.stage_map(stage))?;
    draw_combo(field.areas.len(), event.areas_taken)
}

/// Wait until `ends`, or forever without it.
async fn until(ends: Option<Instant>) {
    match ends {
        Some(ends) => sleep_until(ends).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleared_keeps_one_bit_per_stage() {
        let cleared = Cleared::default();
        assert!(cleared.set(1) && cleared.set(3) && !cleared.set(1));
        assert!(cleared.has(1) && !cleared.has(2) && cleared.has(3));
        // Stages are counted from 1, and sixteen fit.
        assert!(!cleared.set(0) && !cleared.set(17));
        assert!(!cleared.has(0) && !cleared.has(17) && !cleared.has(9));
        assert!(cleared.set(9) && cleared.set(16));
        assert!(cleared.has(9) && cleared.has(16));
    }
}
