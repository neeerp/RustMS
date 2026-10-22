//! The player's side of a party event (`crate::actor::event`): the ticket
//! that puts them in the event's own copy of its maps, the event's clock,
//! its stages' gates, and getting out when it ends or they leave.

use super::quest::gain_exp;
use super::ClientActor;
use crate::actor::event::EventTicket;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldKey, FieldMessage};
use game_data::{Area, PortalTemplate};
use net::events::{
    self, on_areas, standing, Balloon, Cloto, EventCommand, EventKind, HeadCount, LeverCheck,
    Standing,
};
use net::handler::FieldRequest;
use net::npc_scripts::{NpcScript, AQUA_BALLOON, BLUE_BALLOON};
use net::packet::build::world::conti::{build_countdown, build_stop_clock};
use net::packet::build::world::effect::{
    build_field_effect, build_field_object, build_field_sound,
};
use net::packet::build::world::inventory::build_inventory_operation;
use net::packet::handle::world::{portal_name, warp_to_spawn_point};
use net::scripts::{self, Effect};
use net::ClientId;
use packet::Packet;
use tokio::sync::oneshot;
use tokio::time::Instant;

impl ClientActor {
    /// The field `map_id` is played in: the event's copy of it for a member
    /// holding a ticket to its maps, otherwise the channel's own.
    pub(super) fn field_key(&self, map_id: i32) -> FieldKey {
        let event = self.event.as_ref().filter(|ticket| ticket.holds(map_id));
        let wedding = self.wedding.as_ref().filter(|ticket| ticket.holds(map_id));
        match event
            .map(|ticket| ticket.id)
            .or(wedding.map(|ticket| ticket.id))
        {
            Some(instance_id) => FieldKey {
                channel_id: self.channel_id,
                map_id,
                instance_id,
            },
            None => FieldKey::new(self.channel_id, map_id),
        }
    }

    /// The NPC who starts an event tells the player whether their party may
    /// go in.
    pub(super) async fn talk_about_event(&mut self, kind: EventKind) -> Result<(), RuntimeError> {
        let (reply, entry) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::EventEntry {
            from: self.client_id,
            kind,
            reply,
        })
        .await?;
        let Ok(entry) = entry.await else {
            return Ok(());
        };
        let script = match kind {
            EventKind::Kpq => NpcScript::lakelis(entry),
            EventKind::Hpq => NpcScript::tory(entry),
            EventKind::Lpq => NpcScript::red_sign(entry),
            EventKind::Opq => NpcScript::wonky(entry),
        };
        self.start_script(script).await
    }

    /// Cloto sets the party quest's stage for a player in it there, and
    /// hears the leader's answer to a puzzle. Returns whether she did;
    /// elsewhere she has nothing to say.
    pub(super) async fn talk_to_cloto(&mut self) -> Result<bool, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let in_kpq = |ticket: &&EventTicket| ticket.kind == EventKind::Kpq && ticket.holds(map_id);
        let Some(ticket) = self.event.as_ref().filter(in_kpq) else {
            return Ok(false);
        };
        let Some(stage) = ticket.kind.event().stage_on(map_id) else {
            return Ok(false);
        };
        let script = match ticket.cloto(stage).await {
            None => return Ok(false),
            Some(Cloto::Puzzle { stage, combo }) => {
                // Unchecked, she has nothing to say rather than a wrong.
                let judge =
                    |areas: &[Area], positions: &[(i32, i32)]| on_areas(combo, areas, positions);
                let Some(right) = self.standing_on(map_id, judge).await? else {
                    return Ok(false);
                };
                if !right {
                    self.show_wrong_answer().await?;
                }
                NpcScript::cloto_answer(stage, right)
            }
            Some(view) => NpcScript::cloto(view, &self.in_game()?.player),
        };
        self.start_script(script).await?;
        Ok(true)
    }

    /// A balloon on one of the event's stages: the stage's hand-in, the
    /// crates' puzzle, or the maze's one line. Returns whether it spoke;
    /// elsewhere it has nothing to say.
    pub(super) async fn talk_to_balloon(&mut self, npc_id: i32) -> Result<bool, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(false);
        };
        let event = ticket.kind.event();
        let script = if let Some(hand_in) = event.hand_in_on(map_id, npc_id) {
            let Some(view) = ticket.balloon(hand_in.stage).await else {
                return Ok(false);
            };
            NpcScript::balloon(hand_in, view, &self.in_game()?.player)
        } else if npc_id == AQUA_BALLOON && ticket.kind == EventKind::Lpq {
            NpcScript::aqua_balloon()
        } else if npc_id == BLUE_BALLOON && ticket.kind == EventKind::Lpq {
            let puzzle = event.stage_on(map_id);
            let Some(stage) = puzzle.filter(|stage| event.puzzles.contains(stage)) else {
                return Ok(false);
            };
            let Some(view) = ticket.balloon(stage).await else {
                return Ok(false);
            };
            let stood = match view {
                Balloon::Answer { combo } => {
                    let taken = event.areas_taken;
                    let judge = |areas: &[Area], positions: &[(i32, i32)]| {
                        standing(combo, taken, areas, positions)
                    };
                    let Some(stood) = self.standing_on(map_id, judge).await? else {
                        return Ok(false);
                    };
                    if stood == Standing::Wrong {
                        self.show_wrong_answer().await?;
                    }
                    Some(stood)
                }
                _ => None,
            };
            NpcScript::blue_balloon(stage, view, stood)
        } else {
            return Ok(false);
        };
        self.start_script(script).await?;
        Ok(true)
    }

    /// What Eak says to the player in the Orbis party quest, if anything:
    /// in the music room whether its music box plays, in the sealed room
    /// how the team stands on its platforms (a wrong count shows as a
    /// wrong answer), and elsewhere his lines for the cloud stage and the
    /// hub.
    pub(super) async fn opq_eak(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(None);
        };
        if ticket.kind != EventKind::Opq {
            return Ok(None);
        }
        match map_id {
            events::OPQ_MUSIC_ROOM => {
                let cleared = ticket.cleared(events::OPQ_MUSIC_STAGE);
                let states = self.reactor_states(&[net::reactors::MUSIC_BOX]).await?;
                let played = states[0].is_some_and(|state| state > 0);
                Ok(Some(NpcScript::opq_music(played, cleared)))
            }
            events::OPQ_LEVER_ROOM => {
                if ticket.cleared(events::OPQ_LEVER_STAGE) {
                    return Ok(Some(NpcScript::opq_levers(None)));
                }
                let (reply, pair) = oneshot::channel();
                self.send_to_field(FieldMessage::Levers { reply }).await?;
                let Ok(pair) = pair.await else {
                    return Ok(None);
                };
                let states = self.reactor_states(&events::OPQ_LEVERS).await?;
                let mut up = [false; 5];
                for (up, state) in up.iter_mut().zip(states) {
                    *up = state.is_some_and(|state| state > 0);
                }
                let check = events::lever_check(pair, up);
                if matches!(check, LeverCheck::Wrong { .. }) {
                    self.show_wrong_answer().await?;
                }
                Ok(Some(NpcScript::opq_levers(Some(check))))
            }
            events::OPQ_HEAD_COUNT_ROOM => {
                if ticket.cleared(events::OPQ_HEAD_COUNT_STAGE) {
                    return Ok(Some(NpcScript::opq_head_count(None)));
                }
                let (reply, split) = oneshot::channel();
                self.send_to_field(FieldMessage::Split { reply }).await?;
                let Ok(split) = split.await else {
                    return Ok(None);
                };
                let judge = |areas: &[Area], positions: &[(i32, i32)]| {
                    events::head_count(split, areas, positions)
                };
                let Some(count) = self.standing_on(map_id, judge).await? else {
                    return Ok(None);
                };
                if matches!(count, HeadCount::Wrong { .. }) {
                    self.show_wrong_answer().await?;
                }
                Ok(Some(NpcScript::opq_head_count(Some(count))))
            }
            _ => Ok(NpcScript::opq_eak(
                map_id,
                ticket.leader(),
                ticket.cleared(1),
            )),
        }
    }

    /// Where a climb pad of the Orbis party quest, `portal` running
    /// `script`, sends the player: a sound and a warp on the same map.
    /// `None` off the quest's lounge and tall room.
    pub(super) async fn opq_climb(
        &mut self,
        script: &str,
        portal: &PortalTemplate,
    ) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let in_opq = self
            .event
            .as_ref()
            .is_some_and(|ticket| ticket.holds(map_id) && ticket.kind == EventKind::Opq);
        if !in_opq || ![scripts::OPQ_LOUNGE_CLIMB, scripts::OPQ_TOWER_CLIMB].contains(&script) {
            return Ok(None);
        }
        let (reply, combo) = oneshot::channel();
        self.send_to_field(FieldMessage::Climb { reply }).await?;
        let (Ok(combo), Ok(id)) = (combo.await, u8::try_from(portal.id)) else {
            return Ok(None);
        };
        Ok(events::climb(map_id, &combo, id, &portal.name)
            .map(|to| vec![Effect::PortalSound, Effect::WarpToPortal(map_id, to)]))
    }

    /// The states of the reactors named `names` on the player's map.
    async fn reactor_states(
        &mut self,
        names: &'static [&'static str],
    ) -> Result<Vec<Option<u8>>, RuntimeError> {
        let (reply, states) = oneshot::channel();
        self.send_to_field(FieldMessage::ReactorStates { names, reply })
            .await?;
        Ok(states.await.unwrap_or_else(|_| vec![None; names.len()]))
    }

    /// What `judge` makes of where everyone on `map_id`, the player's map,
    /// stands on its areas; `None` if it can't be told. Only the event's
    /// members can be on its copy.
    async fn standing_on<T>(
        &mut self,
        map_id: i32,
        judge: impl FnOnce(&[Area], &[(i32, i32)]) -> T,
    ) -> Result<Option<T>, RuntimeError> {
        let Some(field) = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id))
        else {
            return Ok(None);
        };
        let (reply, positions) = oneshot::channel();
        self.send_to_field(FieldMessage::Positions { reply })
            .await?;
        let Ok(positions) = positions.await else {
            return Ok(None);
        };
        let positions: Vec<(i32, i32)> = positions
            .into_iter()
            .map(|(x, y)| (i32::from(x), i32::from(y)))
            .collect();
        Ok(Some(judge(&field.areas, &positions)))
    }

    /// A wrong answer to a puzzle: everyone on the map sees and hears it.
    async fn show_wrong_answer(&mut self) -> Result<(), RuntimeError> {
        let mut packets = vec![
            build_field_effect("quest/party/wrong_kor")?,
            build_field_sound("Party1/Failed")?,
        ];
        for packet in &packets {
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::Broadcast(packet.clone()),
            })
            .await?;
        }
        self.send_all(&mut packets).await
    }

    /// The player's script did something their event keeps track of.
    pub(super) async fn tell_event(&self, command: EventCommand) {
        if let Some(ticket) = &self.event {
            ticket.command(command).await;
        }
    }

    /// The party went in: the player takes their ticket to the event's
    /// first map, unless they have since left the party or the lobby, or
    /// are in the Cash Shop or another event.
    pub(super) async fn event_started(&mut self, ticket: EventTicket) -> Result<(), RuntimeError> {
        let lobby = ticket.kind.event().lobby;
        let waiting = self.party_id.is_some()
            && !self.in_cash_shop
            && self.event.is_none()
            && self
                .game
                .as_ref()
                .is_some_and(|game| game.player.stats.map_id == lobby);
        if !waiting {
            // Dropping the ticket hands it back.
            return Ok(());
        }
        let kind = ticket.kind;
        let entry = kind.event().entry;
        self.event = Some(ticket);
        self.warp_within_event(entry).await?;
        if kind == EventKind::Opq {
            self.start_script(NpcScript::opq_arrival()).await?;
        }
        Ok(())
    }

    /// Event `id` ended: out of its maps, if the player still holds its
    /// ticket.
    pub(super) async fn event_ended(&mut self, id: u32) -> Result<(), RuntimeError> {
        if self.event.as_ref().is_none_or(|ticket| ticket.id != id) {
            return Ok(());
        }
        self.leave_event().await
    }

    /// The party lost what it protected in event `id`: the event ends,
    /// which sends everyone out.
    pub(super) async fn event_failed(&mut self, id: u32) {
        if let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.id == id) {
            ticket.command(EventCommand::Fail).await;
        }
    }

    /// The player's party passed from or to them, to `to`: their event, if
    /// they led it or now do, follows, and ends if `to` is not in it. The
    /// event checks who tells it, so a ticket's own view of the leader,
    /// which may be behind, doesn't matter here.
    pub(super) async fn handed_over(&mut self, to: ClientId) {
        if let Some(ticket) = &self.event {
            ticket.command(EventCommand::HandOver(to)).await;
        }
    }

    /// Without a party there is no event: the player leaves it, unless it
    /// is already won (Cosmic `playerLeft`).
    pub(super) async fn left_party_in_event(&mut self) -> Result<(), RuntimeError> {
        let playing = self.event.as_ref().is_some_and(|ticket| !ticket.won());
        if self.party_id.is_none() && playing {
            self.leave_event().await?;
        }
        Ok(())
    }

    /// Hand the ticket back, and go to the event's exit if on one of its
    /// maps.
    async fn leave_event(&mut self) -> Result<(), RuntimeError> {
        let Some(ticket) = self.event.as_ref() else {
            return Ok(());
        };
        let exit = ticket.kind.event().exit;
        if !self.in_cash_shop && ticket.holds(self.in_game()?.player.stats.map_id) {
            // Changing field to the exit hands the ticket back.
            return self.warp_within_event(exit).await;
        }
        self.drop_event_ticket().await
    }

    pub(super) async fn warp_within_event(&mut self, map_id: i32) -> Result<(), RuntimeError> {
        self.warp_within_event_to(map_id, 0).await
    }

    /// `warp_within_event`, arriving at `portal` on `map_id`.
    async fn warp_within_event_to(&mut self, map_id: i32, portal: u8) -> Result<(), RuntimeError> {
        let channel_id = self.channel_id;
        let player = &mut self.in_game_mut()?.player;
        for action in warp_to_spawn_point(player, map_id, portal, channel_id)? {
            self.apply(action).await?;
        }
        Ok(())
    }

    /// The player is going to `map_id`: off the event's maps, they hand
    /// their ticket back.
    pub(super) async fn leaving_for(&mut self, map_id: i32) -> Result<(), RuntimeError> {
        if self
            .event
            .as_ref()
            .is_some_and(|ticket| !ticket.holds(map_id))
        {
            self.drop_event_ticket().await?;
        }
        if self
            .wedding
            .as_ref()
            .is_some_and(|ticket| !ticket.holds(map_id))
        {
            self.drop_wedding_ticket().await?;
        }
        Ok(())
    }

    /// Hand the ticket back, and with it the event's own items.
    async fn drop_event_ticket(&mut self) -> Result<(), RuntimeError> {
        // Dropping the ticket hands it back.
        let Some(ticket) = self.event.take() else {
            return Ok(());
        };
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        let changes = ticket.kind.event().take_items(&mut game.player);
        if self.in_cash_shop {
            return Ok(());
        }
        self.send(&mut build_stop_clock()?).await?;
        if !changes.is_empty() {
            self.send(&mut build_inventory_operation(false, &changes)?)
                .await?;
        }
        Ok(())
    }

    /// Whether `packet`, a walk through a portal, would leave a stage before
    /// the party has cleared it. The client doesn't stop the player itself.
    pub(super) fn gate_closed(&self, packet: &Packet) -> bool {
        let (Some(ticket), Some(game)) = (&self.event, &self.game) else {
            return false;
        };
        let map_id = game.player.stats.map_id;
        if self.in_cash_shop || !ticket.holds(map_id) {
            return false;
        }
        portal_name(packet)
            .and_then(|portal| ticket.kind.event().gate_stage(map_id, &portal))
            .is_some_and(|stage| !ticket.cleared(stage))
    }

    /// The party cleared `stage` of event `id`: those on its map see and
    /// hear it and its gate opens, and everyone gets the stage's EXP. The
    /// last stage wins the event, and everyone's clock stops. A stage with a
    /// `clear_portal` first takes those on its map there (Cosmic's Eak warps
    /// the team before the clear effect).
    pub(super) async fn stage_cleared(&mut self, id: u32, stage: u8) -> Result<(), RuntimeError> {
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        let event = ticket.kind.event();
        let map_id = event.stage_map(stage);
        if let Some(portal) = event.clear_portal(stage) {
            if self.in_game()?.player.stats.map_id == map_id {
                self.warp_within_event_to(map_id, net::scripts::named(map_id, portal))
                    .await?;
            }
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        if next.stats.map_id == map_id {
            packets.push(build_field_effect("quest/party/clear")?);
            packets.push(build_field_sound("Party1/Clear")?);
            if event.gates {
                packets.push(build_field_object("gate")?);
            }
        }
        gain_exp(&mut next, event.stage_exp(stage), &mut packets)?;
        self.commit_player(next, None, &mut packets).await?;
        // The EXP comes before the clock stops, as in Cosmic's `clearPQ`.
        if stage == event.last_stage() {
            if let Some(ticket) = self.event.as_mut() {
                ticket.clock = None;
            }
            packets.push(build_stop_clock()?);
        }
        self.send_all(&mut packets).await
    }

    /// Event `id` takes the player, if on `from` (or anywhere in it), to
    /// `to`, where the time left is `clock`.
    pub(super) async fn event_moved(
        &mut self,
        id: u32,
        from: Option<i32>,
        to: i32,
        clock: Option<Instant>,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_mut().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        if !ticket.holds(map_id) || from.is_some_and(|from| from != map_id) {
            return Ok(());
        }
        // The clock shows on arriving; the client keeps one already shown.
        let shown = ticket.clock.is_some();
        ticket.clock = clock;
        self.warp_within_event(to).await?;
        if shown && clock.is_none() {
            self.send(&mut build_stop_clock()?).await?;
        }
        Ok(())
    }

    /// A cleared stage's gate stays open for whoever comes onto its map.
    pub(super) async fn show_event_gate(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(());
        };
        let event = ticket.kind.event();
        if !event.gates {
            return Ok(());
        }
        let cleared = event
            .stage_on(map_id)
            .is_some_and(|stage| ticket.cleared(stage));
        if !cleared {
            return Ok(());
        }
        self.send(&mut build_field_object("gate")?).await
    }

    /// On one of the event's maps the client shows the time left, until the
    /// event is won, and again during its bonus.
    pub(super) async fn show_event_clock(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let event = self.event.as_ref().filter(|ticket| ticket.holds(map_id));
        let wedding = self.wedding.as_ref().filter(|ticket| ticket.holds(map_id));
        let clock = match (event, wedding) {
            (Some(ticket), _) => ticket.clock,
            (None, Some(ticket)) => ticket.clock,
            (None, None) => None,
        };
        let Some(ends) = clock else {
            return Ok(());
        };
        let left = ends.saturating_duration_since(Instant::now());
        let seconds = u32::try_from(left.as_secs()).unwrap_or(u32::MAX);
        self.send(&mut build_countdown(seconds)?).await
    }

    /// A player who leaves the game leaves the event, and its items.
    pub(super) fn forget_event(&mut self) {
        // Dropping the wedding's ticket hands it back.
        self.wedding = None;
        if let (Some(ticket), Some(game)) = (self.event.take(), self.game.as_mut()) {
            ticket.kind.event().take_items(&mut game.player);
        }
    }
}
