//! The player's side of a party event (`crate::actor::event`): the ticket
//! that puts them in the event's own copy of its maps, the event's clock,
//! its stages' gates, and getting out when it ends or they leave.

use super::quest::{gain_exp, pay_mesos};
use super::ClientActor;
use crate::actor::event::EventTicket;
use crate::error::RuntimeError;
use crate::message::{CarnivalPick, DirectoryMessage, FieldKey, FieldMessage};
use game::inventory::InventoryType;
use game::quest_ex::{self, PartyQuestOutcome};
use game::QuestState;
use game_data::server_content::keys;
use game_data::{Area, PortalTemplate};
use net::carnival::SpendRefusal;
use net::events::{
    self, on_areas, standing, Aura, Balloon, ClockOn, Cloto, CwkKeeper, Entry, EventCommand,
    EventKind, HeadCount, JailCheck, LeverCheck, RopesTalk, SnowSpirit, Standing, Statue,
};
use net::handler::FieldRequest;
use net::npc_scripts::{NpcScript, AQUA_BALLOON, BLUE_BALLOON, PIETRI};
use net::packet::build::world::carnival::build_carnival_message;
use net::packet::build::world::conti::{
    build_boss_site_closing, build_countdown, build_stop_clock, BossSite,
};
use net::packet::build::world::effect::{
    build_blow_weather, build_field_effect, build_field_object, build_field_sound,
};
use net::packet::build::world::field::{
    build_dojo_energy, build_dojo_warp_up, build_pyramid_score,
};
use net::packet::build::world::inventory::{build_char_look, build_inventory_operation};
use net::packet::build::world::messaging::{build_broadcast, build_item_line, Broadcast};
use net::packet::build::world::quest::{build_quest_ex_record, build_quest_record};
use net::packet::build::world::stat::build_hp_mp;
use net::packet::handle::world::{portal_name, warp_to_spawn_point};
use net::scripts::{self, Effect};
use net::{ClientId, WorldAction};
use packet::Packet;
use rand::Rng;
use std::time::SystemTime;
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
        if kind.event().squad {
            return self.talk_about_squad(kind).await;
        }
        let Some(entry) = self.event_entry(kind).await? else {
            return Ok(());
        };
        let script = match kind {
            EventKind::Kpq => NpcScript::lakelis(entry),
            EventKind::Hpq => NpcScript::tory(entry),
            EventKind::Lpq => NpcScript::red_sign(entry),
            EventKind::Opq => NpcScript::wonky(entry),
            EventKind::Zpq => NpcScript::adobis(entry, &self.in_game()?.player),
            EventKind::Htpq => NpcScript::htpq_gatekeeper(entry),
            EventKind::Delli => NpcScript::delli(entry),
            EventKind::LudiMaze => NpcScript::rolly(entry),
            EventKind::Tylus => NpcScript::tylus(entry),
            EventKind::Elemental => NpcScript::flo(entry),
            EventKind::Ellin => NpcScript::ellin(entry, &self.in_game()?.player),
            EventKind::Treasure => NpcScript::charles(entry),
            EventKind::Pirate => NpcScript::guon(entry),
            EventKind::Holiday(level) => NpcScript::snow_spirit(level, entry),
            EventKind::PrimeMinister => NpcScript::blocked_entrance(entry),
            EventKind::Amoria => NpcScript::amos_lobby(entry),
            EventKind::Magatia(side) => NpcScript::magatia(side, entry),
            EventKind::Subway => {
                // He offers Train 999 too.
                let Some(train) = self.event_entry(EventKind::SubwayTrain).await? else {
                    return Ok(());
                };
                NpcScript::mr_lim(entry, train, &self.in_game()?.player)
            }
            EventKind::Dojo => {
                // He offers the run with a party too.
                let Some(party) = self.event_entry(EventKind::DojoParty).await? else {
                    return Ok(());
                };
                NpcScript::so_gong(entry, party, &self.in_game()?.player)
            }
            // A squad's, above; Papulatus's and Latanica's have a portal,
            // not an NPC; So Gong starts his trial and the party run from
            // the Dojo's own talk.
            EventKind::Zakum
            | EventKind::Horntail
            | EventKind::Cwk
            | EventKind::Showa
            | EventKind::Balrog
            | EventKind::Scarga
            | EventKind::PinkBean
            | EventKind::Papulatus
            | EventKind::Latanica => return Ok(()),
            EventKind::DojoTrial | EventKind::DojoParty => return Ok(()),
            // Cesar opens the arenas (`talk_to_ariant_lobby`).
            EventKind::Ariant(_) => return Ok(()),
            // Duarte asks after every run at once (`talk_to_duarte`).
            EventKind::Pyramid(_) | EventKind::PyramidParty(_) => return Ok(()),
            // Duarte takes the gem, or sends the player in from the Shades.
            EventKind::PyramidTomb(_) => return Ok(()),
            // Mr. Lim boards Train 999 from the Dusty Platform's own talk,
            // and from the result map.
            EventKind::SubwayTrain => return Ok(()),
            // Mu Young's start script for 2245 takes the player in, and
            // June's own talk and 2291's end script the Spirit of Rock's.
            EventKind::Tristan | EventKind::RockSpirit | EventKind::RockSpiritVip => return Ok(()),
            // The East Castle Tower's portal takes the player in, and the
            // Wedding Hall's door likewise; Olson's own talk the dollhouse
            // room's, Cheng's the Toy Factory's Sector 4 and Kenta's the
            // Watch Hog's room; the Tera Forest's `in00` Guardian Nex's,
            // and `dragonNest` the Nine Spirit's Nest.
            EventKind::KingPepe
            | EventKind::WeddingHall
            | EventKind::Dollhouse
            | EventKind::MechanicalParts
            | EventKind::MonsterRiding
            | EventKind::GuardianNex(_)
            | EventKind::NineSpirit
            | EventKind::NeoBoss(_) => return Ok(()),
            // Spiegelmann offers every room at once (`talk_to_spiegelmann`).
            EventKind::Carnival(_) => return Ok(()),
            // Shuang registers guilds, not parties (`talk_to_shuang`).
            EventKind::GuildQuest => return Ok(()),
        };
        self.start_script(script).await
    }

    /// Duarte in Nett's Pyramid's lobby tells the player which of its runs,
    /// alone or with their party, they may go on.
    pub(super) async fn talk_to_duarte(&mut self) -> Result<(), RuntimeError> {
        let mut entries = Vec::new();
        for kind in events::pyramid_kinds() {
            let Some(entry) = self.event_entry(kind).await? else {
                return Ok(());
            };
            entries.push((kind, entry));
        }
        let script = NpcScript::duarte(&entries, &self.in_game()?.player);
        self.start_script(script).await
    }

    /// Spiegelmann on the Carnival Field or Carnival Field 2 tells the
    /// player which of its rooms their party may take.
    pub(super) async fn talk_to_spiegelmann(&mut self) -> Result<(), RuntimeError> {
        let mut entries = Vec::new();
        let lobby = self.in_game()?.player.stats.map_id;
        for room in net::carnival::rooms_off(lobby) {
            let kind = EventKind::Carnival(room);
            let Some(entry) = self.event_entry(kind).await? else {
                return Ok(());
            };
            entries.push((kind, entry));
        }
        let (reply, rooms) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::CarnivalRooms {
            from: self.client_id,
            reply,
        })
        .await?;
        let Ok(rooms) = rooms.await else {
            return Ok(());
        };
        let script = NpcScript::spiegelmann(&entries, &rooms, &self.in_game()?.player);
        self.start_script(script).await
    }

    /// Cesar in the Ariant Coliseum's lobby shows the player its arenas.
    pub(super) async fn talk_to_ariant_lobby(&mut self) -> Result<(), RuntimeError> {
        let (reply, lobby) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::AriantLobby {
            from: self.client_id,
            reply,
        })
        .await?;
        let Ok(lobby) = lobby.await else {
            return Ok(());
        };
        let script = NpcScript::ariant_lobby(&lobby, &self.in_game()?.player);
        self.start_script(script).await
    }

    /// Cesar (2101017): at an arena's entrance, its roster; in its battle,
    /// on the player's run, the bombs and rocks; anywhere else, the
    /// Coliseum.
    pub(super) async fn talk_to_ariant_host(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(arena) = net::ariant::arena_at(map_id) else {
            let taken = (self.event.as_ref())
                .filter(|ticket| {
                    matches!(ticket.kind, EventKind::Ariant(_))
                        && net::ariant::battle_of(map_id).is_some()
                        && ticket.holds(map_id)
                })
                .map(|ticket| ticket.prize_taken);
            return self.start_script(NpcScript::ariant_battle(taken)).await;
        };
        let (reply, host) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::AriantHost {
            from: self.client_id,
            reply,
        })
        .await?;
        let Ok(host) = host.await else {
            return Ok(());
        };
        self.start_script(NpcScript::ariant_host(arena, &host))
            .await
    }

    /// The player, a party's leader, picked Monster Carnival room `room`:
    /// a free one their party takes, and one where a party waits they
    /// challenge. Spiegelmann says how it went.
    pub(super) async fn pick_carnival_room(&mut self, room: u8) -> Result<(), RuntimeError> {
        let (reply, picked) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::CarnivalPick {
            from: self.client_id,
            room,
            reply,
        })
        .await?;
        let refusal = match picked.await {
            Ok(CarnivalPick::Start(start)) => {
                crate::actor::event::spawn(
                    start,
                    self.services.fields.clone(),
                    self.services.directory.clone(),
                );
                return Ok(());
            }
            Ok(CarnivalPick::Sent) => None,
            Ok(CarnivalPick::Refused(refusal)) => Some(refusal),
            Err(_) => return Ok(()),
        };
        self.start_script(NpcScript::spiegelmann_picked(room, refusal))
            .await
    }

    /// The player, leading the party waiting in a Monster Carnival room,
    /// answered the challenge `challenger` put to them.
    pub(super) async fn answer_carnival(
        &mut self,
        challenger: ClientId,
        accept: bool,
    ) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::CarnivalAnswer {
            from: self.client_id,
            challenger,
            accept,
        })
        .await
    }

    /// Another party's leader challenged the player's, waiting in a
    /// Monster Carnival room: the player is asked, if still there.
    pub(super) async fn carnival_challenged(
        &mut self,
        challenger: ClientId,
        challengers: Vec<net::carnival::Challenger>,
    ) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let waiting = self.event.as_ref().is_some_and(|ticket| {
            matches!(ticket.kind, EventKind::Carnival(_)) && ticket.holds(map_id)
        });
        if !waiting || self.in_cash_shop {
            return self.answer_carnival(challenger, false).await;
        }
        self.start_script(NpcScript::carnival_challenge(challenger, &challengers))
            .await
    }

    /// The player spends CP from the Monster Carnival window: their event
    /// answers, or, for one not in a Carnival, the window hears it failed.
    pub(super) async fn spend_carnival_cp(
        &mut self,
        tab: u8,
        index: u8,
    ) -> Result<(), RuntimeError> {
        let playing = self
            .event
            .as_ref()
            .is_some_and(|ticket| matches!(ticket.kind, EventKind::Carnival(_)));
        if !playing {
            return self
                .send(&mut build_carnival_message(SpendRefusal::Failed)?)
                .await;
        }
        self.tell_event(EventCommand::CarnivalSpend { tab, index })
            .await;
        Ok(())
    }

    /// The player's side in the event whose field they are on, which
    /// others see: a Monster Carnival's, or 0.
    pub(super) fn event_side(&self) -> u8 {
        self.event
            .as_ref()
            .filter(|ticket| matches!(ticket.kind, EventKind::Carnival(_)))
            .map_or(0, |ticket| ticket.side)
    }

    /// Whether the player's party may go into an event of `kind` now; `None`
    /// if the directory dropped the question.
    async fn event_entry(&mut self, kind: EventKind) -> Result<Option<Entry>, RuntimeError> {
        let (reply, entry) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::EventEntry {
            from: self.client_id,
            kind,
            reply,
        })
        .await?;
        Ok(entry.await.ok())
    }

    /// The NPC who gathers a squad tells the player where they stand with
    /// the one on their channel.
    async fn talk_about_squad(&mut self, kind: EventKind) -> Result<(), RuntimeError> {
        let (reply, view) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::SquadTalk {
            from: self.client_id,
            kind,
            reply,
        })
        .await?;
        let Ok(view) = view.await else {
            return Ok(());
        };
        let Some(script) = NpcScript::squad(kind, view, &self.in_game()?.player) else {
            return Ok(());
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
    /// crates' puzzle, or the maze's one line; or Pietri at the end of the
    /// Ludibrium Maze, who takes its coupons. Returns whether it spoke;
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
        } else if npc_id == PIETRI
            && ticket.kind == EventKind::LudiMaze
            && map_id == events::MAZE_GOAL
        {
            // Cosmic `isEventTeamTogether`: only the team can be on the
            // copy, so all of it is here when as many are.
            let (leader, won) = (ticket.leader(), ticket.won());
            let team = ticket.team_size().await;
            let here = self.standing_on(map_id, |_, at| at.len()).await?;
            let together = matches!((team, here), (Some(team), Some(here)) if here >= team);
            NpcScript::pietri(leader, won, together, &self.in_game()?.player)
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

    /// What Ellin says to a member on the Forest of Poison Haze's fourth
    /// stage, as the Poison Flowers left there stand
    /// (`NpcScript::ellin_flowers`); one not holding the Monster Marbles
    /// has heard her out after. `None` anywhere else.
    pub(super) async fn ellin_flowers(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| {
            ticket.kind == EventKind::Ellin
                && ticket.holds(map_id)
                && map_id == events::ELLIN_STAGES[3]
        }) else {
            return Ok(None);
        };
        let (leads, heard) = (ticket.leader(), ticket.hint_heard);
        let (reply, mobs) = oneshot::channel();
        self.send_to_field(FieldMessage::MobCount { reply }).await?;
        let flowers = mobs.await.unwrap_or(1);
        let player = &self.in_game()?.player;
        let script = NpcScript::ellin_flowers(leads, heard, flowers, player);
        let wanted = i32::from(events::MONSTER_MARBLES_WANTED);
        let holds = player.inventory.count(events::MONSTER_MARBLE) >= wanted;
        if let Some(ticket) = self.event.as_mut() {
            ticket.hint_heard |= !holds;
        }
        Ok(Some(script))
    }

    /// What Guon says to a member aboard the Pirate Ship
    /// (`NpcScript::guon_aboard`): to the leader in a bellflower room, as
    /// its monsters stand. `None` off the ship.
    pub(super) async fn guon_aboard(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self
            .event
            .as_ref()
            .filter(|ticket| ticket.kind == EventKind::Pirate && ticket.holds(map_id))
        else {
            return Ok(None);
        };
        let (leader, marks) = (ticket.leader(), ticket.marks());
        let bellflowers = (events::PIRATE_STAGES[2..4].iter()).any(|&deck| deck + 1 == map_id);
        let bare = if leader && bellflowers {
            let (reply, mobs) = oneshot::channel();
            self.send_to_field(FieldMessage::MobCount { reply }).await?;
            mobs.await.unwrap_or(1) == 0
        } else {
            false
        };
        let player = &self.in_game()?.player;
        Ok(Some(NpcScript::guon_aboard(player, leader, marks, bare)))
    }

    /// What Amos or the Glimmer Man, `npc_id`, says to the player on the
    /// Amorian Challenge's first stage, if there.
    pub(super) async fn amoria_stage1(
        &mut self,
        npc_id: i32,
    ) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| {
            ticket.kind == EventKind::Amoria
                && map_id == events::AMORIA_ENTRY
                && ticket.holds(map_id)
        }) else {
            return Ok(None);
        };
        let (leader, fierry, shard) = (ticket.leader(), ticket.fierry(), ticket.shard());
        if npc_id == net::npc_scripts::AMOS_STAGE {
            let cleared = ticket.cleared(1);
            return Ok(Some(NpcScript::amos_stage1(leader, fierry, shard, cleared)));
        }
        let bare = if leader && !fierry && !shard {
            let (reply, mobs) = oneshot::channel();
            self.send_to_field(FieldMessage::MobCount { reply }).await?;
            mobs.await.unwrap_or(1) == 0
        } else {
            false
        };
        let player = &self.in_game()?.player;
        Ok(Some(NpcScript::glimmer_man(
            player, leader, fierry, shard, bare,
        )))
    }

    /// What the Snow Spirit says to the player in a Holiday PQ arena, if
    /// there on its run: before the snowman is out, the leader brings it
    /// out; after, how it grows; once it is whole, the leader brings out
    /// Scrooge; after the win, the member's prize or gift (Cosmic
    /// `9105004`'s `eim` branch).
    pub(super) async fn snow_spirit_arena(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| {
            matches!(ticket.kind, EventKind::Holiday(_))
                && events::holiday_arena(map_id).is_some()
                && ticket.holds(map_id)
        }) else {
            return Ok(None);
        };
        let (leader, won, taken) = (ticket.leader(), ticket.won(), ticket.prize_taken);
        let EventKind::Holiday(level) = ticket.kind else {
            return Ok(None);
        };
        let (reply, arena) = oneshot::channel();
        self.send_to_field(FieldMessage::HolidaySnowman { reply })
            .await?;
        let view = match arena.await.unwrap_or((0, false)) {
            _ if won => SnowSpirit::Won { taken },
            (_, true) => SnowSpirit::Scrooge,
            (form, false) => SnowSpirit::Snowman(form),
        };
        let player = &self.in_game()?.player;
        Ok(Some(NpcScript::snow_spirit_arena(
            level, leader, view, player,
        )))
    }

    /// What Amos says to the player on the Amorian Challenge's rope or
    /// platform stage, if there: how the leader's try weighs, by where the
    /// team stands on the stage's areas. A wrong try shows to the map as a
    /// wrong answer: on the ropes with a pink line (Cosmic `9201044`'s
    /// `mapMessage`), on the platforms with a slime for each platform
    /// right (`spawnMobs`).
    pub(super) async fn amoria_ropes(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let event = EventKind::Amoria.event();
        let on = [events::AMORIA_ROPES_STAGE, events::AMORIA_PLATFORMS_STAGE];
        let Some(stage) = event.stage_on(map_id).filter(|stage| on.contains(stage)) else {
            return Ok(None);
        };
        let in_amoria =
            |ticket: &&EventTicket| ticket.kind == EventKind::Amoria && ticket.holds(map_id);
        if self.event.as_ref().filter(in_amoria).is_none() {
            return Ok(None);
        }
        let (reply, mobs) = oneshot::channel();
        self.send_to_field(FieldMessage::MobCount { reply }).await?;
        let mobs = mobs.await.unwrap_or(1) > 0;
        let Some(heads) = self.standing_on(map_id, events::heads_on).await? else {
            return Ok(None);
        };
        let Some(ticket) = self.event.as_ref() else {
            return Ok(None);
        };
        let Some(talk) = ticket.ropes(stage, mobs, heads).await else {
            return Ok(None);
        };
        if stage == events::AMORIA_PLATFORMS_STAGE {
            if let RopesTalk::Wrong { right } = talk {
                self.bring_out_slimes(right).await?;
                self.show_wrong_answer().await?;
            }
            return Ok(Some(NpcScript::amos_platforms(talk)));
        }
        let notice = match talk {
            RopesTalk::Wrong { right: 0 } => Some(keys::AMORIA_PQ_STAGE2_ALL_DIFFER_NOTICE),
            RopesTalk::Wrong { .. } => Some(keys::AMORIA_PQ_STAGE2_ONE_SAME_NOTICE),
            RopesTalk::Reset => Some(keys::AMORIA_PQ_STAGE2_RESET_NOTICE),
            _ => None,
        };
        if let Some(notice) = notice {
            let line = net::content::text().get(notice, &[]);
            let mut packet = build_broadcast(Broadcast::Pink, &line)?;
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::BroadcastAll(packet.clone()),
            })
            .await?;
            self.send(&mut packet).await?;
            self.show_wrong_answer().await?;
        }
        Ok(Some(NpcScript::amos_ropes(talk)))
    }

    /// What Amos says to the player on the Amorian Challenge's gate stage,
    /// if there: whether the whole team stands at him, and every gate is
    /// open (Cosmic `9201045`).
    pub(super) async fn amoria_gather(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let in_amoria =
            |ticket: &&EventTicket| ticket.kind == EventKind::Amoria && ticket.holds(map_id);
        if map_id != events::AMORIA_STAGE5 || self.event.as_ref().filter(in_amoria).is_none() {
            return Ok(None);
        }
        let gates = self.reactor_states(&events::AMORIA_GATES).await?;
        let open = (gates.iter()).all(|&state| state == Some(events::AMORIA_GATE_OPEN));
        let Some(team) = self.event.as_ref() else {
            return Ok(None);
        };
        let Some(team) = team.team_size().await else {
            return Ok(None);
        };
        let together = self
            .standing_on(map_id, |areas, positions| {
                events::gathered(areas, positions, team)
            })
            .await?
            .unwrap_or(false);
        let Some(ticket) = self.event.as_ref() else {
            return Ok(None);
        };
        let Some(talk) = ticket.gather(together, open).await else {
            return Ok(None);
        };
        Ok(Some(NpcScript::amos_gather(talk)))
    }

    /// What Amos says to the player on the Amorian Challenge's last stage,
    /// if there, by whether they stand below, in its first area, or on its
    /// couples' bonus (Cosmic `9201045`).
    pub(super) fn amoria_geist(&self) -> Result<Option<NpcScript>, RuntimeError> {
        let game = self.in_game()?;
        let map_id = game.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| {
            ticket.kind == EventKind::Amoria
                && [events::AMORIA_STAGE6, events::AMORIA_COUPLES_BONUS].contains(&map_id)
                && ticket.holds(map_id)
        }) else {
            return Ok(None);
        };
        let (x, y) = (i32::from(game.position.0), i32::from(game.position.1));
        let below = map_id == events::AMORIA_STAGE6
            && net::get_game_data()
                .ok()
                .and_then(|data| data.field(map_id))
                .and_then(|field| field.areas.get(events::AMORIA_LIFT_AREA).copied())
                .is_some_and(|area| area.contains(x, y));
        let won = ticket.cleared(events::AMORIA_GEIST_STAGE);
        Ok(Some(NpcScript::amos_geist(
            &game.player,
            ticket.leader(),
            won,
            below,
        )))
    }

    /// Bring out `count` slimes on the Amorian Challenge's platform stage,
    /// each a random one at a random point.
    async fn bring_out_slimes(&mut self, count: usize) -> Result<(), RuntimeError> {
        let slimes: Vec<(i32, (i16, i16))> = {
            let mut rng = rand::thread_rng();
            (0..count)
                .map(|_| {
                    let slime =
                        events::AMORIA_SLIMES[rng.gen_range(0, events::AMORIA_SLIMES.len())];
                    let at =
                        events::AMORIA_SLIMES_AT[rng.gen_range(0, events::AMORIA_SLIMES_AT.len())];
                    (slime, at)
                })
                .collect()
        };
        for (mob_id, at) in slimes {
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::BringOutMobs {
                    mob_id,
                    at: vec![at],
                },
            })
            .await?;
        }
        Ok(())
    }

    /// What the hub's Romeo or Juliet, `npc_id`, says to the player in
    /// the hub of their side's party quest, if anything: whether the hub's
    /// stage is cleared and its way on shut decide it.
    pub(super) async fn magatia_hub(
        &mut self,
        npc_id: i32,
    ) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let side = (events::MagatiaSide::ALL.into_iter()).find(|&side| {
            net::npc_scripts::magatia_hub_host(side) == npc_id && side.stages()[3] == map_id
        });
        let Some(side) = side else {
            return Ok(None);
        };
        let Some(ticket) = self.event.as_ref().filter(|ticket| {
            ticket.kind == EventKind::Magatia(side) && ticket.holds(map_id) && !ticket.won()
        }) else {
            return Ok(None);
        };
        let leader = ticket.leader();
        let cleared = ticket.cleared(events::MAGATIA_FILES_STAGE);
        let door_shut = cleared
            && (self.reactor_states(&[side.hub_door()]).await?)
                .first()
                .is_some_and(|state| *state == Some(0));
        let player = &self.in_game()?.player;
        Ok(Some(NpcScript::magatia_hub(
            side, player, leader, cleared, door_shut,
        )))
    }

    /// What Yulete, `npc_id`, says to the player in his office in their
    /// run of Romeo and Juliet's party quest, as the field decides by where
    /// everyone stands and whether Romeo or Juliet read the letter, or in
    /// the boss's room, where he may bring out his Frankenroid; `None`
    /// anywhere else, or once he has left his office.
    pub(super) async fn yulete(&mut self, npc_id: i32) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self
            .event
            .as_ref()
            .filter(|ticket| matches!(ticket.kind, EventKind::Magatia(_)) && ticket.holds(map_id))
        else {
            return Ok(None);
        };
        let (shocked, caught, won) = (ticket.shocked(), ticket.caught_yulete(), ticket.won());
        let (reply, said) = oneshot::channel();
        self.send_to_field(FieldMessage::Yulete {
            shocked,
            caught,
            won,
            reply,
        })
        .await?;
        Ok(said
            .await
            .ok()
            .flatten()
            .map(|key| NpcScript::yulete(npc_id, key)))
    }

    /// What Romeo or Juliet in the boss's room once it is won, Yulete's
    /// traces, or the one who thanks the party, `npc_id`, says to the
    /// player in their run of Romeo and Juliet's party quest; `None`
    /// anywhere else.
    pub(super) async fn magatia_after(
        &mut self,
        npc_id: i32,
    ) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(None);
        };
        let EventKind::Magatia(side) = ticket.kind else {
            return Ok(None);
        };
        let (escort_failed, caught) = (ticket.escort_failed(), ticket.caught_yulete());
        let saved = net::npc_scripts::magatia_hub_host(side);
        if npc_id == saved && map_id == side.stages()[6] && ticket.won() {
            return Ok(Some(NpcScript::magatia_saved(side, escort_failed, caught)));
        }
        if npc_id == net::npc_scripts::traces(side) && map_id == side.traces_room() {
            return Ok(Some(NpcScript::yulete_traces(side)));
        }
        if npc_id == net::npc_scripts::MAGATIA_THANKS && map_id == side.thanks_room() {
            let player = &self.in_game()?.player;
            let mut rng = rand::thread_rng();
            return Ok(Some(NpcScript::magatia_thanks(
                side,
                player,
                escort_failed,
                &mut rng,
            )));
        }
        Ok(None)
    }

    /// What a portal of Romeo and Juliet's party quest, `portal` running
    /// `script`, does for the player: one of the landing's ways into the
    /// rope rooms takes them in unless another member took that room
    /// (Cosmic `rnj4_r1`), and a rope sends them a row up, or back down if
    /// it is the wrong one; reaching the top clears the ropes' stage the
    /// first time anyone does (Cosmic `rnj5_rp`). `None` for any other
    /// portal.
    pub(super) async fn magatia_portal(
        &mut self,
        script: &str,
        portal: &PortalTemplate,
    ) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(None);
        };
        let EventKind::Magatia(side) = ticket.kind else {
            return Ok(None);
        };
        let landing = side.stages()[5];
        let room = (scripts::MAGATIA_ROOMS_IN.iter())
            .find_map(|ways| ways.iter().position(|&way| way == script));
        if let Some(room) = room.filter(|_| map_id == landing) {
            let room = room as u8;
            return Ok(Some(if ticket.take_room(room) {
                vec![
                    Effect::PortalSound,
                    Effect::WarpToPortal(landing + 1 + i32::from(room), 0),
                ]
            } else {
                let line = net::content::text().get(keys::MAGATIA_ROOM_TAKEN, &[]);
                vec![Effect::Pink(line)]
            }));
        }
        if !scripts::MAGATIA_ROPES.contains(&script) || side.rope_room(map_id).is_none() {
            return Ok(None);
        }
        let (reply, combo) = oneshot::channel();
        self.send_to_field(FieldMessage::Climb { reply }).await?;
        let (Ok(combo), Ok(id)) = (combo.await, u8::try_from(portal.id)) else {
            return Ok(None);
        };
        let Some(to) = events::climb(map_id, &combo, id, &portal.name) else {
            return Ok(None);
        };
        if to == 1 {
            self.tell_event(EventCommand::ClearStage(events::MAGATIA_ROPES_STAGE))
                .await;
        }
        Ok(Some(vec![
            Effect::PortalSound,
            Effect::WarpToPortal(map_id, to),
        ]))
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
            events::OPQ_SALION_ROOM => {
                let cleared = ticket.cleared(events::OPQ_SALION_STAGE);
                let (reply, mobs) = oneshot::channel();
                self.send_to_field(FieldMessage::MobCount { reply }).await?;
                let (reply, drops) = oneshot::channel();
                self.send_to_field(FieldMessage::DropCount { reply })
                    .await?;
                let bare = mobs.await.unwrap_or(1) == 0 && drops.await.unwrap_or(1) == 0;
                Ok(Some(NpcScript::opq_salion(bare, cleared)))
            }
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
            events::OPQ_HUB if ticket.leader() => {
                let garden = ticket.cleared(events::OPQ_GARDEN_STAGE);
                let won = ticket.cleared(ticket.kind.event().last_stage());
                let scars = self.reactor_states(&events::OPQ_SCARS).await?;
                let statue = if !scars.iter().all(|state| state.is_some_and(|s| s > 0)) {
                    Statue::Gathering
                } else if !garden {
                    Statue::Garden
                } else if !won {
                    Statue::GrassOfLife
                } else {
                    Statue::Won
                };
                Ok(Some(NpcScript::opq_hub(statue)))
            }
            _ => Ok(NpcScript::opq_eak(
                map_id,
                ticket.leader(),
                ticket.cleared(1),
            )),
        }
    }

    /// What a portal of the Orbis party quest, `portal` running `script`,
    /// does for the player: a climb pad sends them on up the lounge or the
    /// tall room, or down, with a sound; the garden's way out takes the
    /// whole team back to the hub, for a leader holding the Grass of Life
    /// (Cosmic `party3_gardenin`), else the player hears what it needs;
    /// and the hub's storage door opens once the event is won (Cosmic
    /// `party3_room8`). `None` for any other portal.
    pub(super) async fn opq_portal(
        &mut self,
        script: &str,
        portal: &PortalTemplate,
    ) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self
            .event
            .as_ref()
            .filter(|ticket| ticket.holds(map_id) && ticket.kind == EventKind::Opq)
        else {
            return Ok(None);
        };
        if script == scripts::OPQ_GARDEN_OUT {
            let holds = self
                .in_game()?
                .player
                .inventory
                .count(events::GRASS_OF_LIFE)
                > 0;
            return Ok(Some(if ticket.leader() && holds {
                vec![
                    Effect::PortalSound,
                    Effect::EventCommand(EventCommand::MoveTeam(events::OPQ_HUB)),
                ]
            } else {
                let line = net::content::text().get(keys::OPQ_NOTICE_GARDEN_OUT, &[]);
                vec![Effect::Pink(line)]
            }));
        }
        if script == scripts::OPQ_STORAGE_IN && ticket.won() {
            return Ok(Some(vec![
                Effect::PortalSound,
                Effect::WarpToPortal(events::OPQ_STORAGE, 0),
            ]));
        }
        // The sealed room's doors to the jails open with the storage's
        // (Cosmic `party3_jail1`..`3`).
        let jail = scripts::OPQ_JAILS_IN
            .iter()
            .position(|&door| door == script);
        if let (Some(jail), true) = (jail, ticket.won()) {
            let (jail, _) = events::OPQ_JAILS[jail];
            return Ok(Some(vec![
                Effect::PortalSound,
                Effect::WarpToPortal(jail, 0),
            ]));
        }
        // A jail's way on to its box room, open to the right levers
        // (Cosmic `party3_jailin`), shows the whole jail how they stand.
        if script == scripts::OPQ_JAIL_ON {
            let (reply, check) = oneshot::channel();
            self.send_to_field(FieldMessage::Jail { reply }).await?;
            let Ok(Some(check)) = check.await else {
                return Ok(None);
            };
            match check {
                JailCheck::Misplaced(n) => {
                    self.show_wrong_answer().await?;
                    let n = n.to_string();
                    let line = net::content::text().get(keys::OPQ_NOTICE_JAIL_LEVERS, &[("n", &n)]);
                    return Ok(Some(vec![Effect::Pink(line)]));
                }
                // Cosmic cheers only the jail's opening.
                JailCheck::Solved => {
                    self.show_to_field(&["quest/party/clear"], &["Party1/Clear"])
                        .await?
                }
                JailCheck::Open => {}
            }
            return Ok(Some(vec![
                Effect::PortalSound,
                Effect::WarpToPortal(map_id + 2, 0),
            ]));
        }
        if ![scripts::OPQ_LOUNGE_CLIMB, scripts::OPQ_TOWER_CLIMB].contains(&script) {
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

    /// Aura to a player in the Dead Mine, as their ticket stands; `None`
    /// elsewhere.
    pub(super) fn zpq_aura(&self) -> Result<Option<NpcScript>, RuntimeError> {
        let player = &self.in_game()?.player;
        let map_id = player.stats.map_id;
        let Some(ticket) = self
            .event
            .as_ref()
            .filter(|ticket| ticket.holds(map_id) && ticket.kind == EventKind::Zpq)
        else {
            return Ok(None);
        };
        let view = if ticket.won() {
            Aura::Won {
                documents: ticket.cleared(events::ZPQ_DOCUMENTS),
                taken: ticket.prize_taken,
            }
        } else {
            Aura::Playing {
                leader: ticket.leader(),
            }
        };
        Ok(Some(NpcScript::aura(view, player)))
    }

    /// What the Dead Mine's way out (Cosmic `Zakum03`) does for the player:
    /// it stays shut, with a pink line, until the party has won and the
    /// player has taken their share from Aura. `None` for any other portal.
    pub(super) fn zpq_portal(&self, script: &str) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self
            .event
            .as_ref()
            .filter(|ticket| ticket.holds(map_id) && ticket.kind == EventKind::Zpq)
        else {
            return Ok(None);
        };
        if script != scripts::ZPQ_OUT {
            return Ok(None);
        }
        let pink = |key| vec![Effect::Pink(net::content::text().get(key, &[]))];
        Ok(Some(if !ticket.won() {
            pink(keys::ZAKUM_MINE_NOT_WON)
        } else if !ticket.prize_taken {
            pink(keys::ZAKUM_MINE_NO_PRIZE)
        } else {
            vec![Effect::PortalSound, Effect::Warp(events::ZPQ.exit)]
        }))
    }

    /// What the way out of Save Delli! does for a member there
    /// (`scripts::delli_way_out`). `None` for any other portal.
    pub(super) fn delli_portal(&self, script: &str) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let player = &self.in_game()?.player;
        let Some(ticket) = self
            .event
            .as_ref()
            .filter(|ticket| ticket.holds(player.stats.map_id) && ticket.kind == EventKind::Delli)
        else {
            return Ok(None);
        };
        if script != scripts::DELLI_OUT {
            return Ok(None);
        }
        Ok(Some(scripts::delli_way_out(ticket.won(), player)))
    }

    /// What the way out of Tylus's room does for a member there
    /// (`scripts::tylus_way_out`). `None` for any other portal.
    pub(super) fn tylus_portal(&self, script: &str) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let player = &self.in_game()?.player;
        let here = self.event.as_ref().is_some_and(|ticket| {
            ticket.holds(player.stats.map_id) && ticket.kind == EventKind::Tylus
        });
        if !here || script != scripts::TYLUS_OUT {
            return Ok(None);
        }
        Ok(Some(scripts::tylus_way_out(player)))
    }

    /// What the Forest of Poison Haze's way on does for a member there
    /// (`scripts::ellin_way_on`), the ways out of its maze's rooms
    /// (`scripts::ellin_maze_way`), which lead to its end one time in ten,
    /// and the way out of its last stage (`scripts::ellin_way_out`). `None`
    /// for any other portal.
    pub(super) fn ellin_portal(&self, script: &str) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let ticket = self
            .event
            .as_ref()
            .filter(|ticket| ticket.holds(map_id) && ticket.kind == EventKind::Ellin);
        match ticket {
            Some(ticket) if script == scripts::ELLIN_STAGE => {
                Ok(Some(scripts::ellin_way_on(map_id, |stage| {
                    ticket.cleared(stage)
                })))
            }
            Some(_) if map_id == events::ELLIN_STAGES[2] => {
                let lucky = rand::thread_rng().gen_bool(0.1);
                Ok(scripts::ellin_maze_way(script, lucky))
            }
            Some(ticket) if script == scripts::ELLIN_OUT => {
                Ok(Some(scripts::ellin_way_out(ticket.won(), ticket.leader())))
            }
            _ => Ok(None),
        }
    }

    /// What Crimsonwood Keep's ways on, and its fifth stage's ways into
    /// its rooms, do for a member there (`scripts::cwk_way_on`,
    /// `scripts::cwk_room_way`). `None` for any other portal.
    pub(super) fn cwk_portal(&self, script: &str) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let stats = &self.in_game()?.player.stats;
        let ticket = (self.event.as_ref())
            .filter(|ticket| ticket.holds(stats.map_id) && ticket.kind == EventKind::Cwk);
        let Some(ticket) = ticket else {
            return Ok(None);
        };
        Ok(scripts::cwk_way_on(script, |stage| ticket.cleared(stage))
            .or_else(|| scripts::cwk_room_way(script, stats.job / 100 % 10)))
    }

    /// What a keeper of one of the Keep's fifth stage's rooms, `npc_id`,
    /// tells the player (Cosmic `9201107` to `9201111`): of the room on the
    /// stage's hub, and of the job's heroes on the courtyard. In the room,
    /// to a member, once it is won (its monsters all down, or the thieves'
    /// watchful eyes all struck), the keeper gives its weapon, the first
    /// time and with room for it. `None` for anyone else, or elsewhere.
    pub(super) async fn cwk_keeper(
        &mut self,
        npc_id: i32,
    ) -> Result<Option<NpcScript>, RuntimeError> {
        let Some((job, room)) = events::cwk_keeper(npc_id) else {
            return Ok(None);
        };
        let player = &self.in_game()?.player;
        let map_id = player.stats.map_id;
        let full = player.inventory.free(InventoryType::Etc) == 0;
        let says = |key| Ok(Some(NpcScript::cwk_keeper(npc_id, CwkKeeper::Says(key))));
        match map_id {
            events::CWK_HUB => return says(room.lore[0]),
            events::CWK_COURTYARD => return says(room.lore[1]),
            _ => {}
        }
        let Some(not_yet) = room.not_yet.filter(|_| map_id == room.map) else {
            return Ok(None);
        };
        let in_cwk = |ticket: &&EventTicket| ticket.kind == EventKind::Cwk && ticket.holds(map_id);
        if !self.event.as_ref().is_some_and(|ticket| in_cwk(&ticket)) {
            return Ok(None);
        }
        let won = if map_id == events::CWK_EYES_ROOM {
            let eyes = self.reactor_states(&events::CWK_EYES).await?;
            eyes.iter().all(|&state| state == Some(1))
        } else {
            let (reply, mobs) = oneshot::channel();
            self.send_to_field(FieldMessage::MobCount { reply }).await?;
            mobs.await.is_ok_and(|mobs| mobs == 0)
        };
        let Some(ticket) = self.event.as_ref().filter(in_cwk) else {
            return Ok(None);
        };
        let talk = if !won {
            CwkKeeper::Says(not_yet)
        } else if ticket.room_taken(job) {
            CwkKeeper::Says(keys::CWK_ROOM_TAKEN)
        } else if full {
            CwkKeeper::Says(keys::CWK_ROOM_FULL)
        } else if ticket.take_room(job) {
            CwkKeeper::Gives(room.weapon)
        } else {
            CwkKeeper::Says(keys::CWK_ROOM_TAKEN)
        };
        Ok(Some(NpcScript::cwk_keeper(npc_id, talk)))
    }

    /// The host of the Keep's sixth stage (`events::cwk_host`, Cosmic
    /// `9201115`), to a member there. `None` for anyone else.
    pub(super) async fn cwk_host(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let on_stage = |ticket: &&EventTicket| {
            ticket.kind == EventKind::Cwk && ticket.holds(map_id) && map_id == events::CWK_BOSS
        };
        if !self.event.as_ref().is_some_and(|ticket| on_stage(&ticket)) {
            return Ok(None);
        }
        let (reply, mobs) = oneshot::channel();
        self.send_to_field(FieldMessage::MobCount { reply }).await?;
        let mobs = mobs.await.unwrap_or_default();
        let Some(ticket) = self.event.as_ref().filter(on_stage) else {
            return Ok(None);
        };
        let talk = events::cwk_host(ticket.won(), ticket.leader(), ticket.cwk_host(), mobs);
        Ok(Some(NpcScript::cwk_host(talk)))
    }

    /// Jack (`events::cwk_jack`, Cosmic `9201112`): what the stage the
    /// player is on is about, during the Keep's party quest.
    pub(super) fn cwk_jack(&self) -> Result<NpcScript, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let in_cwk = self
            .event
            .as_ref()
            .is_some_and(|ticket| ticket.kind == EventKind::Cwk && ticket.holds(map_id));
        Ok(NpcScript::cwk_jack(in_cwk.then_some(map_id)))
    }

    /// The Keep's third stage's way up (`scripts::CWK_STEP`, Cosmic
    /// `glpqPortal2`) takes a member back to the stage's start, and their
    /// event counts them (`EventCommand::CwkStep`). `None` elsewhere.
    pub(super) async fn cwk_step(&mut self) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let on_stage = self.event.as_ref().is_some_and(|ticket| {
            ticket.kind == EventKind::Cwk
                && ticket.holds(map_id)
                && map_id == events::CWK_ENTRY + 200
        });
        if !on_stage {
            return Ok(None);
        }
        self.tell_event(EventCommand::CwkStep).await;
        Ok(Some(vec![
            Effect::PortalSound,
            Effect::WarpToPortal(map_id, 0),
        ]))
    }

    /// The player, having moved, stands where the Keep's first stage
    /// hides its way on (`events::cwk_hidden_way`): before anyone has
    /// found it, it clears the stage and takes them back to the stage's
    /// start (Cosmic `glpqPortalDummy`). Found, it does nothing more.
    pub(super) async fn cwk_hidden_way(&mut self) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        let map_id = game.player.stats.map_id;
        let found = self.event.as_ref().is_some_and(|ticket| {
            ticket.kind == EventKind::Cwk && ticket.holds(map_id) && !ticket.cleared(1)
        });
        if self.in_cash_shop || !found || !events::cwk_hidden_way(map_id, game.position) {
            return Ok(());
        }
        self.tell_event(EventCommand::ClearStage(1)).await;
        let effects = [Effect::PortalSound, Effect::WarpToPortal(map_id, 0)];
        self.apply_effects(&effects).await?;
        Ok(())
    }

    /// What the Dojo's two portals do for a challenger on a floor. Its
    /// `dojang_up`, by the door, takes them up to the upper tier once the
    /// floor's boss is beaten (the client moves them, on 0xCF) and opens
    /// the door; its `dojang_next`, behind the door, leads to the next
    /// floor, or from the 38th to the rooftop. A rest floor has no door, so
    /// So Gong takes them on from there. `None` for any other portal.
    pub(super) async fn dojo_portal(
        &mut self,
        script: &str,
    ) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        if script == scripts::DOJO_TUTO {
            let trial = self
                .event
                .as_ref()
                .filter(|ticket| ticket.holds(map_id) && ticket.kind == EventKind::DojoTrial);
            return Ok(trial.map(|ticket| scripts::dojo_tuto(ticket.won())));
        }
        let Some(ticket) = self.event.as_ref().filter(|ticket| {
            ticket.holds(map_id) && matches!(ticket.kind, EventKind::Dojo | EventKind::DojoParty)
        }) else {
            return Ok(None);
        };
        let party = ticket.kind == EventKind::DojoParty;
        let Some(floor) = ticket.kind.event().stage_on(map_id) else {
            return Ok(None);
        };
        let cleared = ticket.cleared(floor);
        let pink = |key| vec![Effect::Pink(net::content::text().get(key, &[]))];
        match script {
            scripts::DOJO_UP if events::rest_floor(floor) => Ok(Some(Vec::new())),
            scripts::DOJO_UP if !cleared => Ok(Some(pink(keys::DOJO_UP_MONSTERS))),
            scripts::DOJO_UP => {
                self.send(&mut build_dojo_warp_up(events::DOJO_UP_PORTAL)?)
                    .await?;
                Ok(Some(vec![Effect::SetReactorState {
                    name: events::DOJO_DOOR,
                    state: 1,
                }]))
            }
            scripts::DOJO_NEXT if events::rest_floor(floor) => Ok(Some(Vec::new())),
            scripts::DOJO_NEXT if !cleared => Ok(Some(pink(keys::DOJO_NEXT_SHUT))),
            scripts::DOJO_NEXT => {
                let record = self.in_game()?.player.dojo;
                Ok(Some(scripts::dojo_next(floor, &record, party)))
            }
            _ => Ok(None),
        }
    }

    /// What the way into the Trial Grounds' rooms does for a member on
    /// their first map (`scripts::htpq_into_rooms`). `None` for any other
    /// portal.
    pub(super) async fn htpq_portal(
        &mut self,
        script: &str,
    ) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let player = &self.in_game()?.player;
        let map_id = player.stats.map_id;
        let Some(ticket) = self
            .event
            .as_ref()
            .filter(|ticket| ticket.holds(map_id) && ticket.kind == EventKind::Htpq)
        else {
            return Ok(None);
        };
        // The last room's door: the leader with the Blue Crystal Key lifts
        // its seal.
        if script == scripts::HTPQ_SEAL && map_id == events::HTPQ.stage_map(5) {
            let has_key = player.inventory.count(events::BLUE_CRYSTAL_KEY) > 0;
            let effects = scripts::htpq_last_seal(ticket.cleared(5), ticket.leader() && has_key);
            return Ok(Some(effects));
        }
        // The lamp's room's way on, as the lamp stands.
        if script == scripts::HTPQ_LAMP_DOOR && map_id == events::HTPQ_LAMP {
            let leader = ticket.leader();
            let [lamp] = self.reactor_states(&[scripts::HTPQ_LAMP_NAME]).await?[..] else {
                return Ok(None);
            };
            return Ok(Some(scripts::htpq_lamp_door(leader, lamp)));
        }
        if script != scripts::HTPQ_INTO_ROOMS {
            return Ok(None);
        }
        let holding_key = self
            .in_game()?
            .player
            .inventory
            .count(events::CRYSTAL_KEYS[0])
            > 0;
        let (reply, positions) = oneshot::channel();
        self.send_to_field(FieldMessage::Positions { reply })
            .await?;
        // Only the event's members can be on its copy.
        let alone = positions.await.map_or(true, |p| p.len() <= 1);
        Ok(Some(scripts::htpq_into_rooms(alone, holding_key)))
    }

    /// The way to Papulatus: the directory says whether the player's party
    /// may go in.
    pub(super) async fn papulatus_portal(&mut self) -> Result<Vec<Effect>, RuntimeError> {
        let (reply, entry) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::EventEntry {
            from: self.client_id,
            kind: EventKind::Papulatus,
            reply,
        })
        .await?;
        Ok(entry
            .await
            .map(scripts::papulatus_portal)
            .unwrap_or_default())
    }

    /// The way to Capt. Latanica: a player holding a White Essence is let
    /// in if the directory says their party may go.
    pub(super) async fn latanica_portal(&mut self) -> Result<Vec<Effect>, RuntimeError> {
        let player = &self.in_game()?.player;
        if player.inventory.count(net::events::WHITE_ESSENCE) == 0 {
            return Ok(scripts::latanica_portal(None));
        }
        let entry = self.event_entry(EventKind::Latanica).await?;
        Ok(entry
            .map(|entry| scripts::latanica_portal(Some(entry)))
            .unwrap_or_default())
    }

    /// The door to Neo City era `era`'s boss: the party is let in if the
    /// directory says it may go.
    pub(super) async fn neo_boss_portal(&mut self, era: u8) -> Result<Vec<Effect>, RuntimeError> {
        let entry = self.event_entry(EventKind::NeoBoss(era)).await?;
        Ok(entry
            .map(|entry| scripts::neo_boss_portal(era, entry))
            .unwrap_or_default())
    }

    /// The states of the reactors named `names` on the player's map.
    pub(super) async fn reactor_states(
        &mut self,
        names: &[&'static str],
    ) -> Result<Vec<Option<u8>>, RuntimeError> {
        let (reply, states) = oneshot::channel();
        let names = names.to_vec();
        let count = names.len();
        self.send_to_field(FieldMessage::ReactorStates { names, reply })
            .await?;
        Ok(states.await.unwrap_or_else(|_| vec![None; count]))
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

    /// The statue keeper's script for a member of the Guild Quest in the
    /// hall of statues, a wrong guess shown to the hall; None elsewhere.
    pub(super) async fn statue_keeper(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        if map_id != net::guild_quest::STATUE_HALL {
            return Ok(None);
        }
        let in_gq =
            |ticket: &&EventTicket| ticket.kind == EventKind::GuildQuest && ticket.holds(map_id);
        let Some(ticket) = self.event.as_ref().filter(in_gq) else {
            return Ok(None);
        };
        let Some(talk) = ticket.statues().await else {
            return Ok(None);
        };
        if talk == net::guild_quest::StatueTalk::Failed {
            self.show_wrong_answer().await?;
        }
        Ok(Some(NpcScript::statue_keeper(talk)))
    }

    /// The vassals' keeper's script for a member of the Guild Quest at the
    /// fountain, judging the offering on its floor, a wrong one shown to
    /// the map; None elsewhere.
    pub(super) async fn vassal_keeper(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        use net::guild_quest::{VassalTalk, FOUNTAIN};
        let map_id = self.in_game()?.player.stats.map_id;
        if map_id != FOUNTAIN {
            return Ok(None);
        }
        let in_gq =
            |ticket: &&EventTicket| ticket.kind == EventKind::GuildQuest && ticket.holds(map_id);
        if self.event.as_ref().filter(in_gq).is_none() {
            return Ok(None);
        }
        let Some(field) = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id))
        else {
            return Ok(None);
        };
        let (reply, items) = oneshot::channel();
        self.send_to_field(FieldMessage::FloorItems { reply })
            .await?;
        let items: Vec<_> = (items.await.unwrap_or_default().into_iter())
            .map(|(id, (x, y))| (id, (i32::from(x), i32::from(y))))
            .collect();
        let offering = net::guild_quest::offering(&items, &field.areas);
        let Some(ticket) = self.event.as_ref() else {
            return Ok(None);
        };
        let Some(talk) = ticket.vassals(offering).await else {
            return Ok(None);
        };
        if matches!(talk, VassalTalk::Wrong(_) | VassalTalk::Failed) {
            self.show_wrong_answer().await?;
        }
        Ok(Some(NpcScript::vassal_keeper(talk)))
    }

    /// Sharen III's soul on the Guild Quest's grave, to a member there.
    pub(super) async fn sharen_soul(&mut self) -> Result<Option<NpcScript>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        if map_id != net::guild_quest::GRAVE {
            return Ok(None);
        }
        let in_gq =
            |ticket: &&EventTicket| ticket.kind == EventKind::GuildQuest && ticket.holds(map_id);
        let Some(ticket) = self.event.as_ref().filter(in_gq) else {
            return Ok(None);
        };
        Ok(ticket.soul().await.map(NpcScript::sharen_soul))
    }

    /// Where the Guild Quest's ways between the waterway's end rooms and
    /// the grave take a member there, who is minded of the room they
    /// came from (`net::guild_quest::grid_way`). `None` for any other
    /// portal.
    pub(super) fn guild_quest_grid(
        &mut self,
        script: &str,
    ) -> Result<Option<Vec<Effect>>, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = (self.event.as_mut())
            .filter(|ticket| ticket.kind == EventKind::GuildQuest && ticket.holds(map_id))
        else {
            return Ok(None);
        };
        let Some(((to, portal), room)) = net::guild_quest::grid_way(script, ticket.grid) else {
            return Ok(None);
        };
        ticket.grid = room;
        Ok(Some(vec![
            Effect::PortalSound,
            Effect::WarpToPortal(to, portal),
        ]))
    }

    /// A wrong answer to a puzzle: everyone on the map sees and hears it.
    async fn show_wrong_answer(&mut self) -> Result<(), RuntimeError> {
        self.show_to_field(&["quest/party/wrong_kor"], &["Party1/Failed"])
            .await
    }

    /// Show everyone on the player's map, the player too, the screen
    /// `effects` and play them the `sounds`.
    async fn show_to_field(
        &mut self,
        effects: &[&str],
        sounds: &[&str],
    ) -> Result<(), RuntimeError> {
        let mut packets = Vec::new();
        for effect in effects {
            packets.push(build_field_effect(effect)?);
        }
        for sound in sounds {
            packets.push(build_field_sound(sound)?);
        }
        for packet in &packets {
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::BroadcastAll(packet.clone()),
            })
            .await?;
        }
        self.send_all(&mut packets).await
    }

    /// The player's script did something their event keeps track of. A
    /// prize taken is the player's own, kept on their ticket.
    pub(super) async fn tell_event(&mut self, command: EventCommand) {
        if let Some(ticket) = &mut self.event {
            if command == EventCommand::PrizeTaken {
                ticket.prize_taken = true;
            } else {
                ticket.command(command).await;
            }
        }
    }

    /// The party went in: the player takes their ticket to the event's
    /// first map, unless they have since left the party (a squad's, or one
    /// taken alone, needs none) or the lobby, or are in the Cash Shop or another event.
    pub(super) async fn event_started(&mut self, ticket: EventTicket) -> Result<(), RuntimeError> {
        let event = ticket.kind.event();
        // A guild's registrant goes into its Guild Quest party or not, and
        // one taken alone (`Event::alone`) has none.
        let partyless =
            event.squad || event.solo() || event.alone || ticket.kind == EventKind::GuildQuest;
        let waiting = (self.party_id.is_some() || partyless)
            && !self.in_cash_shop
            && self.event.is_none()
            && self
                .game
                .as_ref()
                .is_some_and(|game| event.starts_on(game.player.stats.map_id));
        if !waiting {
            // Dropping the ticket hands it back.
            return Ok(());
        }
        let (kind, blessing) = (ticket.kind, ticket.blessing);
        let mut entry = kind.event().draw_entry(&mut rand::thread_rng());
        // A Dojo run starts with no energy (Cosmic `resetDojoEnergy`), and
        // a run alone after the floor So Gong recorded, erasing it.
        let dojo = matches!(kind, EventKind::Dojo | EventKind::DojoParty);
        let player = &mut self.in_game_mut()?.player;
        if dojo {
            player.dojo_energy = 0;
        }
        if kind == EventKind::Dojo && player.dojo.stage > 0 {
            entry = events::DOJO.stage_map(player.dojo.stage + 1);
            player.dojo.stage = 0;
            player.mark_dirty();
        }
        ticket.named(player.name());
        self.event = Some(ticket);
        if dojo {
            self.send(&mut build_dojo_energy(0)?).await?;
        }
        if kind == EventKind::NineSpirit {
            self.warp_within_event_to(entry, events::NINE_SPIRIT_NEST_ARRIVAL)
                .await?;
        } else {
            self.warp_within_event(entry).await?;
        }
        // A Monster Carnival's try is its battle, not its waiting room.
        if !matches!(kind, EventKind::Carnival(_)) {
            self.record_party_quest(kind, PartyQuestOutcome::Entered)
                .await?;
        }
        if kind == EventKind::Opq {
            self.start_script(NpcScript::opq_arrival()).await?;
        }
        // Once there, as Cosmic's `afterSetup` follows `playerEntry`.
        if let Some(item_id) = blessing {
            self.take_item_effect(item_id, false).await?;
        }
        Ok(())
    }

    /// Whether the player holds event `id`'s ticket and is on one of its
    /// maps.
    fn on_event_maps(&self, id: u32) -> bool {
        let Some(map_id) = self.game.as_ref().map(|game| game.player.stats.map_id) else {
            return false;
        };
        !self.in_cash_shop
            && (self.event.as_ref()).is_some_and(|ticket| ticket.id == id && ticket.holds(map_id))
    }

    /// Event `id`'s packets, sent if the player is on its maps.
    pub(super) async fn event_packets(
        &mut self,
        id: u32,
        packets: Vec<Packet>,
    ) -> Result<(), RuntimeError> {
        if self.on_event_maps(id) {
            for mut packet in packets {
                self.send(&mut packet).await?;
            }
        }
        Ok(())
    }

    /// The Nett's Pyramid run `id` reached a Pharaoh's Blessing: a living
    /// player on its maps takes it (Cosmic `checkBuffs`).
    pub(super) async fn pyramid_blessing(
        &mut self,
        id: u32,
        item_id: i32,
    ) -> Result<(), RuntimeError> {
        if self.on_event_maps(id) && self.in_game()?.player.stats.hp > 0 {
            self.take_item_effect(item_id, false).await?;
        }
        Ok(())
    }

    /// The Nett's Pyramid run `id` has `left` uses of Rage of Pharaoh, which
    /// the player's attacks check.
    pub(super) fn pyramid_rage(&mut self, id: u32, left: i32) -> Result<(), RuntimeError> {
        if self.event.as_ref().is_some_and(|ticket| ticket.id == id) {
            self.in_game_mut()?.player.pyramid_rage = left;
        }
        Ok(())
    }

    /// The player's own kills and cool kills on a Nett's Pyramid stage
    /// count toward Protector of Pharaoh (record 7760), and on a Dusty
    /// Platform stage toward Honorary Subway Worker (7662, shown as 7762),
    /// whether or not the quest has started.
    pub(super) async fn count_massacre_kills(
        &mut self,
        hits: net::pyramid::Hits,
    ) -> Result<(), RuntimeError> {
        let kills = i32::from(hits.kills) + i32::from(hits.cools);
        if kills == 0 || self.game.is_none() {
            return Ok(());
        }
        let game = self.in_game_mut()?;
        let records = net::pyramid::kill_records(game.field.key.map_id);
        let player = &mut game.player;
        let count = net::pyramid::kills_in(player, records[0]).saturating_add(kills);
        let count = count.to_string();
        for &record in records {
            player.quests.set_record(record, &count);
        }
        player.mark_dirty();
        let mut packets = (records.iter())
            .map(|&record| build_quest_record(record, &count))
            .collect::<Result<Vec<_>, _>>()?;
        self.send_all(&mut packets).await
    }

    /// A Massacre run ended with `rank` and `exp`: a player its end took to
    /// its result map, `exit`, sees them on the result screen and gets the
    /// EXP (Cosmic `sendScore`, with no rates). A Pyramid run `cleared`
    /// offers its mode's Tomb in the Shades, and a Dusty Platform one
    /// Train 999.
    pub(super) async fn pyramid_score(
        &mut self,
        exit: i32,
        rank: u8,
        exp: i32,
        cleared: bool,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop || self.in_game()?.player.stats.map_id != exit {
            return Ok(());
        }
        let gem = exit - events::PYRAMID_RESULT;
        if cleared && (0..events::PyramidMode::ALL.len() as i32).contains(&gem) {
            self.in_game_mut()?.player.pyramid_gem = Some(gem);
        }
        if cleared && exit == events::SUBWAY_RESULT {
            self.in_game_mut()?.player.subway_ride = true;
        }
        let mut packets = vec![build_pyramid_score(rank, exp)?];
        if exp > 0 {
            let mut next = self.in_game()?.player.clone();
            gain_exp(&mut next, exp, &mut packets)?;
            self.commit_player(next, None, &mut packets).await?;
        }
        self.send_all(&mut packets).await
    }

    /// The Monster Carnival battle `id` is over, as the player's side
    /// `won` or not with `points` CP in all: kept for 2042002 in their
    /// result room, where the event takes them next, and their diseases
    /// end (Cosmic `finish`'s `dispelDebuffs`).
    pub(super) async fn carnival_result(
        &mut self,
        id: u32,
        won: bool,
        points: i32,
    ) -> Result<(), RuntimeError> {
        if !self.on_event_maps(id) {
            return Ok(());
        }
        self.in_game_mut()?.player.carnival_result = Some((won, points));
        self.cure_diseases().await
    }

    /// The Ariant battle `id` has its results: the player's `points` are
    /// kept for Areda in the King's Room, and with them their jewels.
    pub(super) fn ariant_result(&mut self, id: u32, points: i32) {
        if !self.on_event_maps(id) {
            return;
        }
        if let Some(game) = self.game.as_mut() {
            game.player.ariant_tier = Some(points);
        }
    }

    /// A bomb knocked `count` Spirit Jewels out of the player's bag, which
    /// count against the Ariant battle's winner (Cosmic `addLostShards`).
    pub(super) fn ariant_jewels_lost(&mut self, count: i16) {
        if let Some(ticket) =
            (self.event.as_mut()).filter(|t| matches!(t.kind, EventKind::Ariant(_)))
        {
            ticket.jewels_lost = ticket.jewels_lost.saturating_add(count.max(0).into());
        }
    }

    /// Tell an Ariant battle how many Spirit Jewels the player holds, if
    /// that changed or a bomb knocked some out (Cosmic
    /// `updateAriantScore`).
    pub(super) async fn sync_ariant_jewels(&mut self) {
        let (Some(ticket), Some(game)) = (self.event.as_mut(), self.game.as_ref()) else {
            return;
        };
        if !matches!(ticket.kind, EventKind::Ariant(_)) || self.in_cash_shop {
            return;
        }
        let jewels = (game.player.inventory.count(net::ariant::SPIRIT_JEWEL))
            .try_into()
            .unwrap_or(i32::MAX);
        if ticket.jewels_told == Some(jewels) && ticket.jewels_lost == 0 {
            return;
        }
        ticket.jewels_told = Some(jewels);
        let lost = std::mem::take(&mut ticket.jewels_lost);
        (ticket.ariant_jewels(game.player.stats.name.clone(), jewels, lost)).await;
    }

    /// Record `outcome` in the party quest record of event `kind`, if it
    /// keeps one, with the rewards the player has: its info quest starts if
    /// it has not, and the client is sent the record whole.
    pub(super) async fn record_party_quest(
        &mut self,
        kind: EventKind,
        outcome: PartyQuestOutcome,
    ) -> Result<(), RuntimeError> {
        let Some((quest_id, rewards)) = kind.record_quest() else {
            return Ok(());
        };
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        let mut next = self.in_game()?.player.clone();
        let held: Vec<bool> = rewards
            .iter()
            .map(|&item_id| {
                next.inventory.count(item_id) > 0
                    || next.inventory.equipped_ids().any(|(_, id)| id == item_id)
            })
            .collect();
        quest_ex::record_party_quest(&mut next.quest_ex, quest_id, outcome, &held);
        let data = next.quest_ex.raw(quest_id).unwrap_or_default();
        if self.in_game()?.player.quest_ex.raw(quest_id) == Some(data) {
            return Ok(());
        }
        let mut packets = vec![build_quest_ex_record(quest_id, data)?];
        // The info quest starts with the first record, as a record of its
        // own (`game::quest::RECORDS`): the client never offers it.
        if next.quests.state(quest_id) == QuestState::NotStarted {
            next.quests.set_record(quest_id, "");
            packets.push(build_quest_record(quest_id, "")?);
        }
        if self.commit_player(next, None, &mut packets).await? {
            self.send_all(&mut packets).await?;
        }
        Ok(())
    }

    /// Event `id` ended: out of its maps, if the player still holds its
    /// ticket.
    pub(super) async fn event_ended(&mut self, id: u32) -> Result<(), RuntimeError> {
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        // A boss site whose time ran out, unwon, has closed.
        let closed = boss_site(ticket.kind).map(|(site, _)| site).filter(|_| {
            !self.in_cash_shop
                && !ticket.won()
                && ticket.clock.is_some_and(|ends| ends <= Instant::now())
        });
        if let Some(site) = closed {
            self.send(&mut build_boss_site_closing(site, false, 0)?)
                .await?;
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
    /// is already won (Cosmic `playerLeft`), a squad's, whose roster is not
    /// the party (CWKPQ's `leftParty` does nothing either), or one that
    /// lasts until empty (Papulatus's `leftParty` does nothing).
    pub(super) async fn left_party_in_event(&mut self) -> Result<(), RuntimeError> {
        let playing = self.event.as_ref().is_some_and(|ticket| {
            let event = ticket.kind.event();
            !ticket.won() && !event.squad && !event.lasts_until_empty()
        });
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
    /// their ticket back; onto one whose first arrival starts its clock
    /// over or stops it, they tell it. One that stops it takes the clock
    /// down at once, so it doesn't show there before the event answers.
    pub(super) async fn leaving_for(&mut self, map_id: i32) -> Result<(), RuntimeError> {
        // The Shades' gem is offered only on the way from the result map
        // to its room there. The field is the one being left: a warp may
        // have moved the player's map id already.
        let game = self.in_game_mut()?;
        let from = game.field.key.map_id;
        let player = &mut game.player;
        if let Some(mode) = player.pyramid_gem {
            if !(from == events::PYRAMID_RESULT + mode && map_id == events::PYRAMID_SHADES + mode) {
                player.pyramid_gem = None;
            }
        }
        if net::carnival::result_room(from).is_some() {
            player.carnival_result = None;
        }
        if from == events::SUBWAY_RESULT {
            player.subway_ride = false;
        }
        // Areda gives what a battle earned only in the King's Room, and
        // the jewels go with it.
        let mut unearned = Vec::new();
        if player.ariant_tier.is_some() && map_id != net::ariant::KINGS_ROOM {
            player.ariant_tier = None;
            unearned = take_jewels(player);
        }
        if !unearned.is_empty() && !self.in_cash_shop {
            self.send(&mut build_inventory_operation(false, &unearned)?)
                .await?;
        }
        let clock = events::clock_on(map_id);
        match &mut self.event {
            Some(ticket) if !ticket.holds(map_id) => self.drop_event_ticket().await?,
            Some(ticket) if clock.is_some() => {
                if clock == Some(ClockOn::Stop) && ticket.clock.take().is_some() {
                    self.send(&mut build_stop_clock()?).await?;
                }
                self.tell_event(EventCommand::Reached(map_id)).await
            }
            _ => {}
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
        // A won party quest's reward is usually had by now.
        if ticket.won() {
            self.record_party_quest(ticket.kind, PartyQuestOutcome::Left)
                .await?;
        }
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        // A battle's jewels stay for Areda once it has its results, and the
        // Watch Hog's room's items with one who won it (Cosmic `clearPQ`
        // unregisters them first).
        let (changes, worn) = match ticket.kind {
            EventKind::Ariant(_) if game.player.ariant_tier.is_some() => Default::default(),
            EventKind::MonsterRiding if ticket.won() => Default::default(),
            kind => kind.event().take_items(&mut game.player),
        };
        let refit = worn && game.player.fit_hp_mp();
        game.player.pyramid_rage = 0;
        // The blessing stays in the event (Cosmic `playerUnregistered`).
        // Only the player hears of it: they are leaving the map, and arrive
        // on the next without it.
        let unblessed = ticket
            .blessing
            .map(|item_id| game.player.buffs.cancel(-item_id))
            .unwrap_or_default();
        if self.in_cash_shop {
            return Ok(());
        }
        self.send(&mut build_stop_clock()?).await?;
        if !changes.is_empty() {
            self.send(&mut build_inventory_operation(false, &changes)?)
                .await?;
        }
        if !unblessed.is_empty() {
            self.send_cancelled(&unblessed).await?;
        }
        if worn {
            // A worn one shows off on the map (Sharenian's earring).
            let player = &self.in_game()?.player;
            let hp_mp = refit.then(|| build_hp_mp(&player.stats)).transpose()?;
            let look = build_char_look(player)?;
            if let Some(mut hp_mp) = hp_mp {
                self.send(&mut hp_mp).await?;
            }
            Box::pin(self.apply(WorldAction::LookChanged(look))).await?;
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

    /// Where `packet`, a walk through a portal, leads the player in the
    /// Amorian Challenge, if it is one of stage 1's ways on: the open one
    /// to its copy of stage 2, the others nowhere (`Some(None)`).
    pub(super) fn amoria_way(&self, packet: &Packet) -> Option<Option<i32>> {
        let (Some(ticket), Some(game)) = (&self.event, &self.game) else {
            return None;
        };
        let map_id = game.player.stats.map_id;
        if self.in_cash_shop || ticket.kind != EventKind::Amoria || !ticket.holds(map_id) {
            return None;
        }
        events::amoria_way(map_id, &portal_name(packet)?, ticket.way())
    }

    /// The player went through the Amorian Challenge's open way on from
    /// stage 1, onto `to` (Cosmic `apq00`-`apq02`).
    pub(super) async fn take_amoria_way(&mut self, to: i32) -> Result<(), RuntimeError> {
        self.warp_within_event_to(to, 0).await
    }

    /// Where the player, if dead on one of their event's maps, revives
    /// (`EventTicket::revive_to`): read as they answer the death dialog, as
    /// the event may have let them stay since they died. Returns whether
    /// they are dead there, so the event hears they revive.
    pub(super) fn set_revival(&mut self) -> bool {
        let Some(game) = self.game.as_mut() else {
            return false;
        };
        let map_id = game.player.stats.map_id;
        let ticket =
            (self.event.as_ref()).filter(|ticket| !self.in_cash_shop && ticket.holds(map_id));
        game.player.revive_to = ticket.and_then(|ticket| ticket.revive_to());
        ticket.is_some() && game.player.stats.hp <= 0
    }

    /// The player died in the Guild Quest's last room: the quest hears who
    /// stands there and how many mobs are left.
    pub(super) async fn fell_in_king_room(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let in_gq =
            |ticket: &EventTicket| ticket.kind == EventKind::GuildQuest && ticket.holds(map_id);
        if map_id != net::guild_quest::KING_ROOM || !self.event.as_ref().is_some_and(in_gq) {
            return Ok(());
        }
        let (reply, here) = oneshot::channel();
        self.send_to_field(FieldMessage::Positions { reply })
            .await?;
        let here = here.await.unwrap_or_default().len();
        let (reply, mobs) = oneshot::channel();
        self.send_to_field(FieldMessage::MobCount { reply }).await?;
        let mobs = mobs.await.unwrap_or_default();
        self.tell_event(EventCommand::KingRoomFell { here, mobs })
            .await;
        Ok(())
    }

    /// What the Rubian's keeper in Ergoth's room tells the player (Cosmic
    /// `9040010`): out of the quest, they are sent on its way out.
    pub(super) async fn rubian_keeper(&mut self) -> Result<NpcScript, RuntimeError> {
        use net::guild_quest::RUBIAN;
        let player = &self.in_game()?.player;
        let (map_id, count) = (player.stats.map_id, player.inventory.count(RUBIAN));
        let in_gq =
            |ticket: &&EventTicket| ticket.kind == EventKind::GuildQuest && ticket.holds(map_id);
        let talk = match self.event.as_ref().filter(in_gq) {
            Some(ticket) => ticket.rubian(count > 0).await,
            None => None,
        };
        let count = i16::try_from(count).unwrap_or(i16::MAX);
        Ok(NpcScript::rubian_keeper(talk, count))
    }

    /// A reactor on the player's map gave everyone there `exp`, such as
    /// the Forest of Poison Haze's fountain (Cosmic `giveCharacterExp`).
    pub(super) async fn reactor_exp(&mut self, exp: i32) -> Result<(), RuntimeError> {
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        gain_exp(&mut next, exp, &mut packets)?;
        self.commit_player(next, None, &mut packets).await?;
        self.send_all(&mut packets).await
    }

    /// The party cleared `stage` of event `id`: those on its map see and
    /// hear it and its gate opens, and everyone gets the stage's EXP. The
    /// last stage wins the event, and everyone's clock stops. A stage with a
    /// `clear_portal` first takes those on its map there (Cosmic's Eak warps
    /// the team before the clear effect). The Dead Mine's win shows nothing,
    /// and gives more EXP with the documents (Cosmic's Aura); the maze's win
    /// shows nothing either, and gives `exp`, by the coupons handed in
    /// (Cosmic's Pietri). Zakum's fall
    /// showed itself on the altar already (the field's `zakum_falls`), and
    /// gives none; nor do Horntail's stages, nor Delli's rescue.
    pub(super) async fn stage_cleared(
        &mut self,
        id: u32,
        stage: u8,
        exp: Option<i32>,
    ) -> Result<(), RuntimeError> {
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        let (ticket_kind, entered) = (ticket.kind, ticket.entered);
        let gate = ticket.gate_object(stage);
        let event = ticket.kind.event();
        let dojo = matches!(
            ticket.kind,
            EventKind::Dojo | EventKind::DojoTrial | EventKind::DojoParty
        );
        let zpq = ticket.kind == EventKind::Zpq;
        let zakum = ticket.kind == EventKind::Zakum;
        let no_exp = zakum
            || matches!(
                ticket.kind,
                EventKind::Horntail | EventKind::Htpq | EventKind::Delli
            );
        // The maze's win, the Pirate Ship's and the Watch Hog's room's have
        // no effect or sound (Cosmic `clearPQ`), only their EXP; a boss's
        // room has shown its own as the boss fell (`boss_falls`).
        let won = stage == event.last_stage();
        let quiet = zpq
            || zakum
            || ticket.kind == EventKind::LudiMaze
            || ticket.kind == EventKind::MonsterRiding
            || (ticket.kind.boss_falls() && won)
            || (ticket.kind == EventKind::Pirate && won)
            || (matches!(ticket.kind, EventKind::Magatia(_))
                && stage == events::MAGATIA_ROPES_STAGE);
        let exp = if let Some(exp) = exp {
            exp
        } else if zpq && ticket.cleared(events::ZPQ_DOCUMENTS) {
            events::ZPQ_DOCUMENTS_EXP
        } else {
            event.stage_exp(stage)
        };
        let map_id = event.stage_map(stage);
        if let Some(portal) = event.clear_portal(stage) {
            if self.in_game()?.player.stats.map_id == map_id {
                self.warp_within_event_to(map_id, net::scripts::named(map_id, portal))
                    .await?;
            }
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        // The knights' hall's clear shows on the maps off it too.
        let here = next.stats.map_id == map_id
            || event.stage_on(next.stats.map_id) == Some(stage)
            || (ticket_kind == EventKind::GuildQuest
                && map_id == net::guild_quest::KNIGHT_HALL
                && net::guild_quest::KNIGHT_MAPS.contains(&next.stats.map_id));
        if here && dojo {
            // A Dojo floor's own (Cosmic `Monster.killBy`, as its clear
            // marker comes out).
            packets.push(build_field_sound("Dojang/clear")?);
            packets.push(build_field_effect("dojang/end/clear")?);
        } else if here && !quiet {
            packets.push(build_field_effect("quest/party/clear")?);
            packets.push(build_field_sound("Party1/Clear")?);
            if let Some(gate) = gate {
                packets.push(build_field_object(&gate)?);
            }
        }
        // A stage worth no EXP shows no "+0" line.
        if !no_exp && exp != 0 {
            gain_exp(&mut next, exp, &mut packets)?;
        }
        pay_mesos(&mut next, event.stage_meso(stage), &mut packets)?;
        // The Keep's hidden way, found by one, is news to all.
        if ticket_kind == EventKind::Cwk && stage == 1 {
            let found = net::content::text().get(keys::CWK_STAGE1_FOUND, &[]);
            packets.push(build_broadcast(Broadcast::Pink, &found)?);
        }
        self.commit_player(next, None, &mut packets).await?;
        // The EXP comes before the clock stops, as in Cosmic's `clearPQ`;
        // So Gong's trial has none.
        if stage == event.last_stage() {
            if let Some(ticket) = self.event.as_mut() {
                if ticket.clock.take().is_some() {
                    packets.push(build_stop_clock()?);
                }
            }
        }
        self.send_all(&mut packets).await?;
        // The Holiday PQ's win leaves each member a Holiday Buff (Cosmic
        // `clearPQ`'s `applyEventPlayersItemBuff`).
        if let (true, EventKind::Holiday(level)) = (won, ticket_kind) {
            self.take_item_buff(events::holiday_win_buff(level)).await?;
        }
        if won {
            let cleared = PartyQuestOutcome::Cleared {
                took: entered.elapsed(),
                at: SystemTime::now(),
            };
            self.record_party_quest(ticket_kind, cleared).await?;
        }
        Ok(())
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
        // A party gathered on a Dojo rest floor earns each floor's points
        // from the one it was on (Cosmic `dojang_next`).
        let dojo_floors = (ticket.kind == EventKind::DojoParty)
            .then(|| {
                events::DOJO_PARTY
                    .stage_on(map_id)
                    .zip(events::DOJO_PARTY.stage_on(to))
            })
            .flatten()
            .filter(|&(_, rest)| events::rest_floor(rest));
        // The clock shows on arriving; the client keeps one already shown.
        let shown = ticket.clock.is_some();
        ticket.clock = clock;
        if let Some((floor, rest)) = dojo_floors.filter(|&(floor, rest)| floor < rest) {
            let record = self.in_game()?.player.dojo;
            self.apply_effects(&scripts::dojo_points(floor..=rest - 1, &record, false))
                .await?;
        }
        self.warp_within_event(to).await?;
        if shown && clock.is_none() {
            self.send(&mut build_stop_clock()?).await?;
        }
        Ok(())
    }

    /// Event `id`'s clock started over, to run out at `clock`, or stopped:
    /// shown or taken down at once on its maps.
    pub(super) async fn event_clock(
        &mut self,
        id: u32,
        clock: Option<Instant>,
    ) -> Result<(), RuntimeError> {
        let Some(ticket) = self.event.as_mut().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        let shown = ticket.clock.is_some();
        ticket.clock = clock;
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        match clock {
            Some(_) => self.show_event_clock().await,
            None if shown => self.send(&mut build_stop_clock()?).await,
            None => Ok(()),
        }
    }

    /// A cleared stage's gate stays open for whoever comes onto its map.
    pub(super) async fn show_event_gate(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(());
        };
        let gate = (ticket.kind.event().stage_on(map_id))
            .filter(|&stage| ticket.cleared(stage))
            .and_then(|stage| ticket.gate_object(stage));
        match gate {
            Some(gate) => self.send(&mut build_field_object(&gate)?).await,
            None => Ok(()),
        }
    }

    /// On a Dojo boss floor, the challenger hears the gong, sees the
    /// floor's number as they arrive, and has their energy bar refilled to
    /// what they have built (Cosmic `dojang_Eff`).
    pub(super) async fn show_dojo_floor(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let floor = self
            .event
            .as_ref()
            .filter(|ticket| {
                matches!(ticket.kind, EventKind::Dojo | EventKind::DojoParty)
                    && ticket.holds(map_id)
            })
            .and_then(|_| events::dojo_floor(map_id))
            .filter(|&floor| !events::rest_floor(floor));
        let Some(floor) = floor else {
            return Ok(());
        };
        let number = format!("dojang/start/number/{}", events::dojo_round(floor));
        let energy = self.in_game()?.player.dojo_energy;
        self.send_all(&mut vec![
            build_field_sound("Dojang/start")?,
            build_field_effect("dojang/start/stage")?,
            build_field_effect(&number)?,
            build_dojo_energy(energy)?,
        ])
        .await
    }

    /// On a stage of a Massacre run, the Pyramid's or the Dusty Platform's,
    /// the player sees "Stage N" and "Start!" as they arrive (Map.nx
    /// `onFirstUserEnter` `killing_MapSetting`, `Effect.img/killing/first`);
    /// in its bonus room, Pharaoh Yeti's Tomb or Train 999, "Bonus Stage"
    /// (`killing_BonusSetting`, `killing/bonus`). Each part waits out its
    /// own blank first frame, so they are sent together.
    pub(super) async fn show_massacre_stage(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(kind) = self
            .event
            .as_ref()
            .filter(|ticket| ticket.holds(map_id))
            .map(|ticket| ticket.kind)
        else {
            return Ok(());
        };
        let mut packets = if kind.massacre() {
            let Some(stage) = kind.event().stage_on(map_id) else {
                return Ok(());
            };
            vec![
                build_field_effect("killing/first/stage")?,
                build_field_effect(&format!("killing/first/number/{stage}"))?,
                build_field_effect("killing/first/start")?,
            ]
        } else if matches!(kind, EventKind::PyramidTomb(_) | EventKind::SubwayTrain) {
            vec![
                build_field_effect("killing/bonus/bonus")?,
                build_field_effect("killing/bonus/stage")?,
            ]
        } else {
            return Ok(());
        };
        self.send_all(&mut packets).await
    }

    /// Arriving on a Monster Carnival battle field, the player's screen
    /// takes the battle's score from the event (Cosmic
    /// `startMonsterCarnival`).
    /// The party taking a room sees, in its waiting room, that it waits
    /// for a challenge.
    pub(super) async fn show_carnival(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(side) = self
            .event
            .as_ref()
            .filter(|ticket| matches!(ticket.kind, EventKind::Carnival(_)) && ticket.holds(map_id))
            .map(|ticket| ticket.side)
        else {
            return Ok(());
        };
        // The revive room's screen takes the side from START too (b163).
        if net::carnival::battle_room(map_id).is_some()
            || net::carnival::revive_room(map_id).is_some()
        {
            self.tell_event(EventCommand::CarnivalArrived).await;
        } else if net::carnival::waiting_room(map_id).is_some() && side == 0 {
            let text = net::content::text().get(keys::CARNIVAL_WAITING, &[]);
            self.send(&mut build_item_line(&text, 0)?).await?;
        }
        Ok(())
    }

    /// Arriving on the Dojo's entrance, the player alone sees one of three
    /// taunts on a weather's message bar, taken down after 30 s (Cosmic
    /// `dojang_Msg`'s `startMapEffect`; its spawn-point check always holds,
    /// as the entrance's only spawn point is the one it asks for). Their
    /// energy empties there too. So Gong's trial room has its own taunt.
    pub(super) async fn taunt_challenger(&mut self) -> Result<(), RuntimeError> {
        let key = match self.in_game()?.player.stats.map_id {
            events::DOJO_ENTRANCE => {
                self.in_game_mut()?.player.dojo_energy = 0;
                self.send(&mut build_dojo_energy(0)?).await?;
                keys::DOJO_TAUNT[rand::thread_rng().gen_range(0, keys::DOJO_TAUNT.len())]
            }
            events::DOJO_TRIAL_MAP => keys::DOJO_TRIAL_TAUNT,
            _ => return Ok(()),
        };
        let taunt = net::content::text().get(key, &[]);
        let weather = (events::DOJO_TAUNT_WEATHER, taunt.as_str());
        self.send(&mut build_blow_weather(Some(weather))?).await?;
        self.reply_later(events::DOJO_TAUNT_TIME, build_blow_weather(None)?);
        Ok(())
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
        self.send(&mut build_countdown(seconds)?).await?;
        self.tell_closing_time(None).await
    }

    /// At an event's boss site a chat line says when it closes: on
    /// arrival, before or after the boss is out (the site has monsters
    /// only once it is), and again as it is summoned (Horntail's, as his
    /// clock starts over).
    pub(super) async fn tell_closing_time(
        &mut self,
        summoned: Option<bool>,
    ) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some((site, ends)) = self
            .event
            .as_ref()
            .and_then(|ticket| Some((boss_site(ticket.kind)?, ticket.clock?)))
            .filter(|((_, site_map), _)| *site_map == map_id)
            .map(|((site, _), ends)| (site, ends))
        else {
            return Ok(());
        };
        let summoned = match summoned {
            Some(summoned) => summoned,
            None => {
                let (reply, mobs) = oneshot::channel();
                self.send_to_field(FieldMessage::MobCount { reply }).await?;
                mobs.await.unwrap_or(0) > 0
            }
        };
        // Until it closes, at least a minute is left.
        let left = ends.saturating_duration_since(Instant::now()).as_secs();
        let minutes = u32::try_from(left.div_ceil(60)).unwrap_or(u32::MAX).max(1);
        self.send(&mut build_boss_site_closing(site, summoned, minutes)?)
            .await
    }

    /// A player who leaves the game leaves the event or the wedding, and
    /// their items.
    pub(super) fn forget_event(&mut self) {
        // Dropping the wedding's ticket hands it back.
        if let (Some(_), Some(game)) = (self.wedding.take(), self.game.as_mut()) {
            game::wedding::take_keys(&mut game.player);
        }
        if let (Some(ticket), Some(game)) = (self.event.take(), self.game.as_mut()) {
            let _ = ticket.kind.event().take_items(&mut game.player);
        }
        if let Some(game) = self.game.as_mut() {
            if game.player.ariant_tier.take().is_some() {
                take_jewels(&mut game.player);
            }
        }
    }
}

/// Take all the player's Spirit Jewels.
fn take_jewels(player: &mut game::Player) -> Vec<game::InventoryChange> {
    let held = i16::try_from(player.inventory.count(net::ariant::SPIRIT_JEWEL)).unwrap_or(i16::MAX);
    if held < 1 {
        return Vec::new();
    }
    (player.inventory_mut())
        .remove(net::ariant::SPIRIT_JEWEL, held)
        .unwrap_or_default()
}

/// The boss site an event is held at, whose closing time its players are
/// told, and its map.
fn boss_site(kind: EventKind) -> Option<(BossSite, i32)> {
    match kind {
        EventKind::Zakum => Some((BossSite::ZakumShrine, net::mobs::ZAKUM_ALTAR_MAP)),
        EventKind::Horntail => Some((BossSite::HorntailCave, events::HORNTAILS_CAVE)),
        _ => None,
    }
}
