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

use crate::actor::field_drops::Ownership;
use crate::actor::registry::FieldRegistry;
use crate::message::{
    ClientHandle, DirectoryMessage, FieldHandle, FieldKey, FieldMessage, ServerMessage, SquadAnswer,
};
use crate::supervisor::spawn_supervised;
use game::quest_ex::{CarnivalOutcome, PartyQuestOutcome};
use game_data::server_content::keys;
use game_data::skills::CarnivalSkill;
use net::events::{
    draw_combo, draw_question, htpq_room_key, Balloon, ClockOn, Cloto, Event, EventCommand,
    EventKind, GatherTalk, Puzzles, Ropes, RopesTalk, Stage1, Team, CWK_STEPS, HTPQ_ENTRY,
    HTPQ_KEY_AT, MAZE_COUPONS_WANTED, MAZE_COUPON_EXP, ZPQ_DOCUMENTS,
};
use net::loot::Loot;
use net::packet::build::world::carnival::{
    build_carnival_leave, build_carnival_message, build_carnival_result, build_carnival_start,
    build_obtained_cp, build_party_cp,
};
use net::packet::build::world::effect::{build_field_effect, build_field_sound};
use net::packet::build::world::field::{
    build_ariant_result, build_ariant_score, build_pyramid_gauge, build_session_value,
};
use net::packet::build::world::guild::{build_quest_registrant_left, build_quest_too_few};
use net::packet::build::world::messaging::{build_broadcast, build_item_line, Broadcast};
use net::ClientId;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, Ordering};
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
    /// `client_id` talks to Amos on the Amorian Challenge's rope or
    /// platform `stage`, with monsters about or not and `heads` on each of
    /// its areas; None for anyone not in the event.
    Ropes {
        client_id: ClientId,
        stage: u8,
        mobs: bool,
        heads: Vec<u8>,
        reply: oneshot::Sender<Option<RopesTalk>>,
    },
    /// `client_id` talks to Amos on the Amorian Challenge's gate stage,
    /// the team all `together` at him or not and every gate `open` or not;
    /// None for anyone not in the event.
    Gather {
        client_id: ClientId,
        together: bool,
        open: bool,
        reply: oneshot::Sender<Option<GatherTalk>>,
    },
    /// `client_id` talks to the Guild Quest's statue keeper; None for
    /// anyone not in the event.
    Statues {
        client_id: ClientId,
        reply: oneshot::Sender<Option<net::guild_quest::StatueTalk>>,
    },
    /// `client_id` talks to the Guild Quest's vassals' keeper with
    /// `offering` on the fountain's floor; None for anyone not in the
    /// event.
    Vassals {
        client_id: ClientId,
        offering: Option<[u8; net::guild_quest::VASSALS]>,
        reply: oneshot::Sender<Option<net::guild_quest::VassalTalk>>,
    },
    /// `client_id` talks to Sharen III's soul on the Guild Quest's grave;
    /// None for anyone not in the event.
    Soul {
        client_id: ClientId,
        reply: oneshot::Sender<Option<net::guild_quest::SoulTalk>>,
    },
    /// `client_id`, holding the Rubian or not, talks to its keeper in
    /// Ergoth's room; None for anyone not in the event.
    Rubian {
        client_id: ClientId,
        has_rubian: bool,
        reply: oneshot::Sender<Option<net::guild_quest::RubianTalk>>,
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
    /// How many are in the team now.
    TeamSize { reply: oneshot::Sender<usize> },
    /// `client_id`, named `name`, holds `jewels` Spirit Jewels in an
    /// Ariant battle, the bombs having knocked `lost` out since they last
    /// told.
    AriantJewels {
        client_id: ClientId,
        name: String,
        jewels: i32,
        lost: i32,
    },
    /// A Monster Carnival room's waiting party accepted the challenge of
    /// `members`, led by `leader`, who come in as Maple Blue.
    Challengers {
        members: Vec<ClientHandle>,
        leader: ClientId,
    },
    /// `client_id`, who took a Monster Carnival's ticket, is named `name`,
    /// which the battle shows if they quit or come to lead their side.
    Named { client_id: ClientId, name: String },
    /// The battle field answered `client_id`'s spend of `cost` CP on
    /// `index` of its mob (tab 0) or guardian (tab 2) list: whether it came
    /// out, or why not.
    Summoned {
        client_id: ClientId,
        tab: u8,
        index: u8,
        cost: i32,
        summoned: Result<(), net::carnival::SpendRefusal>,
    },
}

/// What a trip up the Keep's third stage's way up comes to.
#[derive(Debug, PartialEq, Eq)]
enum Climb {
    /// The stage clears.
    Done,
    /// So many more trips are to go.
    ToGo(u8),
    /// The stage is already clear.
    Open,
    /// Enough have climbed, but not every sigil is lit.
    NotAll,
}

/// The stages an event's party has cleared, one bit each, how many of the
/// Pirate Ship's Marks it has handed in (`EventCommand::HandInMarks`), and
/// how many of its chests it has opened (`EventCommand::ChestOpened`).
/// Only the event writes it, but for the Magatia rope rooms that members
/// take.
#[derive(Debug, Default)]
pub struct Cleared {
    stages: AtomicU64,
    marks: AtomicU8,
    chests: AtomicU8,
    /// Romeo or Juliet read the other's letter (`EventCommand::Shocked`).
    shocked: AtomicBool,
    /// Yulete left his office caught off guard
    /// (`EventCommand::YuleteLeft`).
    caught_yulete: AtomicBool,
    /// The escort in the Magatia boss's room fainted
    /// (`EventCommand::EscortFailed`).
    escort_failed: AtomicBool,
    /// The Magatia rope rooms taken, one bit each from the first.
    rooms: AtomicU8,
    /// The Guild Quest's King's Corridor's door took the earring
    /// (`EventCommand::KingGateOpened`).
    king_gate: AtomicBool,
    /// The Amorian Challenge's fierry came out (`EventCommand::FierryOut`).
    fierry: AtomicBool,
    /// The Amorian Challenge's leader handed in the Magik Mirror Shard
    /// (`EventCommand::ShardIn`).
    shard: AtomicBool,
    /// Which of the Amorian Challenge's stage 1 ways its clear opened,
    /// counted from 1; 0 before.
    way: AtomicU8,
    /// Crimsonwood Keep's sigils lit, five bits a stage from the second,
    /// the fifth's its altars (`EventCommand::SigilLit`).
    sigils: AtomicU32,
    /// How many have taken the Keep's third stage's way up, up to
    /// `CWK_STEPS` (`EventCommand::CwkStep`).
    steps: AtomicU8,
    /// How many times the Keep's sixth stage's host has brought mobs out
    /// (`EventCommand::CwkHost`).
    host: AtomicU8,
}

impl Cleared {
    pub fn has(&self, stage: u8) -> bool {
        bit(stage).is_some_and(|bit| self.stages.load(Ordering::Acquire) & bit != 0)
    }

    /// Returns whether `stage` was not cleared before.
    fn set(&self, stage: u8) -> bool {
        bit(stage).is_some_and(|bit| self.stages.fetch_or(bit, Ordering::Release) & bit == 0)
    }

    pub fn marks(&self) -> u8 {
        self.marks.load(Ordering::Acquire)
    }

    pub fn chests(&self) -> u8 {
        self.chests.load(Ordering::Acquire)
    }

    pub fn shocked(&self) -> bool {
        self.shocked.load(Ordering::Acquire)
    }

    pub fn caught_yulete(&self) -> bool {
        self.caught_yulete.load(Ordering::Acquire)
    }

    pub fn escort_failed(&self) -> bool {
        self.escort_failed.load(Ordering::Acquire)
    }

    pub fn king_gate(&self) -> bool {
        self.king_gate.load(Ordering::Acquire)
    }

    pub fn fierry(&self) -> bool {
        self.fierry.load(Ordering::Acquire)
    }

    pub fn shard(&self) -> bool {
        self.shard.load(Ordering::Acquire)
    }

    /// The Amorian Challenge's open way from stage 1 (`net::events::AMORIA_WAYS`).
    pub fn way(&self) -> Option<u8> {
        self.way.load(Ordering::Acquire).checked_sub(1)
    }

    /// Light the Keep's sigil of `job` on `stage`, or on the fifth stage
    /// fill its altar; returns the stage's sigils lit before, or `None` if
    /// it already was.
    fn light_sigil(&self, stage: u8, job: u8) -> Option<u32> {
        let shift = stage.checked_sub(2)?.checked_mul(5)?.checked_add(job)?;
        let bit = 1u32
            .checked_shl(shift.into())
            .filter(|_| job < 5 && stage <= 5)?;
        let before = self.sigils.fetch_or(bit, Ordering::AcqRel);
        (before & bit == 0).then(|| (before >> (shift - job)) & 0b1_1111)
    }

    /// Whether all five of the Keep's sigils on `stage` are lit.
    fn sigils_lit(&self, stage: u8) -> bool {
        let shift = stage.checked_sub(2).and_then(|n| n.checked_mul(5));
        shift.filter(|&shift| shift < 20).is_some_and(|shift| {
            (self.sigils.load(Ordering::Acquire) >> shift) & 0b1_1111 == 0b1_1111
        })
    }

    fn steps(&self) -> u8 {
        self.steps.load(Ordering::Acquire)
    }

    fn host(&self) -> u8 {
        self.host.load(Ordering::Acquire)
    }

    /// The Keep's sixth stage's host moves on from `phase`; returns whether
    /// it was there.
    fn host_heard(&self, phase: u8) -> bool {
        let next = phase.checked_add(1).unwrap_or(phase);
        (self.host)
            .compare_exchange(phase, next, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Count one more up the Keep's third stage's way; returns how many
    /// now have, or `None` if `CWK_STEPS` already had.
    fn step(&self) -> Option<u8> {
        let more = |n: u8| (n < CWK_STEPS).then_some(n + 1);
        let before = self
            .steps
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, more);
        before.ok().map(|n| n + 1)
    }

    /// One more up the Keep's third stage's way (Cosmic `glpqPortal2`):
    /// the last of `CWK_STEPS`, with all the stage's sigils lit, clears it.
    fn climb(&self) -> Climb {
        let lit = self.sigils_lit(3);
        match self.step() {
            Some(CWK_STEPS) if lit => Climb::Done,
            Some(n) => Climb::ToGo(CWK_STEPS - n),
            None if lit => Climb::Open,
            None => Climb::NotAll,
        }
    }

    /// Whether room `room` is taken.
    fn room_taken(&self, room: u8) -> bool {
        1u8.checked_shl(room.into())
            .is_some_and(|bit| self.rooms.load(Ordering::Acquire) & bit != 0)
    }

    /// Take rope room `room`; returns whether no one had.
    pub fn take_room(&self, room: u8) -> bool {
        let Some(bit) = 1u8.checked_shl(room.into()) else {
            return false;
        };
        self.rooms.fetch_or(bit, Ordering::AcqRel) & bit == 0
    }
}

fn bit(stage: u8) -> Option<u64> {
    stage
        .checked_sub(1)
        .and_then(|shift| 1u64.checked_shl(shift.into()))
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
    /// Whether the member heard Ellin out on the forest's fourth stage
    /// (Cosmic `gridInsert`).
    pub hint_heard: bool,
    /// The Guild Quest's waterway room the member last came to the grave
    /// from (`net::guild_quest::grid_way`).
    pub grid: Option<u8>,
    /// The member's side in a Monster Carnival, 0 Maple Red or 1 Maple
    /// Blue, which their look shows others.
    pub side: u8,
    /// When the member was let in, which a party quest's record times its
    /// clear from.
    pub entered: Instant,
    /// How many Spirit Jewels the member last told an Ariant battle they
    /// hold.
    pub jewels_told: Option<i32>,
    /// How many Spirit Jewels a bomb knocked out of the member's bag since
    /// they last told.
    pub jewels_lost: i32,
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
    /// Tell a Monster Carnival the member's name, which the battle shows
    /// if they quit or come to lead their side.
    pub fn named(&self, name: &str) {
        if matches!(self.kind, EventKind::Carnival(_)) {
            let _ = self.event.tx.try_send(EventMessage::Named {
                client_id: self.member,
                name: name.to_string(),
            });
        }
    }

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

    /// How many of the Pirate Ship's Marks the party has handed Guon.
    pub fn marks(&self) -> u8 {
        self.cleared.marks()
    }

    /// How many of the Pirate Ship's chests the party has opened.
    pub fn chests(&self) -> u8 {
        self.cleared.chests()
    }

    /// Whether Romeo or Juliet has read the other's letter.
    pub fn shocked(&self) -> bool {
        self.cleared.shocked()
    }

    /// Whether the Glimmer Man brought out the Amorian Challenge's fierry.
    pub fn fierry(&self) -> bool {
        self.cleared.fierry()
    }

    /// Whether the leader handed the Glimmer Man the Magik Mirror Shard.
    pub fn shard(&self) -> bool {
        self.cleared.shard()
    }

    /// Which of the Amorian Challenge's stage 1 ways is open, if any.
    pub fn way(&self) -> Option<u8> {
        self.cleared.way()
    }

    /// The object on `stage`'s map that shows its way on open once it is
    /// cleared: the stage maps' `gate` (`Event::gates`), on the Amorian
    /// Challenge's first the gate of the way drawn, `gate0` to `gate2`, or
    /// on Crimsonwood Keep's second to fifth their portals, `2pt` to `5pt`.
    pub fn gate_object(&self, stage: u8) -> Option<String> {
        if self.kind == EventKind::Amoria && stage == 1 {
            return self.way().map(|way| format!("gate{way}"));
        }
        if self.kind == EventKind::Cwk {
            return (2..=5).contains(&stage).then(|| format!("{stage}pt"));
        }
        self.kind.event().gates.then(|| "gate".to_string())
    }

    /// Where the member, dead on one of the event's maps, revives instead
    /// of their map's return map: the Guild Quest's way out, which they
    /// leave it by, until the King's Corridor's door takes the earring
    /// (Cosmic `playerRevive` while `canRevive` is 0); Pink Bean's
    /// Forgotten Twilight, in the copy, whose portal leads back to the
    /// fight (Cosmic's wz `returnMap`; Royals's Map.nx sends them out); an
    /// era's Nex's room, the Tera Forest, not its Leafre (Cosmic
    /// `GuardianNex.playerRevive`).
    pub fn revive_to(&self) -> Option<i32> {
        let event = self.kind.event();
        match self.kind {
            EventKind::GuildQuest if !self.cleared.king_gate() => Some(event.exit),
            EventKind::GuardianNex(_) => Some(event.exit),
            EventKind::PinkBean => Some(net::events::PINK_BEAN_REVIVE),
            _ => None,
        }
    }

    /// Whether Yulete left his office caught off guard.
    pub fn caught_yulete(&self) -> bool {
        self.cleared.caught_yulete()
    }

    /// Whether the escort in the Magatia boss's room fainted.
    pub fn escort_failed(&self) -> bool {
        self.cleared.escort_failed()
    }

    /// Take the Magatia rope room `room`, from 0, for the member, or the
    /// weapon of Crimsonwood Keep's fifth stage's room of job `room`;
    /// returns whether no one had (Cosmic `rnj4_r1`'s `statusStg5`,
    /// `9201107`'s `glpq5_room`).
    pub fn take_room(&self, room: u8) -> bool {
        self.cleared.take_room(room)
    }

    /// Whether `take_room(room)` has been.
    pub fn room_taken(&self, room: u8) -> bool {
        self.cleared.room_taken(room)
    }

    /// How many times the Keep's sixth stage's host has brought mobs out.
    pub fn cwk_host(&self) -> u8 {
        self.cleared.host()
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

    /// What Amos makes of the member's talk on the Amorian Challenge's
    /// rope or platform `stage`, with monsters about or not and `heads` on
    /// each of its areas; a right try clears the stage. `None` if the
    /// event is over.
    pub async fn ropes(&self, stage: u8, mobs: bool, heads: Vec<u8>) -> Option<RopesTalk> {
        let (reply, talk) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Ropes {
                client_id: self.member,
                stage,
                mobs,
                heads,
                reply,
            })
            .await
            .ok()?;
        talk.await.ok().flatten()
    }

    /// What Amos makes of the member's talk on the Amorian Challenge's
    /// gate stage, the team all `together` at him or not and every gate
    /// `open` or not. `None` if the event is over.
    pub async fn gather(&self, together: bool, open: bool) -> Option<GatherTalk> {
        let (reply, talk) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Gather {
                client_id: self.member,
                together,
                open,
                reply,
            })
            .await
            .ok()?;
        talk.await.ok().flatten()
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

    /// What the Guild Quest's statue keeper tells the member, or `None` if
    /// the event is over.
    pub async fn statues(&self) -> Option<net::guild_quest::StatueTalk> {
        let (reply, talk) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Statues {
                client_id: self.member,
                reply,
            })
            .await
            .ok()?;
        talk.await.ok().flatten()
    }

    /// What the Guild Quest's vassals' keeper tells the member, who finds
    /// `offering` on the fountain's floor, or `None` if the event is over.
    pub async fn vassals(
        &self,
        offering: Option<[u8; net::guild_quest::VASSALS]>,
    ) -> Option<net::guild_quest::VassalTalk> {
        let (reply, talk) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Vassals {
                client_id: self.member,
                offering,
                reply,
            })
            .await
            .ok()?;
        talk.await.ok().flatten()
    }

    /// What Sharen III's soul tells the member, or `None` if the event is
    /// over.
    pub async fn soul(&self) -> Option<net::guild_quest::SoulTalk> {
        let (reply, talk) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Soul {
                client_id: self.member,
                reply,
            })
            .await
            .ok()?;
        talk.await.ok().flatten()
    }

    /// What the Rubian's keeper tells the member, who holds it or not, or
    /// `None` if the event is over.
    pub async fn rubian(&self, has_rubian: bool) -> Option<net::guild_quest::RubianTalk> {
        let (reply, talk) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Rubian {
                client_id: self.member,
                has_rubian,
                reply,
            })
            .await
            .ok()?;
        talk.await.ok().flatten()
    }

    /// How many are in the team now, or `None` if the event is over.
    pub async fn team_size(&self) -> Option<usize> {
        let (reply, size) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::TeamSize { reply })
            .await
            .ok()?;
        size.await.ok()
    }

    /// Tell an Ariant battle the member, named `name`, holds `jewels`
    /// Spirit Jewels, the bombs having knocked `lost` out since last.
    pub async fn ariant_jewels(&self, name: String, jewels: i32, lost: i32) {
        let _ = (self.event.tx)
            .send(EventMessage::AriantJewels {
                client_id: self.member,
                name,
                jewels,
                lost,
            })
            .await;
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
    /// Whether each member's partner came too (Cosmic AmoriaPQ's
    /// `isTeamAllCouple`), for the couples' bonus.
    pub couples: bool,
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

impl EventJoiner {
    /// Bring the party of `members`, led by `leader`, into a Monster
    /// Carnival room's event as the challengers. False if the event is
    /// over or busy.
    pub fn challengers(&self, members: Vec<ClientHandle>, leader: ClientId) -> bool {
        let message = EventMessage::Challengers { members, leader };
        self.0
            .upgrade()
            .is_some_and(|tx| tx.try_send(message).is_ok())
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
        // So Gong's trial has no time limit; a Monster Carnival room is
        // waited in for a challenge for so long.
        ends: match kind {
            EventKind::Carnival(_) => {
                Some(Instant::now() + net::events::scaled(net::carnival::WAITING))
            }
            // A guild gathers before its quest's clock starts.
            EventKind::GuildQuest => {
                Some(Instant::now() + net::events::scaled(net::guild_quest::GATHERING))
            }
            // An Ariant battle's results come a little before its end.
            EventKind::Ariant(_) => Some(
                Instant::now() + kind.event().duration()
                    - net::events::scaled(net::ariant::RESULTS_BEFORE),
            ),
            _ => (kind.event().minutes > 0).then(|| Instant::now() + kind.event().duration()),
        },
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
        carnival_stage: net::carnival::Stage::default(),
        carnival_summoned: Vec::new(),
        carnival_end: None,
        carnival_outcome: None,
        carnival_overtime: false,
        // The party waiting plays Maple Red.
        carnival: {
            let mut score = net::carnival::Score::default();
            for member in &start.team {
                score.join(member.client_id, 0);
            }
            score.lead(start.leader);
            score
        },
        members: start
            .team
            .into_iter()
            .map(|member| (member.client_id, member))
            .collect(),
        stage1: Stage1::default(),
        puzzles: Puzzles::default(),
        statues: Default::default(),
        vassals: Default::default(),
        heard: HashSet::new(),
        ropes: Ropes::default(),
        rushed_from: None,
        cleared,
        leader: Arc::new(AtomicI32::new(start.leader)),
        directory: directory.clone(),
        fields: fields.clone(),
        everyone: everyone.clone(),
        failed: false,
        bonus_taken: false,
        couples: start.couples,
        couples_taken: false,
        moved_to: Vec::new(),
        reached: HashSet::new(),
        lord_pirate_out: false,
        win_exp: None,
        blessing: start.blessing,
        pyramid_stage: 1,
        tally: net::pyramid::Tally::default(),
        stage_started: Instant::now(),
        began: Instant::now(),
        fallen: HashSet::new(),
        showa_fell: false,
        pink_bean_fallen: 0,
        gauge: None,
        gauge_out: None,
        pyramid_end: None,
        gathering: None,
        closing: false,
        ariant: net::ariant::Scores::default(),
        ariant_board: None,
        ariant_results: false,
        bonus_at: None,
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
    /// The Guild Quest's statue keeper's test.
    statues: net::guild_quest::Statues,
    /// The Guild Quest's vassals' wish and the guild's tries at it.
    vassals: net::guild_quest::Vassals,
    /// The hand-in stages whose NPC the leader has heard out, and the
    /// Amorian Challenge's rope stage.
    heard: HashSet<u8>,
    /// The Amorian Challenge's ropes and the tries at them.
    ropes: Ropes,
    /// When the Amorian Challenge's clock ran out before stage 4 cut it
    /// short; stage 5 gives it back.
    rushed_from: Option<Instant>,
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
    /// Each member's partner came too, and the leader took the team to the
    /// couples' bonus (`Bonus::couples`).
    couples: bool,
    couples_taken: bool,
    /// The maps the team has been moved to (`EventCommand::MoveTeam`).
    moved_to: Vec<i32>,
    /// The maps whose first arrival has started the clock over
    /// (`EventCommand::Reached`).
    reached: HashSet<i32>,
    /// The Lord Pirate is out (`EventCommand::PastTheDoors`).
    lord_pirate_out: bool,
    blessing: Option<i32>,
    /// The EXP every member gets for the win rather than the last stage's
    /// (`EventCommand::WinWithCoupons`).
    win_exp: Option<i32>,
    /// The stage a Nett's Pyramid run is on, which its clock moves on.
    pyramid_stage: u8,
    /// A Nett's Pyramid run's kills, cool kills and misses, and the
    /// stage's gauge (`EventCommand::Massacre`).
    tally: net::pyramid::Tally,
    stage_started: Instant,
    /// When the event was set up, its gathering and all.
    began: Instant,
    /// The members dead in the Guild Quest's last room who haven't
    /// revived (`EventCommand::KingRoomFell`).
    fallen: HashSet<ClientId>,
    /// A member died in Showa's battle before it was won
    /// (`EventCommand::Fell`).
    showa_fell: bool,
    /// How many times Pink Bean's fighters died before it fell.
    pink_bean_fallen: u32,
    /// The stage's gauge rules (`mobMassacre/gauge`).
    gauge: Option<game_data::Massacre>,
    /// When the stage's gauge runs out, as the tally stands.
    gauge_out: Option<Instant>,
    /// How a Nett's Pyramid run ended, for its score: outlasting the last
    /// stage's clock (true) or with its gauge run out (false).
    pyramid_end: Option<bool>,
    /// A Monster Carnival battle's sides and CP.
    carnival: net::carnival::Score,
    carnival_stage: net::carnival::Stage,
    /// How many of each mob of the battle field's list were summoned.
    carnival_summoned: Vec<u8>,
    /// When the battle's clock runs out; `ends` is its result's time, a
    /// little before.
    carnival_end: Option<Instant>,
    /// How the battle ends, once its last seconds come; None for a tie.
    carnival_outcome: Option<net::carnival::Outcome>,
    /// Whether a tie already gave the battle its overtime.
    carnival_overtime: bool,
    /// While a guild gathers for its Guild Quest, which keeps it open with
    /// nobody in yet, and lets its members in.
    gathering: Option<EventHandle>,
    /// The Guild Quest can't go on: it ends when the clock runs out, a few
    /// seconds after its notice.
    closing: bool,
    /// An Ariant battle's scores.
    ariant: net::ariant::Scores,
    /// When an Ariant battle's players next see its changed scores.
    ariant_board: Option<Instant>,
    /// When a won team goes on to its bonus, a while after the win.
    bonus_at: Option<Instant>,
    /// An Ariant battle's results are in: its time ends soon, and a player
    /// leaving doesn't end it while one is left (Cosmic `eventClear`).
    ariant_results: bool,
}

impl EventActor {
    async fn run(mut self) {
        let (id, kind) = (self.id, self.kind);
        info!(?kind, id, members = ?self.team.members(), blessing = ?self.blessing, "Event started");
        if let Some(event) = self.event.take() {
            if kind == EventKind::GuildQuest {
                self.gathering = Some(event.clone());
            }
            for member in self.members.values() {
                // A ticket that doesn't arrive hands itself back.
                let ticket = self.ticket(member.client_id, event.clone());
                member.send(ServerMessage::EventStarted(ticket));
            }
        }
        if self.kind.massacre() {
            self.start_pyramid_stage();
        }
        if self.kind == EventKind::Cwk {
            self.cwk_guardians();
        }
        let mut over = false;
        while !over {
            let ends = self.ends.map(|ends| ends + self.overtime());
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(EventMessage::Left { client_id }) => {
                        let left = self.team.leave(client_id);
                        over = self
                            .carnival_left(client_id)
                            .or_else(|| self.ariant_left())
                            .or_else(|| self.guild_quest_left(client_id, left))
                            .unwrap_or(left);
                    }
                    Some(EventMessage::Challengers { members, leader }) => {
                        self.challengers(members, leader)
                    }
                    Some(EventMessage::Named { client_id, name }) => {
                        self.carnival.name(client_id, &name)
                    }
                    Some(EventMessage::Summoned { client_id, tab, index, cost, summoned }) => {
                        self.carnival_summoned(client_id, tab, index, cost, summoned);
                    }
                    Some(EventMessage::Cloto { client_id, stage, reply }) => {
                        let _ = reply.send(self.cloto(client_id, stage));
                    }
                    Some(EventMessage::Balloon { client_id, stage, reply }) => {
                        let _ = reply.send(self.balloon(client_id, stage));
                    }
                    Some(EventMessage::Ropes { client_id, stage, mobs, heads, reply }) => {
                        let _ = reply.send(self.ropes(client_id, stage, mobs, &heads));
                    }
                    Some(EventMessage::Gather { client_id, together, open, reply }) => {
                        let _ = reply.send(self.gather(client_id, together, open));
                    }
                    Some(EventMessage::Statues { client_id, reply }) => {
                        let _ = reply.send(self.statues(client_id));
                    }
                    Some(EventMessage::Vassals { client_id, offering, reply }) => {
                        let _ = reply.send(self.vassals(client_id, offering));
                    }
                    Some(EventMessage::Soul { client_id, reply }) => {
                        let _ = reply.send(self.soul(client_id));
                    }
                    Some(EventMessage::Rubian { client_id, has_rubian, reply }) => {
                        let _ = reply.send(self.rubian(client_id, has_rubian));
                    }
                    Some(EventMessage::Command { client_id, command }) => {
                        self.command(client_id, command);
                        over = self.failed;
                    }
                    Some(EventMessage::Join { member, reply }) => {
                        let _ = reply.send(self.join(member));
                    }
                    Some(EventMessage::TeamSize { reply }) => {
                        let _ = reply.send(self.team.members().len());
                    }
                    Some(EventMessage::AriantJewels { client_id, name, jewels, lost }) => {
                        self.ariant_jewels(client_id, &name, jewels, lost);
                    }
                    None => over = true,
                },
                _ = until(ends) => over = match self.kind {
                    EventKind::Ariant(arena) => self.ariant_time_up(arena).await,
                    _ => self.time_up(),
                },
                _ = until(self.ariant_board) => self.show_ariant_scores(),
                _ = until(self.bonus_at) => {
                    self.bonus_at = None;
                    self.start_bonus();
                }
                _ = until(self.gauge_out) => over = self.gauge_ran_out(),
            }
        }
        info!(?kind, id, "Event ended");
        let score = self.pyramid_score();
        // Nothing is answered from here on: a member asking Cloto hears
        // nothing at once rather than after the grace.
        drop(self.rx);
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventEnded { id });
            }
        }
        // After `EventEnded`, which takes them to the result map.
        for member in self.team.members() {
            if let (Some(handle), Some((exit, rank, exp, cleared))) =
                (self.members.get(member), score)
            {
                handle.send(ServerMessage::PyramidScore {
                    exit,
                    rank,
                    exp,
                    cleared,
                });
            }
        }
        // Give them time to get out before the copy closes.
        sleep(GRACE).await;
    }

    fn ticket(&self, member: ClientId, event: EventHandle) -> EventTicket {
        EventTicket {
            id: self.id,
            kind: self.kind,
            clock: self.clock(),
            member,
            event,
            cleared: self.cleared.clone(),
            leader: self.leader.clone(),
            blessing: self.blessing,
            prize_taken: false,
            hint_heard: false,
            grid: None,
            side: self.carnival.side(member).unwrap_or(0),
            entered: Instant::now(),
            jewels_told: None,
            jewels_lost: 0,
        }
    }

    /// A Monster Carnival room's waiting party accepted the challenge of
    /// `members`' party, who come in as Maple Blue. Both stand in the
    /// waiting room until the battle, the waiting side seeing the new
    /// clock (Cosmic `startCPQ`). Tickets that don't arrive hand
    /// themselves back.
    fn challengers(&mut self, members: Vec<ClientHandle>, leader: ClientId) {
        let Some(tx) = self.joiner.upgrade() else {
            return;
        };
        if self.carnival_stage != net::carnival::Stage::Waiting {
            return;
        }
        self.carnival_stage = net::carnival::Stage::Ready;
        let ends = Instant::now() + net::events::scaled(net::carnival::READY);
        self.ends = Some(ends);
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventClock {
                    id: self.id,
                    clock: Some(ends),
                });
            }
        }
        info!(kind = ?self.kind, id = self.id, ?members, "Carnival challengers");
        for member in members {
            let id = member.client_id;
            self.team.admit(id);
            self.carnival.join(id, 1);
            let ticket = self.ticket(id, EventHandle { tx: tx.clone() });
            self.everyone
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(member.clone());
            member.send(ServerMessage::EventStarted(ticket));
            self.members.insert(id, member);
        }
        self.carnival.lead(leader);
    }

    /// A Monster Carnival room's time ran out. Waiting, the party goes back
    /// to the Carnival Field; ready, the battle starts, each side told
    /// theirs; in battle, it is over (Cosmic `MonsterCarnival`'s timers).
    fn carnival_time_up(&mut self, room: u8) -> bool {
        use net::carnival::Stage;
        match self.carnival_stage {
            Stage::Waiting => {
                info!(id = self.id, room, "Carnival room not challenged");
                // Free before they arrive, so they may take it again.
                free_lobby(&self.directory, None, self.channel_id, self.kind, self.id);
                self.move_team(None, self.kind.event().lobby, None);
                true
            }
            Stage::Ready => {
                self.carnival_stage = Stage::Battle;
                let ends = Instant::now() + net::carnival::battle_length(room);
                self.carnival_end = Some(ends);
                self.ends = Some(closing_at(ends));
                self.move_team(None, net::carnival::battle_map(room), Some(ends));
                let text = net::content::text();
                for member in self.team.members() {
                    if let Some(handle) = self.members.get(member) {
                        handle.send(ServerMessage::RecordPartyQuest {
                            id: self.id,
                            outcome: PartyQuestOutcome::Entered,
                        });
                    }
                    let key = match self.carnival.side(*member) {
                        Some(1) => keys::CARNIVAL_TEAM_BLUE,
                        _ => keys::CARNIVAL_TEAM_RED,
                    };
                    let line = build_item_line(&text.get(key, &[]), 0);
                    if let (Some(handle), Ok(line)) = (self.members.get(member), line) {
                        handle.send(ServerMessage::EventPackets {
                            id: self.id,
                            packets: vec![line],
                        });
                    }
                }
                info!(id = self.id, room, "Carnival battle started");
                false
            }
            Stage::Battle => self.carnival_close(room),
            Stage::Closing => self.carnival_closed(room),
            Stage::Over => false,
        }
    }

    /// A battle's last seconds came: the side that earned more CP in all
    /// has won, and a tie after the overtime is a draw. Unless it is a tie
    /// still to be played out, the mobs go, and each side sees and hears
    /// how it did and is told in the chat log (Cosmic `complete`, which
    /// sends no MONSTER_CARNIVAL_RESULT).
    fn carnival_close(&mut self, room: u8) -> bool {
        use net::carnival::Outcome;
        self.carnival_stage = net::carnival::Stage::Closing;
        self.ends = self.carnival_end;
        let outcome = match self.carnival.leader() {
            Some(side) => Some(Outcome::Won(side)),
            None if self.carnival_overtime => Some(Outcome::Draw),
            None => None,
        };
        self.carnival_outcome = outcome;
        let Some(outcome) = outcome else {
            return false;
        };
        info!(id = self.id, room, ?outcome, "Carnival decided");
        self.clear_mobs(net::carnival::battle_map(room));
        for member in self.team.members() {
            let (Some(handle), Some(side)) =
                (self.members.get(member), self.carnival.side(*member))
            else {
                continue;
            };
            let (effect, sound) = net::carnival::result_effects(room, outcome, side);
            let packets = [
                build_field_effect(&effect),
                build_field_sound(&sound),
                build_carnival_result(outcome.code(side)),
            ];
            if let Ok(packets) = packets.into_iter().collect() {
                handle.send(ServerMessage::EventPackets {
                    id: self.id,
                    packets,
                });
            }
        }
        false
    }

    /// A battle's clock ran out. A tie goes on for the field's
    /// `timeExpand`, the clock shown again (Cosmic `extendTime`); otherwise
    /// each side goes to its result room with its CP in all, cured, and the
    /// room is free for the next parties (Cosmic `finish`). The event goes
    /// on until the last of them leaves the result rooms.
    fn carnival_closed(&mut self, room: u8) -> bool {
        let Some(outcome) = self.carnival_outcome else {
            self.carnival_overtime = true;
            self.carnival_stage = net::carnival::Stage::Battle;
            let ends = Instant::now() + net::carnival::overtime_length(room);
            self.carnival_end = Some(ends);
            self.ends = Some(closing_at(ends));
            let text = net::content::text().get(keys::CARNIVAL_OVERTIME, &[]);
            let line = build_item_line(&text, 0);
            for member in self.team.members() {
                if let Some(handle) = self.members.get(member) {
                    handle.send(ServerMessage::EventClock {
                        id: self.id,
                        clock: Some(ends),
                    });
                    if let Ok(line) = &line {
                        handle.send(ServerMessage::EventPackets {
                            id: self.id,
                            packets: vec![line.clone()],
                        });
                    }
                }
            }
            info!(id = self.id, room, "Carnival overtime");
            return false;
        };
        self.carnival_stage = net::carnival::Stage::Over;
        self.ends = None;
        self.team.win();
        self.cleared.set(self.kind.event().last_stage());
        free_lobby(&self.directory, None, self.channel_id, self.kind, self.id);
        for member in self.team.members() {
            let (Some(handle), Some(side)) =
                (self.members.get(member), self.carnival.side(*member))
            else {
                continue;
            };
            let result = match outcome {
                net::carnival::Outcome::Draw => CarnivalOutcome::Drew,
                net::carnival::Outcome::Won(winner) if winner == side => CarnivalOutcome::Won,
                net::carnival::Outcome::Won(_) => CarnivalOutcome::Lost,
            };
            handle.send(ServerMessage::CarnivalResult {
                id: self.id,
                won: result == CarnivalOutcome::Won,
                points: self.carnival.total(side),
            });
            handle.send(ServerMessage::RecordPartyQuest {
                id: self.id,
                outcome: PartyQuestOutcome::Carnival(result),
            });
            handle.send(ServerMessage::EventMoved {
                id: self.id,
                from: None,
                to: outcome.room(room, side),
                clock: None,
            });
        }
        info!(id = self.id, room, "Carnival over");
        false
    }

    /// `member` left a Monster Carnival room: the waiting party's leader
    /// takes it back to the Carnival Field, and once challenged, a side
    /// with nobody left ends it, not its leader leaving; once the result
    /// is known it plays out. Whether that ends the event, or None to go
    /// by the team's own rule.
    fn carnival_left(&mut self, member: ClientId) -> Option<bool> {
        use net::carnival::Stage;
        if !matches!(self.kind, EventKind::Carnival(_)) {
            return None;
        }
        let members = self.team.members();
        match self.carnival_stage {
            Stage::Waiting if member == self.team.leader() => {
                self.move_team(None, self.kind.event().lobby, None);
                Some(true)
            }
            Stage::Waiting | Stage::Over => None,
            Stage::Closing if self.carnival_outcome.is_some() => Some(members.is_empty()),
            Stage::Ready => Some(!self.carnival.both_sides(members)),
            Stage::Battle | Stage::Closing => {
                let over = !self.carnival.both_sides(members);
                if over {
                    self.carnival_deserted();
                } else {
                    let members = members.to_vec();
                    self.carnival_quit(member, &members);
                }
                Some(over)
            }
        }
    }

    /// A player left an Ariant battle: with fewer than two left in it, it
    /// ends and they go back to the lobby (Cosmic AriantColiseum's
    /// `leaveArenaInternal` and `dispose`). Whether that ends the event,
    /// or None if it isn't one.
    fn ariant_left(&mut self) -> Option<bool> {
        if !matches!(self.kind, EventKind::Ariant(_)) {
            return None;
        }
        let least = if self.ariant_results { 1 } else { 2 };
        let over = self.team.members().len() < least;
        if over {
            self.move_team(None, net::ariant::ARIANT_LOBBY, None);
        }
        Some(over)
    }

    /// When the time shown on the event's maps runs out: an Ariant
    /// battle's ten minutes, past its results.
    fn clock(&self) -> Option<Instant> {
        match self.kind {
            EventKind::Ariant(_) if !self.ariant_results => {
                (self.ends).map(|ends| ends + net::events::scaled(net::ariant::RESULTS_BEFORE))
            }
            _ => self.ends,
        }
    }

    /// A member of an Ariant battle holds `jewels` Spirit Jewels: the
    /// players see the scores within `SCORES_EVERY`.
    fn ariant_jewels(&mut self, client_id: ClientId, name: &str, jewels: i32, lost: i32) {
        if !matches!(self.kind, EventKind::Ariant(_)) || !self.team.has(client_id) {
            return;
        }
        if self.ariant.hold(client_id, name, jewels, lost) && self.ariant_board.is_none() {
            self.ariant_board = Some(Instant::now() + net::ariant::SCORES_EVERY);
        }
    }

    /// The players of an Ariant battle see its scores.
    fn show_ariant_scores(&mut self) {
        self.ariant_board = None;
        let Ok(board) = build_ariant_score(&self.ariant.board()) else {
            return;
        };
        self.to_members(&[board]);
    }

    /// `packets` to every member, if on the event's maps.
    fn to_members(&self, packets: &[packet::Packet]) {
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventPackets {
                    id: self.id,
                    packets: packets.to_vec(),
                });
            }
        }
    }

    /// An Ariant battle's time is up: first its results, ten seconds
    /// before its end, then the end, which takes its players to the
    /// King's Room (Cosmic `showArenaResults` and `enterKingsRoom`). At
    /// the results its players see the scoreboard, its mobs go, and each
    /// is told their Battle Arena points.
    async fn ariant_time_up(&mut self, arena: u8) -> bool {
        if self.ariant_results {
            return true;
        }
        self.ariant_results = true;
        self.ends = Some(Instant::now() + net::events::scaled(net::ariant::RESULTS_BEFORE));
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: net::ariant::battle(arena),
            instance_id: self.id,
        });
        let (reply, items) = oneshot::channel();
        let on_floor = match field.tx.send(FieldMessage::FloorItems { reply }).await {
            Ok(()) => (items.await.unwrap_or_default().iter())
                .filter(|&&(item_id, _)| item_id == net::ariant::SPIRIT_JEWEL)
                .count(),
            Err(_) => 0,
        };
        let _ = field.tx.send(FieldMessage::ClearMobs).await;
        let points = self
            .ariant
            .points(i32::try_from(on_floor).unwrap_or(i32::MAX));
        info!(id = self.id, arena, ?points, "Ariant results");
        if let Ok(result) = build_ariant_result() {
            self.to_members(&[result]);
        }
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                let earned = points.iter().find(|(who, _)| who == member);
                handle.send(ServerMessage::AriantResult {
                    id: self.id,
                    points: earned.map_or(0, |&(_, points)| points),
                });
            }
        }
        false
    }

    /// `member` quit a battle that goes on with `staying`: everyone in it
    /// hears who, or, if they led their side, who leads it now.
    fn carnival_quit(&mut self, member: ClientId, staying: &[ClientId]) {
        let Some(quit) = self.carnival.quit(member, staying) else {
            return;
        };
        let Ok(leave) = build_carnival_leave(quit.kind, quit.side, &quit.name) else {
            return;
        };
        for member in staying {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventPackets {
                    id: self.id,
                    packets: vec![leave.clone()],
                });
            }
        }
    }

    /// The other side left the battle field before the end: those still
    /// in the battle are told in the chat log, on their way out.
    fn carnival_deserted(&self) {
        let Ok(result) = build_carnival_result(net::carnival::RESULT_LEFT) else {
            return;
        };
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventPackets {
                    id: self.id,
                    packets: vec![result.clone()],
                });
            }
        }
    }

    /// `member` of a squad's roster comes back while it is not won. A
    /// ticket that doesn't arrive hands itself back.
    fn join(&mut self, member: ClientHandle) -> SquadAnswer {
        let Some(tx) = self.joiner.upgrade() else {
            return SquadAnswer::Refused;
        };
        let admitted = if self.kind == EventKind::GuildQuest {
            // Any of the guild, the directory saw to that, while it gathers
            // and has room (Cosmic GuildQuest's `maxPlayers`).
            self.gathering.is_some()
                && !self.team.has(member.client_id)
                && self.team.members().len() < *self.kind.event().team.end()
                && {
                    self.team.admit(member.client_id);
                    true
                }
        } else {
            self.team.rejoin(member.client_id)
        };
        if !admitted {
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

    /// The leader first hears what the rope or platform stage asks; then,
    /// once no monster is about, each try with five on the ropes or
    /// platforms is judged, and the right one clears the stage (Cosmic
    /// `9201044`).
    fn ropes(
        &mut self,
        client_id: ClientId,
        stage: u8,
        mobs: bool,
        heads: &[u8],
    ) -> Option<RopesTalk> {
        use net::events::{AMORIA_PLATFORMS_STAGE, AMORIA_ROPES_STAGE};
        let on = [AMORIA_ROPES_STAGE, AMORIA_PLATFORMS_STAGE];
        if !self.team.has(client_id) || self.kind != EventKind::Amoria || !on.contains(&stage) {
            return None;
        }
        let talk = if self.cleared.has(stage) {
            RopesTalk::Cleared
        } else if client_id != self.team.leader() {
            RopesTalk::NotLeader
        } else if self.heard.insert(stage) {
            RopesTalk::Intro
        } else if mobs {
            RopesTalk::Mobs
        } else if stage == AMORIA_ROPES_STAGE {
            let draw = || net::events::draw_ropes(&mut rand::thread_rng());
            self.ropes.try_heads(net::events::first_three(heads), draw)
        } else {
            self.ropes.try_platforms(heads, draw_amoria_platforms)
        };
        if talk == RopesTalk::Right {
            self.clear(stage);
        }
        Some(talk)
    }

    /// The leader first hears what the gate stage asks; then, with the
    /// whole team gathered at Amos and every gate open, clears it. Once
    /// cleared, any member's talk takes the team on to stage 6, once
    /// (Cosmic `9201045`).
    fn gather(&mut self, client_id: ClientId, together: bool, open: bool) -> Option<GatherTalk> {
        use net::events::{AMORIA_GATES_STAGE, AMORIA_STAGE6};
        let stage = AMORIA_GATES_STAGE;
        if !self.team.has(client_id) || self.kind != EventKind::Amoria {
            return None;
        }
        let talk = if self.cleared.has(stage) {
            GatherTalk::Cleared
        } else if client_id != self.team.leader() {
            GatherTalk::NotLeader
        } else if self.heard.insert(stage) {
            GatherTalk::Intro
        } else if !together {
            GatherTalk::Apart
        } else if !open {
            GatherTalk::Shut
        } else {
            GatherTalk::Right
        };
        match talk {
            GatherTalk::Right => {
                self.amoria_clock(stage);
                self.clear(stage);
            }
            GatherTalk::Cleared if !self.moved_to.contains(&AMORIA_STAGE6) => {
                self.moved_to.push(AMORIA_STAGE6);
                self.move_team(None, AMORIA_STAGE6, self.ends);
            }
            _ => {}
        }
        Some(talk)
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
                } else if self.kind == EventKind::Amoria && !self.cleared.has(stage) {
                    self.amoria_clock(stage);
                    self.clear(stage);
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
                } else if self.clear(stage) {
                    self.send_room_key(stage);
                }
            }
            // Once the shard is in, opening one of the three ways at
            // random (Cosmic `9201044`'s `clearStage`).
            EventCommand::ClearStage(1) if self.kind == EventKind::Amoria => {
                if leader && self.cleared.shard() && !self.cleared.has(1) {
                    let ways = net::events::AMORIA_WAYS.len();
                    let way = rand::Rng::gen_range(&mut rand::thread_rng(), 0, ways);
                    let way = u8::try_from(way).unwrap_or(0);
                    self.cleared.way.store(way + 1, Ordering::Release);
                    self.clear(1);
                } else {
                    debug!(client_id, "Clear not taken");
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
            // A squad's lead stays its own, whatever the parties, and so
            // does one taken alone; an event that lasts until it is empty
            // doesn't end with a lead outside.
            // A Guild Quest's registrant is no party's leader.
            EventCommand::HandOver(to)
                if !event.squad
                    && !event.solo()
                    && self.kind != EventKind::GuildQuest
                    && (leader || client_id == to) =>
            {
                self.leader.store(to, Ordering::Relaxed);
                // Whichever side's party it is.
                self.carnival.lead(to);
                if self.team.hand_over(to) {
                    info!(kind = ?self.kind, id = self.id, to, "Event's lead left it");
                    self.failed = true;
                }
            }
            EventCommand::HandOver(_) => {}
            // The leader's; or, once any member has cleared the stage
            // before it, theirs (Ellin at the maze's end).
            EventCommand::MoveTeam(map)
                if event.maps.contains(&map) && (leader || self.member_moves_to(map)) =>
            {
                self.moved_to.push(map);
                self.move_team(None, map, self.ends);
            }
            EventCommand::MoveTeam(map) => debug!(map, client_id, "Move not taken"),
            // Out of the floor before a party Dojo's rest floor, once its
            // boss is beaten, everyone on the section's floors gathers on
            // the rest floor, each time anyone goes that way (Cosmic
            // `dojang_next`), so whoever comes after is taken too.
            EventCommand::DojoRest(rest)
                if self.kind == EventKind::DojoParty
                    && net::events::rest_floor(rest)
                    && self.cleared.has(rest - 1) =>
            {
                let to = event.stage_map(rest);
                for floor in rest.saturating_sub(5)..rest {
                    self.move_team(Some(event.stage_map(floor)), to, self.ends);
                }
            }
            EventCommand::DojoRest(rest) => debug!(rest, client_id, "Rest not taken"),
            // The leader takes whoever is on the rest floor on with them
            // (Cosmic `2091005`'s `warpParty`).
            EventCommand::DojoOn(rest)
                if leader && self.kind == EventKind::DojoParty && net::events::rest_floor(rest) =>
            {
                let from = event.stage_map(rest);
                self.move_team(Some(from), event.stage_map(rest + 1), self.ends);
            }
            EventCommand::DojoOn(rest) => debug!(rest, client_id, "Dojo move not taken"),
            EventCommand::WinWithDocuments if leader && self.kind == EventKind::Zpq && !won => {
                self.cleared.set(ZPQ_DOCUMENTS);
                self.win();
            }
            EventCommand::WinWithDocuments => debug!(client_id, "Win not taken"),
            EventCommand::WinWithCoupons(coupons)
                if leader
                    && self.kind == EventKind::LudiMaze
                    && !won
                    && coupons >= MAZE_COUPONS_WANTED =>
            {
                self.win_exp = Some(coupons.saturating_mul(MAZE_COUPON_EXP));
                self.win();
            }
            EventCommand::WinWithCoupons(_) => debug!(client_id, "Win not taken"),
            EventCommand::RestartClock if self.kind == EventKind::Horntail && !won => {
                self.restart_clock(net::events::HORNTAIL_FIGHT_MINUTES);
            }
            EventCommand::RestartClock => debug!(client_id, "Restart not taken"),
            // The Keep's treasure room comes after its win.
            EventCommand::Reached(map)
                if event.holds(map) && (!won || map == net::events::CWK_TREASURE) =>
            {
                if let Some(clock) = net::events::clock_on(map) {
                    if self.reached.insert(map) {
                        match clock {
                            ClockOn::Restart(minutes) => self.restart_clock(minutes),
                            ClockOn::Stop => self.stop_clock(),
                        }
                    }
                }
            }
            EventCommand::Reached(_) => {}
            EventCommand::ShowStatues if self.statues_open(client_id) => self.show_statues(),
            EventCommand::ShowStatues => debug!(client_id, "Statues not shown"),
            EventCommand::StatueStruck(statue) if self.kind == EventKind::GuildQuest => {
                self.statues.struck(statue, std::time::Instant::now());
            }
            EventCommand::StatueStruck(_) => {}
            EventCommand::SigilLit { map_id, job } if self.kind == EventKind::Cwk && !won => {
                self.sigil_lit(map_id, job)
            }
            EventCommand::SigilLit { .. } => {}
            EventCommand::CwkStep if self.kind == EventKind::Cwk && !won => {
                self.cwk_step(client_id)
            }
            EventCommand::CwkStep => {}
            EventCommand::CwkHost(phase) if leader && self.kind == EventKind::Cwk && !won => {
                self.cwk_host(phase)
            }
            EventCommand::CwkHost(_) => {}
            EventCommand::SpearsSet(set) if self.kind == EventKind::GuildQuest => {
                self.spears_set(client_id, set)
            }
            EventCommand::SpearsSet(_) => {}
            EventCommand::KingRoomFell { here, mobs } if self.kind == EventKind::GuildQuest => {
                self.king_room_fell(client_id, here, mobs);
            }
            EventCommand::KingRoomFell { .. } => {}
            EventCommand::Revived => {
                self.fallen.remove(&client_id);
            }
            EventCommand::Fell if self.kind == EventKind::Showa && !won => {
                self.showa_fell = true;
            }
            EventCommand::Fell if self.kind == EventKind::PinkBean && !won => {
                self.pink_bean_fallen += 1;
                if let Some((key, ends)) = net::events::pink_bean_fallen(self.pink_bean_fallen) {
                    self.tell_team(key);
                    if ends {
                        info!(kind = ?self.kind, id = self.id, "Too many fell");
                        self.failed = true;
                    }
                }
            }
            EventCommand::Fell => {}
            EventCommand::KingGateOpened if self.kind == EventKind::GuildQuest => {
                if !self.cleared.king_gate.swap(true, Ordering::AcqRel) {
                    info!(id = self.id, client_id, "King's Corridor opened");
                }
            }
            EventCommand::KingGateOpened => {}
            // Kept by the member's own ticket.
            EventCommand::PrizeTaken => {}
            // Each in turn, by the leader (Cosmic `2094002`).
            EventCommand::HandInMarks(n)
                if leader
                    && self.kind == EventKind::Pirate
                    && !won
                    && usize::from(n) <= net::events::PIRATE_MARKS.len()
                    && n == self.cleared.marks() + 1 =>
            {
                self.cleared.marks.store(n, Ordering::Release);
                let next = net::events::PIRATE_MARKS.get(usize::from(n));
                self.swap_mobs(event.stage_map(2), next.map(|&(_, mob_id)| mob_id));
                if next.is_none() {
                    self.clear(2);
                }
            }
            EventCommand::HandInMarks(n) => debug!(n, client_id, "Marks not taken"),
            // One a room, so two at most.
            EventCommand::ChestOpened
                if self.kind == EventKind::Pirate
                    && !won
                    && usize::from(self.cleared.chests())
                        < net::reactors::PIRATE_CHEST_NAMES.len() =>
            {
                self.cleared.chests.fetch_add(1, Ordering::Release);
            }
            EventCommand::ChestOpened => debug!(client_id, "Chest not counted"),
            // The first member past the doors finds him, picked by the
            // chests opened (Cosmic `davy_next4`, `spawnedBoss`).
            EventCommand::PastTheDoors
                if self.kind == EventKind::Pirate && !won && !self.lord_pirate_out =>
            {
                self.lord_pirate_out = true;
                let field = self.fields.field(FieldKey {
                    channel_id: self.channel_id,
                    map_id: net::events::PIRATE_STAGES[5],
                    instance_id: self.id,
                });
                let message = FieldMessage::BringOut {
                    mob_id: net::events::lord_pirate(self.cleared.chests()),
                    at: net::events::LORD_PIRATE_AT,
                };
                tokio::spawn(async move {
                    let _ = field.tx.send(message).await;
                });
            }
            EventCommand::PastTheDoors => debug!(client_id, "Lord Pirate already out"),
            // Once a run (Cosmic `9201047`'s `statusStg1`).
            EventCommand::FierryOut
                if self.kind == EventKind::Amoria && !won && !self.cleared.fierry() =>
            {
                self.cleared.fierry.store(true, Ordering::Release);
                let field = self.fields.field(FieldKey {
                    channel_id: self.channel_id,
                    map_id: event.stage_map(1),
                    instance_id: self.id,
                });
                let message = FieldMessage::BringOut {
                    mob_id: net::events::FIERRY,
                    at: net::events::FIERRY_AT,
                };
                tokio::spawn(async move {
                    let _ = field.tx.send(message).await;
                });
            }
            EventCommand::FierryOut => debug!(client_id, "Fierry already out"),
            EventCommand::ShardIn if leader && self.kind == EventKind::Amoria => {
                self.cleared.shard.store(true, Ordering::Release);
            }
            EventCommand::ShardIn => debug!(client_id, "Shard not taken"),
            // A Magatia hub's side door opened the way back out of the
            // party's copy of the room behind it.
            EventCommand::OpenReactor { map_id, name } if event.holds(map_id) && !won => {
                let field = self.fields.field(FieldKey {
                    channel_id: self.channel_id,
                    map_id,
                    instance_id: self.id,
                });
                tokio::spawn(async move {
                    let _ = field.tx.send(FieldMessage::OpenReactor { name }).await;
                });
            }
            EventCommand::OpenReactor { map_id, .. } => {
                debug!(map_id, client_id, "Event reactor not opened")
            }
            // Once a run, everyone hears that Romeo or Juliet read the
            // letter (Cosmic `2112005`/`2112006`'s `mapMessage(6, ..)` to
            // the party's maps).
            EventCommand::Shocked if !self.cleared.shocked() && !won => {
                let key = match self.kind {
                    EventKind::Magatia(net::events::MagatiaSide::Zenumist) => {
                        keys::MAGATIA_ROMEO_SHOCKED
                    }
                    EventKind::Magatia(net::events::MagatiaSide::Alcadno) => {
                        keys::MAGATIA_JULIET_SHOCKED
                    }
                    _ => return debug!(client_id, "Shock outside Magatia"),
                };
                self.cleared.shocked.store(true, Ordering::Release);
                let text = net::content::text().get(key, &[]);
                if let Ok(line) = build_item_line(&text, 0) {
                    for member in self.team.members() {
                        if let Some(handle) = self.members.get(member) {
                            handle.send(ServerMessage::EventPackets {
                                id: self.id,
                                packets: vec![line.clone()],
                            });
                        }
                    }
                }
            }
            EventCommand::Shocked => debug!(client_id, "Already shocked"),
            // Everyone hears what Yulete said as he left (Cosmic
            // `yuleteAction`'s `dropMessage(5, ..)`).
            EventCommand::YuleteLeft(caught) if matches!(self.kind, EventKind::Magatia(_)) => {
                self.cleared.caught_yulete.store(caught, Ordering::Release);
                let key = net::events::yulete_leaves(caught);
                let text = net::content::text().get(key, &[]);
                if let Ok(line) = build_broadcast(Broadcast::Pink, &text) {
                    for member in self.team.members() {
                        if let Some(handle) = self.members.get(member) {
                            handle.send(ServerMessage::EventPackets {
                                id: self.id,
                                packets: vec![line.clone()],
                            });
                        }
                    }
                }
            }
            EventCommand::YuleteLeft(_) => debug!(client_id, "Yulete outside Magatia"),
            // Everyone hears Yulete bring out his Frankenroid (Cosmic
            // `2112000`'s `dropMessage(5, ..)`).
            EventCommand::FrankenroidOut(caught) if matches!(self.kind, EventKind::Magatia(_)) => {
                let key = net::events::frankenroid_called(caught);
                let text = net::content::text().get(key, &[]);
                if let Ok(line) = build_broadcast(Broadcast::Pink, &text) {
                    for member in self.team.members() {
                        if let Some(handle) = self.members.get(member) {
                            handle.send(ServerMessage::EventPackets {
                                id: self.id,
                                packets: vec![line.clone()],
                            });
                        }
                    }
                }
            }
            EventCommand::FrankenroidOut(_) => debug!(client_id, "Frankenroid outside Magatia"),
            // Whenever it happens, as Cosmic's `friendlyKilled`.
            EventCommand::EscortFailed if matches!(self.kind, EventKind::Magatia(_)) => {
                self.cleared.escort_failed.store(true, Ordering::Release);
            }
            EventCommand::EscortFailed => debug!(client_id, "Escort outside Magatia"),
            EventCommand::Massacre(hits) if self.kind.massacre() => {
                let seconds = self.stage_seconds();
                let mut reached = Vec::new();
                if let Some(gauge) = &self.gauge {
                    self.tally.add(hits, gauge, seconds);
                    reached = self.tally.reached(gauge);
                }
                for effect in reached.iter().filter(|effect| effect.buff > 0) {
                    self.bless(effect.buff);
                }
                let rage = reached.iter().any(|effect| effect.skill_use);
                self.tell_tally(Some(hits), rage);
            }
            EventCommand::Massacre(_) => debug!(client_id, "Massacre not counted"),
            EventCommand::Rage if self.kind.pyramid_mode().is_some() => {
                if !self.tally.spend_rage() {
                    debug!(client_id, "Rage of Pharaoh with no uses left");
                }
                // Even with none spent, the members hear how many are left.
                self.tell_tally(Some(net::pyramid::Hits::default()), true);
            }
            EventCommand::Rage => debug!(client_id, "Rage of Pharaoh outside the Pyramid"),
            EventCommand::CarnivalCp(cp) if matches!(self.kind, EventKind::Carnival(_)) => {
                self.carnival_cp(client_id, cp);
            }
            EventCommand::CarnivalArrived if matches!(self.kind, EventKind::Carnival(_)) => {
                self.carnival_start(client_id);
            }
            EventCommand::CarnivalSpend { tab, index }
                if matches!(self.kind, EventKind::Carnival(_)) =>
            {
                self.carnival_spend(client_id, tab, index);
            }
            EventCommand::CarnivalCast(id) if matches!(self.kind, EventKind::Carnival(_)) => {
                self.carnival_cast(client_id, id);
            }
            EventCommand::CarnivalDied if matches!(self.kind, EventKind::Carnival(_)) => {
                self.carnival_died(client_id);
            }
            EventCommand::CarnivalCp(_)
            | EventCommand::CarnivalArrived
            | EventCommand::CarnivalSpend { .. }
            | EventCommand::CarnivalCast(_)
            | EventCommand::CarnivalDied => {
                debug!(client_id, "Not in a Monster Carnival");
            }
        }
    }

    /// A member's kill or item in a Monster Carnival battle earned `cp`:
    /// their screen shows their own CP, and everyone's their side's (Cosmic
    /// `gainCP`). Nothing is earned outside the battle.
    fn carnival_cp(&mut self, client_id: ClientId, cp: i32) {
        if self.carnival_stage != net::carnival::Stage::Battle {
            return;
        }
        if let Some(gain) = self.carnival.gain(client_id, cp) {
            self.tell_cp(client_id, gain);
        }
    }

    /// A member died on the battle field: they and their side lose up to
    /// its `deathCP` of their CP, and everyone there hears of it (Cosmic
    /// `playerDead`).
    fn carnival_died(&mut self, client_id: ClientId) {
        let EventKind::Carnival(room) = self.kind else {
            return;
        };
        if self.carnival_stage != net::carnival::Stage::Battle {
            return;
        }
        let Some((lost, standing)) = self.carnival.lose(client_id, net::carnival::death_cp(room))
        else {
            return;
        };
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: net::carnival::battle_map(room),
            instance_id: self.id,
        });
        let died = FieldMessage::CarnivalDied {
            died: client_id,
            side: standing.side,
            lost: u8::try_from(lost).unwrap_or(u8::MAX),
        };
        tokio::spawn(async move {
            let _ = field.tx.send(died).await;
        });
        self.tell_cp(client_id, standing);
    }

    /// `client_id`'s CP changed: their screen shows their own, and
    /// everyone's their side's.
    fn tell_cp(&self, client_id: ClientId, gain: net::carnival::Gain) {
        let side: Vec<_> = build_party_cp(gain.side, gain.ours)
            .ok()
            .into_iter()
            .collect();
        for member in self.team.members() {
            let Some(handle) = self.members.get(member) else {
                continue;
            };
            let mut packets = side.clone();
            if *member == client_id {
                packets.splice(0..0, build_obtained_cp(gain.mine).ok());
            }
            handle.send(ServerMessage::EventPackets {
                id: self.id,
                packets,
            });
        }
    }

    /// A member spends CP on `index` of the Carnival window's tab `tab`:
    /// mobs or skills; guardians aren't sold yet. For a mob the CP is set
    /// aside while the battle field brings it out, and given back if it
    /// can't (Cosmic `MonsterCarnivalHandler`). Every request is answered,
    /// as the client's window waits for it.
    fn carnival_spend(&mut self, client_id: ClientId, tab: u8, index: u8) {
        use net::carnival::SpendRefusal;
        let EventKind::Carnival(room) = self.kind else {
            return;
        };
        if self.carnival_stage != net::carnival::Stage::Battle || tab > 2 {
            return self.refuse_spend(client_id, SpendRefusal::Failed);
        }
        if tab == 1 {
            return self.carnival_skill(client_id, room, index);
        }
        let Some(side) = self.carnival.side(client_id) else {
            return self.refuse_spend(client_id, SpendRefusal::NotEnoughCp);
        };
        let (reply, came) = oneshot::channel();
        let (cost, ask) = if tab == 0 {
            let Some((mob_id, cost, cap)) = net::carnival::summon(room, index) else {
                return self.refuse_spend(client_id, SpendRefusal::NotEnoughCp);
            };
            let ask = FieldMessage::CarnivalSummon {
                summoner: client_id,
                index,
                mob_id,
                side,
                cap,
                reply,
            };
            (cost, ask)
        } else {
            let Some((guardian, cap)) = net::carnival::guardian(room, index) else {
                return self.refuse_spend(client_id, SpendRefusal::NotEnoughCp);
            };
            let ask = FieldMessage::CarnivalGuard {
                summoner: client_id,
                index,
                guardian,
                side,
                cap,
                reply,
            };
            (guardian.cost, ask)
        };
        let Some(tx) = self.joiner.upgrade() else {
            return self.refuse_spend(client_id, SpendRefusal::Failed);
        };
        if self.carnival.spend(client_id, cost).is_none() {
            return self.refuse_spend(client_id, SpendRefusal::NotEnoughCp);
        }
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: net::carnival::battle_map(room),
            instance_id: self.id,
        });
        tokio::spawn(async move {
            let summoned = match field.tx.send(ask).await {
                Ok(()) => came.await.unwrap_or(Err(SpendRefusal::Failed)),
                Err(_) => Err(SpendRefusal::Failed),
            };
            let summoned = EventMessage::Summoned {
                client_id,
                tab,
                index,
                cost,
                summoned,
            };
            let _ = tx.send(summoned).await;
        });
    }

    /// The battle field brought out the mob `client_id` spent `cost` CP on,
    /// `index` of its list, and told everyone; or it didn't, say as the
    /// side has as many out as it may, and the CP goes back.
    fn carnival_summoned(
        &mut self,
        client_id: ClientId,
        tab: u8,
        index: u8,
        cost: i32,
        summoned: Result<(), net::carnival::SpendRefusal>,
    ) {
        if let Err(why) = summoned {
            self.carnival.refund(client_id, cost);
            // A kill meanwhile showed the CP without what was set aside.
            if let Some(standing) = self.carnival.standing(client_id) {
                self.tell_cp(client_id, standing);
            }
            return self.refuse_spend(client_id, why);
        }
        // START counts the mobs summoned.
        if tab == 0 {
            let index = usize::from(index);
            if self.carnival_summoned.len() <= index {
                self.carnival_summoned.resize(index + 1, 0);
            }
            self.carnival_summoned[index] = self.carnival_summoned[index].saturating_add(1);
        }
        if let Some(standing) = self.carnival.standing(client_id) {
            self.tell_cp(client_id, standing);
        }
    }

    /// A member spends CP on `index` of room `room`'s skill list: it falls
    /// on the other side, and everyone on the battle field hears who cast
    /// it (Cosmic `MonsterCarnivalHandler`).
    fn carnival_skill(&mut self, client_id: ClientId, room: u8, index: u8) {
        use net::carnival::SpendRefusal;
        let (Some(side), Some(listed)) = (
            self.carnival.side(client_id),
            net::carnival::skill(room, index),
        ) else {
            return self.refuse_spend(client_id, SpendRefusal::NotEnoughCp);
        };
        let Some(skill) = net::carnival::cast(listed, rand::random()) else {
            return self.refuse_spend(client_id, SpendRefusal::Failed);
        };
        let Some(standing) = self.carnival.spend(client_id, listed.cost) else {
            return self.refuse_spend(client_id, SpendRefusal::NotEnoughCp);
        };
        self.cast_on_rivals(side, skill, true);
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: net::carnival::battle_map(room),
            instance_id: self.id,
        });
        let announce = FieldMessage::CarnivalAnnounce {
            summoner: client_id,
            tab: 1,
            index,
        };
        // With the field gone the window still hears its spend went.
        let handle = self.members.get(&client_id).cloned();
        let id = self.id;
        tokio::spawn(async move {
            if field.tx.send(announce).await.is_err() {
                let message = build_carnival_message(net::carnival::SpendRefusal::Failed);
                if let (Some(handle), Ok(packet)) = (handle, message) {
                    let packets = vec![packet];
                    handle.send(ServerMessage::EventPackets { id, packets });
                }
            }
        });
        self.tell_cp(client_id, standing);
    }

    /// A member picked up an item that casts Monster Carnival skill `id`
    /// on the other side. It always takes hold, and nobody hears of it
    /// (Cosmic `StatEffect`'s `nuffSkill`).
    fn carnival_cast(&self, client_id: ClientId, id: i32) {
        if self.carnival_stage != net::carnival::Stage::Battle {
            return;
        }
        let skill = net::carnival::carnival_skill(id)
            .and_then(|skill| net::carnival::cast(skill, rand::random()));
        if let (Some(side), Some(skill)) = (self.carnival.side(client_id), skill) {
            self.cast_on_rivals(side, skill, false);
        }
    }

    /// Carnival skill `skill`, cast by side `side`, falls on one of the
    /// other side's members at random, or on each of them, if it takes
    /// hold (always, unless `rolled`). It works on those on the battle
    /// field: a disease, or Cancel Buff's dispel.
    fn cast_on_rivals(&self, side: u8, skill: CarnivalSkill, rolled: bool) {
        let EventKind::Carnival(room) = self.kind else {
            return;
        };
        if rolled && !net::carnival::takes_hold(skill.mob_skill, rand::random()) {
            return;
        }
        let map_id = net::carnival::battle_map(room);
        let rivals: Vec<ClientId> = self
            .team
            .members()
            .iter()
            .copied()
            .filter(|member| {
                self.carnival
                    .side(*member)
                    .is_some_and(|theirs| theirs != side)
            })
            .collect();
        let struck = if skill.all {
            rivals
        } else {
            let one = rand::random::<usize>() % rivals.len().max(1);
            rivals.get(one).copied().into_iter().collect()
        };
        let dispel = matches!(
            game::mob_skill::effect(skill.mob_skill, 0),
            Some(game::mob_skill::MobSkillEffect::Dispel)
        );
        let disease = net::mobs::skill_level(skill.mob_skill, skill.level).and_then(|level| {
            game::mob_skill::inflicted(skill.mob_skill, skill.level, level.x, level.duration)
        });
        for member in struck {
            let Some(handle) = self.members.get(&member) else {
                continue;
            };
            if dispel {
                handle.send(ServerMessage::Dispelled { map_id });
            } else if let Some(disease) = disease {
                handle.send(ServerMessage::Diseased { disease, map_id });
            }
        }
    }

    /// Tell `client_id`'s Carnival window why their spend failed.
    fn refuse_spend(&self, client_id: ClientId, why: net::carnival::SpendRefusal) {
        let (Some(handle), Ok(packet)) =
            (self.members.get(&client_id), build_carnival_message(why))
        else {
            return;
        };
        handle.send(ServerMessage::EventPackets {
            id: self.id,
            packets: vec![packet],
        });
    }

    /// A member arrived on the Monster Carnival battle field: their screen
    /// takes their side and the score, with none of the field's mobs
    /// summoned yet.
    fn carnival_start(&self, client_id: ClientId) {
        let (EventKind::Carnival(room), Some(start), Some(handle)) = (
            self.kind,
            self.carnival.start(client_id),
            self.members.get(&client_id),
        ) else {
            return;
        };
        let mut summoned = self.carnival_summoned.clone();
        summoned.resize(net::carnival::summonable(room), 0);
        if let Ok(packet) = build_carnival_start(start, &summoned) {
            handle.send(ServerMessage::EventPackets {
                id: self.id,
                packets: vec![packet],
            });
        }
    }

    /// Whether any member may move the team to `map`: its stage follows
    /// one that any member clears, which is, and the team hasn't been taken
    /// there yet (a second member's talk, left open, would take it back).
    fn member_moves_to(&self, map: i32) -> bool {
        let event = self.kind.event();
        let before = event.stage_on(map).and_then(|stage| stage.checked_sub(1));
        let cleared = before
            .is_some_and(|stage| event.member_clears.contains(&stage) && self.cleared.has(stage));
        cleared && !self.moved_to.contains(&map)
    }

    /// The leader takes the team to the bonus, on a clock of its own
    /// (Cosmic `1012113`: `startEventTimer`, `warpEventTeam`).
    fn start_bonus(&mut self) {
        let Some(bonus) = &self.kind.event().bonus else {
            return;
        };
        // A team of couples plays its own bonus first (Cosmic AmoriaPQ's
        // `marriedGroup`).
        let first = bonus
            .couples
            .filter(|_| self.couples && !self.couples_taken);
        if let Some((map, seconds)) = first {
            let ends = Instant::now() + net::events::scaled(Duration::from_secs(seconds));
            self.couples_taken = true;
            self.ends = Some(ends);
            self.move_team(None, map, Some(ends));
            info!(kind = ?self.kind, id = self.id, "Couples' bonus started");
            return;
        }
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
        self.tell_clock();
    }

    /// The Amorian Challenge's clock on clearing `stage` (Cosmic
    /// `9201045`): stage 4 cuts it short, if there is time enough to, and
    /// tells the team in pink; stage 5 gives back what was cut, and tells
    /// them only the boss is left.
    fn amoria_clock(&mut self, stage: u8) {
        use net::events::{AMORIA_CODES_STAGE, AMORIA_RUSH_FROM, AMORIA_RUSH_MINUTES};
        if stage == AMORIA_CODES_STAGE {
            let enough = net::events::scaled(Duration::from_secs(AMORIA_RUSH_FROM * 60));
            let ends = self.ends.filter(|ends| *ends >= Instant::now() + enough);
            if let Some(ends) = ends {
                self.rushed_from = Some(ends);
                self.restart_clock(AMORIA_RUSH_MINUTES);
            }
            self.tell_team(keys::AMORIA_PQ_STAGE4_NOTICE);
        } else if stage == AMORIA_CODES_STAGE + 1 {
            if let Some(ends) = self.rushed_from.take() {
                self.ends = Some(ends);
                info!(kind = ?self.kind, id = self.id, "Clock given back");
                self.tell_clock();
            }
            self.tell_team(keys::AMORIA_PQ_STAGE5_NOTICE);
        }
    }

    /// Every member hears `key` in pink.
    fn tell_team(&self, key: &str) {
        let text = net::content::text().get(key, &[]);
        let Ok(line) = build_broadcast(Broadcast::Pink, &text) else {
            return;
        };
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventPackets {
                    id: self.id,
                    packets: vec![line.clone()],
                });
            }
        }
    }

    /// The clock stops, and the time no longer runs out, as on a Dojo rest
    /// floor until the next section starts it over.
    fn stop_clock(&mut self) {
        self.ends = None;
        info!(kind = ?self.kind, id = self.id, "Clock stopped");
        self.tell_clock();
    }

    /// Every member's ticket and screen take the clock as it now is.
    fn tell_clock(&self) {
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventClock {
                    id: self.id,
                    clock: self.ends,
                });
            }
        }
    }

    /// How long after the clock shows 0 the time is up: a Dojo section
    /// gives its challenger a moment more.
    fn overtime(&self) -> Duration {
        match self.kind {
            EventKind::Dojo | EventKind::DojoParty => {
                net::events::scaled(net::events::DOJO_OVERTIME)
            }
            _ => Duration::ZERO,
        }
    }

    /// Whether `client_id` may take the statue keeper's test: the Guild
    /// Quest's registrant, the gate still shut.
    fn statues_open(&self, client_id: ClientId) -> bool {
        self.kind == EventKind::GuildQuest
            && client_id == self.team.leader()
            && !self.cleared.has(self.statue_stage())
    }

    /// The Guild Quest's stage played in the hall of statues.
    fn statue_stage(&self) -> u8 {
        (self.kind.event().stage_on(net::guild_quest::STATUE_HALL)).unwrap_or(u8::MAX)
    }

    /// What the statue keeper tells `client_id` (Cosmic `9040009`): a
    /// judged guess is told to everyone in the hall, and the last part
    /// passed opens the gate and clears the stage.
    fn statues(&mut self, client_id: ClientId) -> Option<net::guild_quest::StatueTalk> {
        use net::guild_quest::StatueTalk;
        if self.kind != EventKind::GuildQuest || !self.team.has(client_id) {
            return None;
        }
        if self.cleared.has(self.statue_stage()) {
            return Some(StatueTalk::Cleared);
        }
        if client_id != self.team.leader() {
            return Some(StatueTalk::NotLeader);
        }
        let talk = self.statues.talk(std::time::Instant::now());
        let notice = match talk {
            StatueTalk::Passed(part) => {
                let part = part.to_string();
                net::content::text().get(keys::GUILD_QUEST_STATUES_PART_PASSED, &[("part", &part)])
            }
            StatueTalk::Failed => net::content::text().get(keys::GUILD_QUEST_STATUES_FAILED, &[]),
            StatueTalk::Won => {
                self.clear(self.statue_stage());
                let hall = self.hall();
                tokio::spawn(async move {
                    let open = FieldMessage::SetReactor {
                        name: net::guild_quest::STATUE_GATE,
                        state: 1,
                    };
                    let _ = hall.tx.send(open).await;
                });
                return Some(talk);
            }
            _ => return Some(talk),
        };
        self.tell_hall(vec![notice], Duration::ZERO);
        Some(talk)
    }

    /// What the vassals' keeper tells `client_id`, who finds `offering` on
    /// the fountain's floor (Cosmic `9040006`): the right one opens the
    /// water gate, clears the stage and is taken; a wrong one brings out a
    /// knight of each kind, and the last wrong one six of each.
    fn vassals(
        &mut self,
        client_id: ClientId,
        offering: Option<[u8; net::guild_quest::VASSALS]>,
    ) -> Option<net::guild_quest::VassalTalk> {
        use net::guild_quest::{
            VassalTalk, FOUNTAIN, KNIGHTS, KNIGHTS_ON_FAILING, KNIGHTS_X, OFFERINGS, WATER_GATE,
        };
        use rand::Rng;
        if self.kind != EventKind::GuildQuest || !self.team.has(client_id) {
            return None;
        }
        let stage = self.kind.event().stage_on(FOUNTAIN)?;
        if self.cleared.has(stage) {
            return Some(VassalTalk::Cleared);
        }
        if client_id != self.team.leader() {
            return Some(VassalTalk::NotLeader);
        }
        let mut rng = rand::thread_rng();
        let talk = self.vassals.talk(offering, &mut rng);
        info!(id = self.id, ?talk, "Vassals judged");
        let mut sent = Vec::new();
        match talk {
            VassalTalk::Right => {
                self.clear(stage);
                sent.push(FieldMessage::SetReactor {
                    name: WATER_GATE,
                    state: 1,
                });
                sent.push(FieldMessage::TakeItems { ids: OFFERINGS });
            }
            VassalTalk::Wrong(_) => {
                for (mob_id, at) in KNIGHTS {
                    sent.push(FieldMessage::BringOut { mob_id, at });
                }
            }
            VassalTalk::Failed => {
                for (mob_id, (_, y)) in KNIGHTS {
                    for _ in 0..KNIGHTS_ON_FAILING {
                        let at = (rng.gen_range(KNIGHTS_X.start, KNIGHTS_X.end), y);
                        sent.push(FieldMessage::BringOut { mob_id, at });
                    }
                }
            }
            _ => {}
        }
        let fountain = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: FOUNTAIN,
            instance_id: self.id,
        });
        tokio::spawn(async move {
            for message in sent {
                let _ = fountain.tx.send(message).await;
            }
        });
        Some(talk)
    }

    /// What Sharen III's soul tells `client_id` (Cosmic `9040003`): the
    /// registrant's talk clears the stage and opens the grave's gate.
    fn soul(&mut self, client_id: ClientId) -> Option<net::guild_quest::SoulTalk> {
        use net::guild_quest::{SoulTalk, GHOST_GATE, GRAVE};
        if self.kind != EventKind::GuildQuest || !self.team.has(client_id) {
            return None;
        }
        let stage = self.kind.event().stage_on(GRAVE)?;
        if self.cleared.has(stage) {
            return Some(SoulTalk::Cleared);
        }
        if client_id != self.team.leader() {
            return Some(SoulTalk::NotLeader);
        }
        self.clear(stage);
        let grave = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: GRAVE,
            instance_id: self.id,
        });
        tokio::spawn(async move {
            let open = FieldMessage::SetReactor {
                name: GHOST_GATE,
                state: 1,
            };
            let _ = grave.tx.send(open).await;
        });
        Some(SoulTalk::Opened)
    }

    /// What the Rubian's keeper tells `client_id` (Cosmic `9040010`): the
    /// registrant's Rubian wins the quest, earning the guild its points the
    /// first time by how long the quest has gone on.
    fn rubian(
        &mut self,
        client_id: ClientId,
        has_rubian: bool,
    ) -> Option<net::guild_quest::RubianTalk> {
        use net::guild_quest::RubianTalk;
        if self.kind != EventKind::GuildQuest || !self.team.has(client_id) {
            return None;
        }
        if client_id != self.team.leader() {
            return Some(RubianTalk::NotLeader);
        }
        if !has_rubian {
            return Some(RubianTalk::Missing);
        }
        let first = !self.cleared.has(self.kind.event().last_stage());
        let points = first.then(|| net::guild_quest::rubian_points(self.began.elapsed()));
        self.win();
        Some(RubianTalk::HandedIn(points))
    }

    /// `client_id` died in the Guild Quest's last room, where `here` stand
    /// with `mobs` alive: with everyone there dead and a mob left, the
    /// quest ends (Cosmic GuildQuest's `playerDead`).
    fn king_room_fell(&mut self, client_id: ClientId, here: usize, mobs: usize) {
        if !self.team.has(client_id) {
            return;
        }
        self.fallen.insert(client_id);
        let won = self.cleared.has(self.kind.event().last_stage());
        let fallen = (self.fallen.iter())
            .filter(|member| self.team.has(**member))
            .count();
        if !won && mobs > 0 && here <= fallen {
            info!(kind = ?self.kind, id = self.id, "Everyone in Ergoth's room fell");
            self.failed = true;
        }
    }

    /// `set` spears are set in the room of the spears, the last by
    /// `client_id` or with them there: the knights' hall's gate moves on
    /// to match (never back, as two members' word may cross), and the last
    /// opens it, clears the stage and gains the guild its points (Cosmic
    /// `9208007`).
    fn spears_set(&mut self, client_id: ClientId, set: u8) {
        use net::guild_quest::{KNIGHT_HALL, SPEARS, SPEAR_GATE, SPEAR_POINTS};
        let hall = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: KNIGHT_HALL,
            instance_id: self.id,
        });
        tokio::spawn(async move {
            let moved = FieldMessage::ForwardReactor {
                name: SPEAR_GATE,
                state: set.min(SPEARS),
            };
            let _ = hall.tx.send(moved).await;
        });
        let stage = self.kind.event().stage_on(KNIGHT_HALL);
        if set >= SPEARS && stage.is_some_and(|stage| self.clear(stage)) {
            if let Some(handle) = self.members.get(&client_id) {
                handle.send(ServerMessage::GainGuildPoints(SPEAR_POINTS));
            }
        }
    }

    /// The event's copy of the hall of statues.
    fn hall(&self) -> FieldHandle {
        self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: net::guild_quest::STATUE_HALL,
            instance_id: self.id,
        })
    }

    /// Pink `notices` to everyone in the hall of statues, the first after
    /// `after`.
    fn tell_hall(&self, notices: Vec<String>, after: Duration) {
        let hall = self.hall();
        tokio::spawn(async move {
            sleep(after).await;
            for notice in notices {
                if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
                    let _ = hall.tx.send(FieldMessage::Show(packet)).await;
                }
            }
        });
    }

    /// The part's statues move in turn for everyone in the hall (Cosmic
    /// `delayedHitReactor`), and the hall hears when to strike them.
    fn show_statues(&mut self) {
        use net::guild_quest::{shown_after, STATUE_NAMES};
        // The statues move on the clock their judging goes by.
        let start = Instant::now();
        let Some(combo) = (self.statues).show(start.into_std(), &mut rand::thread_rng()) else {
            return;
        };
        info!(id = self.id, ?combo, "Statues shown");
        let text = net::content::text();
        let wait = text.get(keys::GUILD_QUEST_STATUES_SHOWING, &[]);
        self.tell_hall(vec![wait], Duration::ZERO);
        let names: Vec<_> = (combo.iter())
            .filter_map(|&statue| STATUE_NAMES.get(usize::from(statue) - 1).copied())
            .collect();
        let shown = text.get(keys::GUILD_QUEST_STATUES_SHOWN, &[]);
        let hall = self.hall();
        tokio::spawn(async move {
            for (n, name) in names.into_iter().enumerate() {
                sleep_until(start + shown_after(n)).await;
                let _ = hall.tx.send(FieldMessage::PlayReactor { name }).await;
            }
            if let Ok(packet) = build_broadcast(Broadcast::Pink, &shown) {
                let _ = hall.tx.send(FieldMessage::Show(packet)).await;
            }
        });
    }

    /// The guild's time to gather is up (Cosmic GuildQuest's
    /// `scheduledTimeout` while `canJoin`): no one else comes in, and with
    /// too few in or its registrant missing, the quest ends a few seconds
    /// after the client's notice; otherwise the way into Sharenian opens and
    /// the quest's own clock starts.
    fn guild_quest_gathered(&mut self) -> bool {
        self.gathering = None;
        let members = self.team.members().len();
        if members < self.kind.event().least() {
            self.close_guild_quest(build_quest_too_few());
        } else if !self.team.has(self.team.leader()) {
            self.close_guild_quest(build_quest_registrant_left());
        } else {
            self.cleared.set(1);
            info!(kind = ?self.kind, id = self.id, members, "Guild Quest under way");
            self.restart_clock(u64::from(self.kind.event().minutes));
        }
        false
    }

    /// `member` left a Guild Quest, which the team's `leave` found `ended`
    /// it or not. While the guild gathers, anyone may come and go; once
    /// under way, its registrant's leaving or too few staying ends it a few
    /// seconds after the client's notice (Cosmic `isEventTeamLackingNow`).
    fn guild_quest_left(&mut self, member: ClientId, ended: bool) -> Option<bool> {
        if self.kind != EventKind::GuildQuest || self.gathering.is_some() {
            return (self.kind == EventKind::GuildQuest).then_some(false);
        }
        if ended || self.closing || self.cleared.has(self.kind.event().last_stage()) {
            return Some(ended);
        }
        if member == self.team.leader() {
            self.close_guild_quest(build_quest_registrant_left());
        } else if self.team.members().len() < self.kind.event().least() {
            self.close_guild_quest(build_quest_too_few());
        }
        Some(false)
    }

    /// The Guild Quest can't go on: those in it hear why, and it ends in a
    /// few seconds.
    fn close_guild_quest(&mut self, notice: Result<packet::Packet, net::error::NetworkError>) {
        self.closing = true;
        self.ends = Some(Instant::now() + net::guild_quest::CLOSING);
        info!(kind = ?self.kind, id = self.id, "Guild Quest closing");
        match notice {
            Ok(notice) => {
                for member in self.team.members() {
                    if let Some(handle) = self.members.get(member) {
                        handle.send_packet(notice.clone());
                    }
                }
            }
            Err(error) => tracing::warn!(id = self.id, %error, "Failed to build packet"),
        }
    }

    /// The time ran out. A bonus's ends, taking whoever is still on its map
    /// to its exit (Cosmic `scheduledTimeout`); a team that protects a
    /// friendly mob, which would have lost already without it, wins, and
    /// the other mobs go (Cosmic DelliBattle's `scheduledTimeout`); a
    /// Nett's Pyramid run goes on to its next stage; otherwise the event
    /// ends.
    fn time_up(&mut self) -> bool {
        if let EventKind::Carnival(room) = self.kind {
            return self.carnival_time_up(room);
        }
        if self.kind == EventKind::GuildQuest && self.gathering.is_some() {
            return self.guild_quest_gathered();
        }
        let event = self.kind.event();
        match &event.bonus {
            Some(bonus) if self.bonus_taken || self.couples_taken => {
                self.ends = None;
                // After the couples' bonus, some may still be on it.
                let from = Some(bonus.map).filter(|_| !self.couples_taken);
                self.move_team(from, bonus.exit, None);
                info!(kind = ?self.kind, id = self.id, "Bonus over");
                false
            }
            // The Watch Hog's room is won only at Kenta's double; its time
            // running out loses it (Cosmic `3rdJob_mount`'s
            // `scheduledTimeout`).
            _ if event.survive.is_some()
                && self.kind != EventKind::MonsterRiding
                && !self.cleared.has(event.last_stage()) =>
            {
                self.clear_mobs(event.stage_map(event.last_stage()));
                self.win();
                false
            }
            _ if self.kind.massacre() && self.pyramid_stage < event.last_stage() => {
                self.next_pyramid_stage();
                false
            }
            _ if self.kind.massacre() => {
                self.pyramid_end = Some(true);
                true
            }
            _ => true,
        }
    }

    /// A Nett's Pyramid stage's time ran out: the team goes on to the next
    /// stage, whose clock gives 3 minutes (Cosmic `Pyramid.timer`), as the
    /// Dusty Platform's do. The last stage's ends the run, which sends
    /// everyone to its result map.
    fn next_pyramid_stage(&mut self) {
        self.pyramid_stage += 1;
        let ends = Instant::now() + net::events::scaled(net::events::PYRAMID_STAGE_TIME);
        self.ends = Some(ends);
        let to = self.kind.event().stage_map(self.pyramid_stage);
        self.move_team(None, to, Some(ends));
        self.start_pyramid_stage();
        info!(kind = ?self.kind, id = self.id, stage = self.pyramid_stage, "Pyramid stage");
    }

    /// A Nett's Pyramid stage starts with its gauge full, and the team's
    /// screens show the run's tally so far (Cosmic `Pyramid.timer`). After
    /// the move, so that the gauge reaches the stage's map.
    fn start_pyramid_stage(&mut self) {
        let map = self.kind.event().stage_map(self.pyramid_stage);
        self.gauge = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map)?.massacre.clone());
        self.stage_started = Instant::now();
        self.tally.new_stage();
        self.tell_tally(None, true);
    }

    /// The whole seconds the Nett's Pyramid stage has run. They aren't
    /// scaled as its clock is, since the client's gauge goes down with the
    /// seconds the clock shows.
    fn stage_seconds(&self) -> i32 {
        i32::try_from(self.stage_started.elapsed().as_secs()).unwrap_or(i32::MAX)
    }

    /// Every member on the run's maps takes item `item_id`'s effect.
    fn bless(&self, item_id: i32) {
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                let id = self.id;
                handle.send(ServerMessage::PyramidBlessing { id, item_id });
            }
        }
    }

    /// Every member's screen takes the tallies `hits` changed, with the
    /// uses of Rage of Pharaoh if `rage`, or all of them, and the gauge
    /// (Cosmic `broadcastInfo`); and the gauge's run out is timed again.
    fn tell_tally(&mut self, hits: Option<net::pyramid::Hits>, rage: bool) {
        let left = self.tally.skill;
        let mut packets: Vec<_> = self
            .tally
            .values(hits, rage)
            .into_iter()
            .filter_map(|(key, value)| build_session_value(key, &value.to_string()).ok())
            .collect();
        packets.extend(build_pyramid_gauge(self.tally.count).ok());
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventPackets {
                    id: self.id,
                    packets: packets.clone(),
                });
                if rage || hits.is_none() {
                    let id = self.id;
                    handle.send(ServerMessage::PyramidRage { id, left });
                }
            }
        }
        let started = self.stage_started;
        self.gauge_out = self
            .gauge
            .as_ref()
            .and_then(|gauge| self.tally.runs_out(gauge))
            .map(|seconds| started + Duration::from_secs(seconds.max(0) as u64));
    }

    /// The Nett's Pyramid stage's gauge ran out, which ends the run
    /// (Cosmic `startGaugeSchedule`), sending everyone to its result map.
    fn gauge_ran_out(&mut self) -> bool {
        self.gauge_out = None;
        let seconds = self.stage_seconds();
        let out = self
            .gauge
            .as_ref()
            .is_some_and(|gauge| self.tally.lost(gauge, seconds) >= gauge.total);
        if out {
            info!(kind = ?self.kind, id = self.id, stage = self.pyramid_stage, tally = ?self.tally, "Pyramid gauge ran out");
            self.pyramid_end = Some(false);
        }
        out
    }

    /// The score of a Nett's Pyramid run that ended by its clock or its
    /// gauge, for its members on its result map (Cosmic `sendScore`,
    /// computed once for the run).
    /// Its result map, rank and EXP.
    fn pyramid_score(&self) -> Option<(i32, u8, i32, bool)> {
        let finished = self.pyramid_end?;
        let ranks = net::pyramid::ranks(self.kind)?;
        let (rank, exp) = net::pyramid::score(&self.tally, &ranks, finished);
        info!(kind = ?self.kind, id = self.id, rank, exp, "Pyramid score");
        Some((self.kind.event().exit, rank, exp, finished))
    }

    /// The mobs that aren't friendly on the copy of `map_id` go, and no
    /// more come.
    fn clear_mobs(&self, map_id: i32) {
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id,
            instance_id: self.id,
        });
        tokio::spawn(async move {
            let _ = field.tx.send(FieldMessage::ClearMobs).await;
        });
    }

    /// The mobs on the copy of `map_id` go, giving nothing, and `only`
    /// these come in their place, or none.
    fn swap_mobs(&self, map_id: i32, only: Option<i32>) {
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id,
            instance_id: self.id,
        });
        tokio::spawn(async move {
            let _ = field.tx.send(FieldMessage::SwapMobs { only }).await;
        });
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
        // The Pirate Ship's pays by the chests the team opened.
        if self.kind == EventKind::Pirate {
            self.win_exp = Some(net::events::pirate_win_exp(self.cleared.chests()));
        }
        // Romeo and Juliet's pays by the escort and the Frankenroid.
        if let EventKind::Magatia(_) = self.kind {
            self.win_exp = Some(net::events::magatia_win_exp(
                self.cleared.escort_failed(),
                self.cleared.caught_yulete(),
            ));
        }
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
        // Ludibrium's takes the team on to its bonus at once, and MV's
        // Lair's ten seconds on (Cosmic `clearPQ`, TreasurePQ's
        // `warpBonus`).
        match event.bonus.as_ref().filter(|bonus| bonus.on_win) {
            Some(bonus) if bonus.after > 0 => {
                let after = net::events::scaled(Duration::from_secs(bonus.after));
                self.bonus_at = Some(Instant::now() + after);
            }
            Some(_) => self.start_bonus(),
            None => {}
        }
        if self.kind == EventKind::Showa && !self.showa_fell {
            self.slot_machine();
        }
    }

    /// Showa's battle was won with no member dying: the Slot Machine comes
    /// out on the copy's clear map, and Konpei tells the team (Cosmic
    /// `ShowaBattle.clearPQ`).
    fn slot_machine(&self) {
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: net::events::SHOWA_CLEAR,
            instance_id: self.id,
        });
        let message = FieldMessage::BringOut {
            mob_id: net::events::SLOT_MACHINE,
            at: net::events::SLOT_MACHINE_AT,
        };
        tokio::spawn(async move {
            let _ = field.tx.send(message).await;
        });
        self.tell_team(keys::SHOWA_NO_CASUALTIES);
    }

    /// A Trial Grounds room's seal took its key: the same key falls on the
    /// copy's first map, free for anyone, and everyone there is told
    /// (Cosmic `2408002`, at the reactor `keyDrop1`).
    fn send_room_key(&self, stage: u8) {
        let Some(item_id) = htpq_room_key(stage).filter(|_| self.kind == EventKind::Htpq) else {
            return;
        };
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: HTPQ_ENTRY,
            instance_id: self.id,
        });
        let (x, y) = HTPQ_KEY_AT;
        let key = FieldMessage::SpawnLoot {
            owner: self.team.leader(),
            ownership: Ownership::FreeForAll,
            loot: vec![Loot::Item {
                item_id,
                quantity: 1,
                quest: None,
            }],
            x,
            y,
            dropper: 0,
        };
        let notice = net::content::text().get(keys::HTPQ_KEY_APPEARED, &[]);
        let notice = build_broadcast(Broadcast::Pink, &notice).ok();
        tokio::spawn(async move {
            let _ = field.tx.send(key).await;
            if let Some(notice) = notice {
                let _ = field.tx.send(FieldMessage::Show(notice)).await;
            }
        });
    }

    /// A member lit the Keep's sigil of `job` on `map_id`, or filled the
    /// fifth stage's altar of `job`: everyone hears of it, and when all
    /// five of the stage's are, that its way on is open, and the stage
    /// clears (Cosmic `6109000`'s, `6109016`'s and `6109005`'s `act`). The
    /// third stage's also wait on its way up.
    fn sigil_lit(&self, map_id: i32, job: u8) {
        let Some(stage) = self.kind.event().stage_on(map_id) else {
            return;
        };
        let Some(before) = self.cleared.light_sigil(stage, job) else {
            return debug!(stage, job, "Sigil already lit");
        };
        // The third stage's sigils also set its menhirs going.
        let line = match stage {
            3 => net::events::CWK_MENHIR_LINES[usize::from(job)],
            5 => keys::CWK_WEAPON_RESTORED,
            _ => net::events::CWK_SIGIL_LINES[usize::from(job)],
        };
        let mut lines = vec![line];
        // The third stage's also waits on its way up.
        let done = before | 1 << job == 0b1_1111
            && match stage {
                2 | 4 | 5 => true,
                3 => self.cleared.steps() == CWK_STEPS,
                _ => false,
            };
        if done && stage == 5 {
            lines.push(keys::CWK_WEAPONS_DONE);
        } else if done {
            lines.push(keys::CWK_SIGILS_DONE);
        }
        let text = net::content::text();
        let lines: Vec<_> = lines.into_iter().map(|key| text.get(key, &[])).collect();
        self.tell_items(None, &lines);
        if done {
            self.clear(stage);
        }
    }

    /// A member took the Keep's third stage's way up (Cosmic
    /// `glpqPortal2`). Until `CWK_STEPS` have, each counts, and everyone
    /// hears how many are to go; the last, with all the stage's sigils lit,
    /// clears it. After that, everyone hears the sigils aren't all lit, or
    /// once the stage is clear, the member hears its way on is open.
    fn cwk_step(&self, client_id: ClientId) {
        let text = net::content::text();
        match self.cleared.climb() {
            Climb::Done => {
                self.tell_items(None, &[text.get(keys::CWK_SIGILS_DONE, &[])]);
                self.clear(3);
            }
            Climb::ToGo(n) => {
                let to_go = n.to_string();
                let line = text.get(keys::CWK_STEPS_TO_GO, &[("n", &to_go)]);
                self.tell_items(None, &[line]);
            }
            Climb::Open => self.tell_items(Some(client_id), &[text.get(keys::CWK_STEPS_OPEN, &[])]),
            Climb::NotAll => self.tell_items(None, &[text.get(keys::CWK_STEPS_NOT_ALL, &[])]),
        }
    }

    /// The leader heard the Keep's sixth stage's host out after it had
    /// brought mobs out `phase` times (Cosmic `9201115`'s `glpq6`), once
    /// for each: everyone there hears what it brings out, the escort at
    /// random along the floor, then the Twisted Masters; after them, the
    /// way on opens and the Keep is won.
    fn cwk_host(&mut self, phase: u8) {
        use net::events::{CWK_ESCORT, CWK_ESCORT_X, CWK_ESCORT_Y, CWK_MASTERS};
        use rand::Rng;
        if !self.cleared.host_heard(phase) {
            debug!(id = self.id, phase, "Host already heard");
            return;
        }
        let text = net::content::text();
        let mut rng = rand::thread_rng();
        let mut sent = Vec::new();
        match phase {
            0 => {
                let line = text.get(keys::CWK_ESCORT_OUT, &[]);
                sent.extend(build_item_line(&line, 0).ok().map(FieldMessage::Show));
                for (mob_id, count) in CWK_ESCORT {
                    for _ in 0..count {
                        let x = rng.gen_range(CWK_ESCORT_X.start, CWK_ESCORT_X.end);
                        let at = (x, CWK_ESCORT_Y);
                        sent.push(FieldMessage::BringOut { mob_id, at });
                    }
                }
            }
            1 => {
                let line = text.get(keys::CWK_MASTERS_OUT, &[]);
                sent.extend(build_item_line(&line, 0).ok().map(FieldMessage::Show));
                for (mob_id, at) in CWK_MASTERS {
                    sent.push(FieldMessage::BringOut { mob_id, at });
                }
            }
            _ => {
                // The way on opens with the win, before anyone hears.
                self.win();
                let line = text.get(keys::CWK_BOSS_OPEN, &[]);
                let packet = build_broadcast(Broadcast::Pink, &line);
                sent.extend(packet.ok().map(FieldMessage::Show));
            }
        }
        let field = self.fields.field(FieldKey {
            channel_id: self.channel_id,
            map_id: net::events::CWK_BOSS,
            instance_id: self.id,
        });
        tokio::spawn(async move {
            for message in sent {
                let _ = field.tx.send(message).await;
            }
        });
    }

    /// A minute after the Keep's copy opens, Master Guardians come onto its
    /// first stage, if anyone is still there (Cosmic CWKPQ's
    /// `spawnGuardians`), and those there hear in pink they were found.
    fn cwk_guardians(&self) {
        let (mob_id, at, count) = net::events::CWK_GUARDIANS;
        let key = FieldKey {
            channel_id: self.channel_id,
            map_id: net::events::CWK_ENTRY,
            instance_id: self.id,
        };
        let line = net::content::text().get(keys::CWK_GUARDIANS_DETECTED, &[]);
        let notice = build_broadcast(Broadcast::Pink, &line).ok();
        let fields = self.fields.clone();
        tokio::spawn(async move {
            sleep(net::events::scaled(net::events::CWK_GUARDIANS_AFTER)).await;
            // Only a copy still open: the event may be over.
            let Some(field) = fields.running(|running| *running == key).pop() else {
                return;
            };
            let (reply, here) = oneshot::channel();
            let _ = field.tx.send(FieldMessage::Positions { reply }).await;
            if here.await.map_or(true, |here| here.is_empty()) {
                return;
            }
            let notice = notice.map(FieldMessage::Show);
            let mobs = (0..count).map(|_| FieldMessage::BringOut { mob_id, at });
            for message in notice.into_iter().chain(mobs) {
                let _ = field.tx.send(message).await;
            }
        });
    }

    /// `only`, or every member, sees `lines` in the chat log, as of an
    /// item.
    fn tell_items(&self, only: Option<ClientId>, lines: &[String]) {
        let packets: Vec<_> = (lines.iter())
            .filter_map(|line| build_item_line(line, 0).ok())
            .collect();
        for member in self.team.members() {
            if only.is_some_and(|only| only != *member) {
                continue;
            }
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventPackets {
                    id: self.id,
                    packets: packets.clone(),
                });
            }
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
                handle.send(ServerMessage::StageCleared {
                    id: self.id,
                    stage,
                    exp: self
                        .win_exp
                        .filter(|_| stage == self.kind.event().last_stage()),
                });
            }
        }
        true
    }
}

/// The platforms Amos wants on the Amorian Challenge's stage 3, five of
/// its map's.
fn draw_amoria_platforms() -> Option<u32> {
    let field = net::get_game_data()
        .ok()?
        .field(net::events::AMORIA_STAGE3)?;
    draw_combo(field.areas.len(), net::events::AMORIA_ROPERS.into())
}

/// A combo for puzzle stage `stage` of `event`, from its map's areas.
fn draw_stage_combo(event: &Event, stage: u8) -> Option<u32> {
    let field = net::get_game_data().ok()?.field(event.stage_map(stage))?;
    draw_combo(field.areas.len(), event.areas_taken)
}

/// When a Monster Carnival battle whose clock runs out at `ends` is
/// decided: a little before, but not before now.
fn closing_at(ends: Instant) -> Instant {
    ends.checked_sub(net::carnival::result_lead())
        .unwrap_or(ends)
        .max(Instant::now())
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
    fn the_keep_s_sigils_count_once_each_by_stage() {
        let cleared = Cleared::default();
        assert_eq!(cleared.light_sigil(2, 0), Some(0));
        assert_eq!(cleared.light_sigil(2, 0), None);
        assert_eq!(cleared.light_sigil(2, 3), Some(0b1));
        // Another stage's are its own.
        assert_eq!(cleared.light_sigil(3, 3), Some(0));
        assert_eq!(cleared.light_sigil(4, 4), Some(0));
        for job in [1, 2] {
            cleared.light_sigil(2, job);
        }
        assert_eq!(cleared.light_sigil(2, 4), Some(0b0_1111));
        assert_eq!(cleared.light_sigil(3, 4), Some(0b0_1000));
        // No sigils on the first stage, nor past the fifth job's, nor past
        // the fifth stage's altars.
        assert_eq!(cleared.light_sigil(1, 0), None);
        assert_eq!(cleared.light_sigil(2, 5), None);
        assert_eq!(cleared.light_sigil(6, 0), None);
        // All of a stage's lit.
        assert!(!cleared.sigils_lit(3));
        for job in 0..4 {
            cleared.light_sigil(3, job);
        }
        assert!(cleared.sigils_lit(2) && cleared.sigils_lit(3) && !cleared.sigils_lit(4));
        assert!(!cleared.sigils_lit(1) && !cleared.sigils_lit(5));
        // The fifth stage's altars, its own too.
        for job in 0..4 {
            assert_eq!(cleared.light_sigil(5, job), Some((1 << job) - 1));
        }
        assert_eq!(cleared.light_sigil(5, 4), Some(0b0_1111));
        assert!(cleared.sigils_lit(5) && !cleared.sigils_lit(6));
    }

    #[test]
    fn the_keep_s_way_up_counts_five() {
        let cleared = Cleared::default();
        let steps: Vec<_> = (0..7).map(|_| cleared.step()).collect();
        assert_eq!(
            steps,
            [Some(1), Some(2), Some(3), Some(4), Some(5), None, None]
        );
        assert_eq!(cleared.steps(), CWK_STEPS);
    }

    #[test]
    fn the_keep_s_way_up_clears_with_the_last_of_climbs_and_sigils() {
        // The sigils lit first: the fifth trip up clears the stage.
        let cleared = Cleared::default();
        for job in 0..5 {
            cleared.light_sigil(3, job);
        }
        let climbs: Vec<_> = (0..6).map(|_| cleared.climb()).collect();
        use Climb::*;
        assert_eq!(climbs, [ToGo(4), ToGo(3), ToGo(2), ToGo(1), Done, Open]);
        // The climbs first: then not all are lit, until they are.
        let cleared = Cleared::default();
        let climbs: Vec<_> = (0..6).map(|_| cleared.climb()).collect();
        assert_eq!(
            climbs,
            [ToGo(4), ToGo(3), ToGo(2), ToGo(1), ToGo(0), NotAll]
        );
        for job in 0..5 {
            cleared.light_sigil(3, job);
        }
        assert_eq!(cleared.climb(), Open);
    }

    #[test]
    fn the_keep_s_host_moves_on_once_from_each_phase() {
        let cleared = Cleared::default();
        // Two talks as it brings out the escort: only the first counts.
        assert!(cleared.host_heard(0) && !cleared.host_heard(0));
        assert_eq!(cleared.host(), 1);
        // A talk of a phase gone by, or ahead, doesn't count.
        assert!(!cleared.host_heard(2) && cleared.host_heard(1));
        assert!(cleared.host_heard(2) && !cleared.host_heard(1));
        assert_eq!(cleared.host(), 3);
    }

    #[test]
    fn cleared_keeps_one_bit_per_stage() {
        let cleared = Cleared::default();
        assert!(cleared.set(1) && cleared.set(3) && !cleared.set(1));
        assert!(cleared.has(1) && !cleared.has(2) && cleared.has(3));
        // Stages are counted from 1, and sixty-four fit, the Dojo's 38
        // among them.
        assert!(!cleared.set(0) && !cleared.set(65));
        assert!(!cleared.has(0) && !cleared.has(65) && !cleared.has(9));
        assert!(cleared.set(9) && cleared.set(38) && cleared.set(64));
        assert!(cleared.has(9) && cleared.has(38) && cleared.has(64));
        assert_eq!((cleared.marks(), cleared.chests()), (0, 0));
    }
}
