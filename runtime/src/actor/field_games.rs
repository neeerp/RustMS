//! Omok and match cards on one field. Pure state, like `field_shops`: this
//! module decides who gets which message, and `FieldActor` delivers them.
//!
//! A game seats its owner in slot 0 and one visitor in slot 1. The visitor
//! says they are ready, the owner starts, and the two take turns until one
//! wins, they tie, or one gives up or leaves. Each player's client actor
//! keeps their record; the room shows copies and says how each game ended.
//! A player who leaves in the middle loses, and counts that themselves.

use crate::actor::field_trades::Outbox;
use crate::message::ServerMessage;
use game::{AvatarLook, GameOutcome, GameRecord, MiniRoomBox};
use net::error::NetworkError;
use net::mini_game::{self, MiniGameRequest, Omok, Placed, GAME_USERS, OMOK};
use net::packet::build::world::mini_room::{
    build_card_first, build_card_second, build_cards_start, build_enter_error, build_game_message,
    build_game_ready, build_game_result, build_game_room, build_game_turn, build_game_visit,
    build_omok_rejected, build_omok_start, build_omok_stone, build_room_chat, build_room_leave,
    build_seat_left, build_tie_asked, build_tie_denied, build_undo_asked, build_undo_result,
    enter_error, game_message, game_result, leave_reason,
};
use net::ClientId;
use packet::Packet;
use rand::Rng;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::error;

/// What a player asks of the game they are in, or opens one.
#[derive(Debug)]
pub enum GameRoomRequest {
    /// Open game `room` of `kind`, the sender its owner with `record`.
    Create {
        room: i32,
        kind: u8,
        title: String,
        password: Option<String>,
        piece: u8,
        record: GameRecord,
    },
    Play(MiniGameRequest),
    Chat {
        text: String,
    },
    Leave,
}

/// What a game tells a member's client actor.
#[derive(Debug)]
pub enum GameEvent {
    /// The player is in game `kind` now, in `slot`.
    Entered {
        kind: u8,
        slot: u8,
    },
    /// A game began, or ended so for the player.
    Started,
    Ended(GameOutcome),
    /// The player is out of the game; send the packet, if any.
    Left(Option<Packet>),
}

struct Member {
    client_id: ClientId,
    name: String,
    look: AvatarLook,
    record: GameRecord,
    /// Leave once this game ends.
    leave_after: bool,
}

/// What is on the table.
enum Board {
    Omok(Omok),
    /// The cards, -1 once matched, the turn's first card, and the pairs
    /// each slot has found.
    Cards {
        deck: Vec<i32>,
        first: Option<u8>,
        pairs: [u8; 2],
    },
}

/// A game being played.
struct Play {
    board: Board,
    /// Whose turn it is.
    turn: u8,
    /// Who moved first, with Omok's black stones (color 1).
    first: u8,
    /// Who asked for a tie, and whether anyone did this turn: v83 asks
    /// once a turn (report 164 §4, +0x73c).
    tie_asked: Option<u8>,
    tie_used: bool,
    /// Omok: who asked to take a stone back, and who already did.
    undo_asked: Option<u8>,
    undone: [bool; 2],
    /// When the turn began. Both clients send a skip once its time runs
    /// out (report 164 §4), so only the first after `Play::skip_after` counts.
    turn_since: Instant,
}

/// An Omok turn lasts 30 seconds on the client, a match cards turn 10 or
/// more (report b100 §2.7); these leave room for lag.
const OMOK_SKIP_AFTER: Duration = Duration::from_secs(25);
const CARDS_SKIP_AFTER: Duration = Duration::from_secs(8);

impl Play {
    fn skip_after(&self) -> Duration {
        match self.board {
            Board::Omok(_) => OMOK_SKIP_AFTER,
            Board::Cards { .. } => CARDS_SKIP_AFTER,
        }
    }

    fn color(&self, slot: u8) -> u8 {
        if slot == self.first {
            1
        } else {
            2
        }
    }

    fn pass_turn(&mut self) {
        self.give_turn(1 - self.turn);
        if let Board::Cards { first, .. } = &mut self.board {
            *first = None;
        }
    }

    /// The player in `slot` moves next, with a fresh turn.
    fn give_turn(&mut self, slot: u8) {
        self.turn = slot;
        self.tie_asked = None;
        self.tie_used = false;
        self.undo_asked = None;
        self.turn_since = Instant::now();
    }
}

struct Game {
    kind: u8,
    owner: Member,
    visitor: Option<Member>,
    title: String,
    password: Option<String>,
    piece: u8,
    ready: bool,
    play: Option<Play>,
    /// Who moves second in the next game: the last winner, so the loser
    /// opens (inferred from the colors 0x3e hands out); the owner at
    /// first.
    waits: u8,
}

impl Game {
    fn members(&self) -> impl Iterator<Item = (u8, &Member)> {
        std::iter::once((0, &self.owner)).chain(self.visitor.iter().map(|m| (1, m)))
    }

    fn member(&self, slot: u8) -> Option<&Member> {
        match slot {
            0 => Some(&self.owner),
            _ => self.visitor.as_ref(),
        }
    }

    fn member_mut(&mut self, slot: u8) -> Option<&mut Member> {
        match slot {
            0 => Some(&mut self.owner),
            _ => self.visitor.as_mut(),
        }
    }

    fn balloon(&self, room: i32) -> MiniRoomBox {
        MiniRoomBox {
            kind: self.kind,
            sn: room,
            title: self.title.clone(),
            skin: self.piece,
            users: self.members().count() as u8,
            max_users: GAME_USERS,
            private: self.password.is_some(),
            in_progress: self.play.is_some(),
        }
    }

    /// Send `packet` to everyone in the game.
    fn tell_all(&self, out: &mut Outbox, build: impl Fn() -> Result<Packet, NetworkError>) {
        for (_, member) in self.members() {
            packet(out, member.client_id, build());
        }
    }

    fn tell(&self, out: &mut Outbox, slot: u8, built: Result<Packet, NetworkError>) {
        if let Some(member) = self.member(slot) {
            packet(out, member.client_id, built);
        }
    }
}

#[derive(Default)]
pub struct Games {
    rooms: HashMap<i32, Game>,
    member_of: HashMap<ClientId, i32>,
    /// Owners whose balloon changed (`None`: it went), for the field to
    /// show.
    balloons: Vec<(ClientId, Option<MiniRoomBox>)>,
}

fn packet(out: &mut Outbox, to: ClientId, packet: Result<Packet, NetworkError>) {
    match packet {
        Ok(packet) => out.push((to, ServerMessage::SendPacket(packet))),
        Err(e) => error!(to, error = %e, "Failed to build a game packet"),
    }
}

fn event(out: &mut Outbox, to: ClientId, room: i32, event: GameEvent) {
    out.push((to, ServerMessage::MiniGame { room, event }));
}

fn leave_packet(packet: Result<Packet, NetworkError>) -> Option<Packet> {
    packet
        .map_err(|e| error!(error = %e, "Failed to build a game packet"))
        .ok()
}

impl Games {
    pub fn contains(&self, room: i32) -> bool {
        self.rooms.contains_key(&room)
    }

    pub fn is_member(&self, client_id: ClientId) -> bool {
        self.member_of.contains_key(&client_id)
    }

    /// The balloons that changed since last asked.
    pub fn take_balloons(&mut self) -> Vec<(ClientId, Option<MiniRoomBox>)> {
        std::mem::take(&mut self.balloons)
    }

    fn game_of(&self, client_id: ClientId) -> Option<(i32, u8)> {
        let id = *self.member_of.get(&client_id)?;
        let game = self.rooms.get(&id)?;
        let (slot, _) = game.members().find(|(_, m)| m.client_id == client_id)?;
        Some((id, slot))
    }

    fn show_balloon(&mut self, id: i32) {
        if let Some(game) = self.rooms.get(&id) {
            self.balloons
                .push((game.owner.client_id, Some(game.balloon(id))));
        }
    }

    /// `busy`: the owner is in another kind of room.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &mut self,
        room: i32,
        owner: ClientId,
        name: &str,
        look: &AvatarLook,
        kind: u8,
        title: String,
        password: Option<String>,
        piece: u8,
        record: GameRecord,
        busy: bool,
    ) -> Outbox {
        let mut out = Outbox::new();
        if busy || self.member_of.contains_key(&owner) || self.rooms.contains_key(&room) {
            event(&mut out, owner, room, GameEvent::Left(None));
            return out;
        }
        event(&mut out, owner, room, GameEvent::Entered { kind, slot: 0 });
        packet(
            &mut out,
            owner,
            build_game_room(kind, 0, &[(0, look, name, &record)], &title, piece),
        );
        self.rooms.insert(
            room,
            Game {
                kind,
                owner: Member {
                    client_id: owner,
                    name: name.to_string(),
                    look: look.clone(),
                    record,
                    leave_after: false,
                },
                visitor: None,
                title,
                password,
                piece,
                ready: false,
                play: None,
                waits: 0,
            },
        );
        self.member_of.insert(owner, room);
        self.show_balloon(room);
        out
    }

    /// `records`: the visitor's at Omok and at match cards.
    #[allow(clippy::too_many_arguments)]
    pub fn visit(
        &mut self,
        from: ClientId,
        room: i32,
        name: &str,
        look: &AvatarLook,
        password: Option<&str>,
        records: [GameRecord; 2],
        busy: bool,
    ) -> Outbox {
        let mut out = Outbox::new();
        let refusal = match self.rooms.get(&room) {
            _ if busy || self.member_of.contains_key(&from) => Some(enter_error::CLOSED),
            None => Some(enter_error::CLOSED),
            Some(game) if game.visitor.is_some() => Some(enter_error::FULL),
            Some(game) if game.password.is_some() && game.password.as_deref() != password => {
                Some(enter_error::WRONG_PASSWORD)
            }
            Some(_) => None,
        };
        if let Some(refusal) = refusal {
            packet(&mut out, from, build_enter_error(refusal));
            return out;
        }
        let game = self.rooms.get_mut(&room).expect("checked above");
        let record = records[usize::from(game.kind - 1)];
        game.visitor = Some(Member {
            client_id: from,
            name: name.to_string(),
            look: look.clone(),
            record,
            leave_after: false,
        });
        game.ready = false;
        let kind = game.kind;
        event(&mut out, from, room, GameEvent::Entered { kind, slot: 1 });
        let members: Vec<_> = game
            .members()
            .map(|(slot, m)| (slot, &m.look, m.name.as_str(), &m.record))
            .collect();
        packet(
            &mut out,
            from,
            build_game_room(kind, 1, &members, &game.title, game.piece),
        );
        packet(
            &mut out,
            game.owner.client_id,
            build_game_visit(kind, 1, look, name, &record),
        );
        self.member_of.insert(from, room);
        self.show_balloon(room);
        out
    }

    pub fn chat(&mut self, from: ClientId, text: &str) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, slot)) = self.game_of(from) else {
            return out;
        };
        let game = &self.rooms[&id];
        let name = &game.member(slot).expect("a member").name;
        let line = format!("{name} : {text}");
        game.tell_all(&mut out, || build_room_chat(slot, &line));
        out
    }

    pub fn play(&mut self, from: ClientId, request: MiniGameRequest) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, slot)) = self.game_of(from) else {
            return out;
        };
        let game = self.rooms.get_mut(&id).expect("a member's game");
        let other = 1 - slot;
        let playing = game.play.is_some();
        match request {
            MiniGameRequest::Ready(ready) if slot == 1 && !playing => {
                game.ready = ready;
                game.tell_all(&mut out, || build_game_ready(ready));
            }
            MiniGameRequest::Start if slot == 0 && !playing && game.ready => {
                return self.start(id, &mut rand::thread_rng());
            }
            MiniGameRequest::Expel if slot == 0 && !playing => {
                return self.expel(id);
            }
            MiniGameRequest::LeaveAfter(leave) if playing => {
                let member = game.member_mut(slot).expect("a member");
                member.leave_after = leave;
                let name = member.name.clone();
                let message = if leave {
                    game_message::LEAVE_AFTER
                } else {
                    game_message::STAY_AFTER
                };
                game.tell_all(&mut out, || build_game_message(message, &name));
            }
            MiniGameRequest::AskTie => {
                let Some(play) = game.play.as_mut().filter(|p| !p.tie_used) else {
                    return out;
                };
                play.tie_asked = Some(slot);
                play.tie_used = true;
                game.tell(&mut out, other, build_tie_asked());
            }
            MiniGameRequest::AnswerTie(accept) => {
                let Some(play) = game.play.as_mut().filter(|p| p.tie_asked == Some(other)) else {
                    return out;
                };
                play.tie_asked = None;
                if accept {
                    return self.finish(id, None, game_result::TIE);
                }
                game.tell(&mut out, other, build_tie_denied());
            }
            MiniGameRequest::GiveUp if playing => {
                let name = game.member(slot).expect("a member").name.clone();
                game.tell_all(&mut out, || {
                    build_game_message(game_message::FORFEITED, &name)
                });
                out.extend(self.finish(id, Some(other), game_result::FORFEIT));
            }
            MiniGameRequest::AskUndo => {
                let Some(play) = game.play.as_mut() else {
                    return out;
                };
                let color = play.color(slot);
                let Board::Omok(board) = &play.board else {
                    return out;
                };
                if play.undo_asked.is_some()
                    || play.undone[usize::from(slot)]
                    || !board.has_stones(color)
                {
                    return out;
                }
                play.undo_asked = Some(slot);
                game.tell(&mut out, other, build_undo_asked());
            }
            MiniGameRequest::AnswerUndo(accept) => {
                let Some(play) = game.play.as_mut().filter(|p| p.undo_asked == Some(other)) else {
                    return out;
                };
                play.undo_asked = None;
                if !accept {
                    game.tell(&mut out, other, build_undo_result(None));
                    return out;
                }
                let color = play.color(other);
                let Board::Omok(board) = &mut play.board else {
                    return out;
                };
                let count = board.undo(color) as u8;
                play.undone[usize::from(other)] = true;
                play.give_turn(other);
                game.tell_all(&mut out, || build_undo_result(Some((count, other))));
            }
            MiniGameRequest::Skip => {
                let Some(play) = game
                    .play
                    .as_mut()
                    .filter(|p| p.turn == slot && p.turn_since.elapsed() >= p.skip_after())
                else {
                    return out;
                };
                play.pass_turn();
                let turn = play.turn;
                game.tell_all(&mut out, || build_game_turn(turn));
            }
            MiniGameRequest::Move { x, y, color } => {
                let Some(play) = game.play.as_mut().filter(|p| p.turn == slot) else {
                    return out;
                };
                if color != play.color(slot) {
                    return out;
                }
                let Board::Omok(board) = &mut play.board else {
                    return out;
                };
                match board.place(x, y, color) {
                    Err(rejected) => {
                        game.tell(&mut out, slot, build_omok_rejected(rejected.code()));
                    }
                    Ok(placed) => {
                        play.pass_turn();
                        game.tell_all(&mut out, || build_omok_stone(x, y, color));
                        match placed {
                            Placed::Played => {}
                            Placed::Won => {
                                out.extend(self.finish(id, Some(slot), game_result::WON))
                            }
                            Placed::Full => out.extend(self.finish(id, None, game_result::TIE)),
                        }
                    }
                }
            }
            MiniGameRequest::Flip { first: true, index } => {
                let Some(play) = game.play.as_mut().filter(|p| p.turn == slot) else {
                    return out;
                };
                let Board::Cards { deck, first, .. } = &mut play.board else {
                    return out;
                };
                if first.is_some() || deck.get(usize::from(index)).is_none_or(|&c| c < 0) {
                    return out;
                }
                *first = Some(index);
                game.tell(&mut out, other, build_card_first(index));
            }
            MiniGameRequest::Flip {
                first: false,
                index,
            } => {
                let Some(play) = game.play.as_mut().filter(|p| p.turn == slot) else {
                    return out;
                };
                let Board::Cards { deck, first, pairs } = &mut play.board else {
                    return out;
                };
                let Some(picked) = *first else {
                    return out;
                };
                let (a, b) = (usize::from(picked), usize::from(index));
                if a == b || deck.get(b).is_none_or(|&c| c < 0) {
                    return out;
                }
                let matched = deck[a] == deck[b];
                *first = None;
                if matched {
                    deck[a] = -1;
                    deck[b] = -1;
                    pairs[usize::from(slot)] += 1;
                }
                let done = deck.iter().all(|&c| c < 0);
                let pairs = *pairs;
                if matched {
                    // The finder goes on, with a fresh turn.
                    let turn = play.turn;
                    play.give_turn(turn);
                } else {
                    play.pass_turn();
                }
                game.tell_all(&mut out, || build_card_second(index, picked, slot, matched));
                if !matched {
                    return out;
                }
                if !done {
                    let name = game.member(slot).expect("a member").name.clone();
                    game.tell_all(&mut out, || {
                        build_game_message(game_message::MATCHED, &name)
                    });
                    return out;
                }
                let winner = match pairs[0].cmp(&pairs[1]) {
                    std::cmp::Ordering::Greater => Some(0),
                    std::cmp::Ordering::Less => Some(1),
                    std::cmp::Ordering::Equal => None,
                };
                let result = winner.map_or(game_result::TIE, |_| game_result::WON);
                out.extend(self.finish(id, winner, result));
            }
            _ => {}
        }
        out
    }

    fn start(&mut self, id: i32, rng: &mut impl Rng) -> Outbox {
        let mut out = Outbox::new();
        let Some(game) = self.rooms.get_mut(&id) else {
            return out;
        };
        if game.visitor.is_none() {
            return out;
        }
        let waits = game.waits;
        let board = match game.kind {
            OMOK => Board::Omok(Omok::default()),
            _ => Board::Cards {
                deck: mini_game::deal(game.piece, rng),
                first: None,
                pairs: [0; 2],
            },
        };
        let start = match &board {
            Board::Omok(_) => build_omok_start(waits),
            Board::Cards { deck, .. } => build_cards_start(waits, deck),
        };
        let Ok(start) = start else {
            return out;
        };
        for (_, member) in game.members() {
            out.push((member.client_id, ServerMessage::SendPacket(start.clone())));
            event(&mut out, member.client_id, id, GameEvent::Started);
        }
        game.ready = false;
        game.play = Some(Play {
            board,
            turn: 1 - waits,
            first: 1 - waits,
            tie_asked: None,
            tie_used: false,
            undo_asked: None,
            undone: [false; 2],
            turn_since: Instant::now(),
        });
        self.show_balloon(id);
        out
    }

    /// The game ends, won by `winner` or tied: both records change, and
    /// whoever asked to leave after it does.
    fn finish(&mut self, id: i32, winner: Option<u8>, result: u8) -> Outbox {
        let mut out = Outbox::new();
        let Some(game) = self.rooms.get_mut(&id) else {
            return out;
        };
        if game.play.take().is_none() {
            return out;
        }
        game.ready = false;
        if let Some(winner) = winner {
            game.waits = winner;
        }
        for slot in 0..GAME_USERS {
            let outcome = match winner {
                None => GameOutcome::Tie,
                Some(w) if w == slot => GameOutcome::Win,
                Some(_) => GameOutcome::Loss,
            };
            if let Some(member) = game.member_mut(slot) {
                member.record.add(outcome);
                let to = member.client_id;
                event(&mut out, to, id, GameEvent::Ended(outcome));
            }
        }
        let visitor = game.visitor.as_ref().map(|v| v.record).unwrap_or_default();
        let records = [&game.owner.record, &visitor];
        let kind = game.kind;
        game.tell_all(&mut out, || {
            build_game_result(kind, result, winner.unwrap_or(0), records)
        });
        self.show_balloon(id);
        let game = &self.rooms[&id];
        let leavers: Vec<ClientId> = game
            .members()
            .filter(|(_, m)| m.leave_after)
            .map(|(_, m)| m.client_id)
            .collect();
        for leaver in leavers {
            // An owner's going took the visitor out with them.
            let Some((_, slot)) = self.game_of(leaver) else {
                continue;
            };
            let left = leave_packet(build_room_leave(slot, leave_reason::LEFT));
            event(&mut out, leaver, id, GameEvent::Left(left));
            out.extend(self.leave(leaver));
        }
        out
    }

    fn expel(&mut self, id: i32) -> Outbox {
        let mut out = Outbox::new();
        let Some(game) = self.rooms.get_mut(&id) else {
            return out;
        };
        let Some(visitor) = game.visitor.take() else {
            return out;
        };
        game.ready = false;
        // A new visitor starts afresh, the owner waiting.
        game.waits = 0;
        self.member_of.remove(&visitor.client_id);
        let expelled = leave_packet(build_room_leave(1, leave_reason::EXPELLED));
        event(&mut out, visitor.client_id, id, GameEvent::Left(expelled));
        packet(&mut out, game.owner.client_id, build_seat_left(1));
        self.show_balloon(id);
        out
    }

    /// The player leaves their game, losing it if one is being played;
    /// they count that loss themselves. An owner closes the room.
    pub fn leave(&mut self, client_id: ClientId) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, slot)) = self.game_of(client_id) else {
            return out;
        };
        let game = self.rooms.get_mut(&id).expect("a member's game");
        if game.play.is_some() {
            // The leaver's own result goes nowhere: they may be gone.
            let finished = self.finish(id, Some(1 - slot), game_result::FORFEIT);
            out.extend(finished.into_iter().filter(|(to, _)| *to != client_id));
            if !self.member_of.contains_key(&client_id) {
                // They had asked to leave after the game, and did.
                return out;
            }
        }
        self.member_of.remove(&client_id);
        if slot == 0 {
            let game = self.rooms.remove(&id).expect("a member's game");
            if let Some(visitor) = game.visitor {
                self.member_of.remove(&visitor.client_id);
                let closed = leave_packet(build_room_leave(1, leave_reason::SHOP_CLOSED));
                event(&mut out, visitor.client_id, id, GameEvent::Left(closed));
            }
            self.balloons.push((game.owner.client_id, None));
            return out;
        }
        let game = self.rooms.get_mut(&id).expect("a member's game");
        game.visitor = None;
        game.ready = false;
        game.waits = 0;
        packet(&mut out, game.owner.client_id, build_seat_left(1));
        self.show_balloon(id);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    const ALICE: ClientId = 1;
    const BOB: ClientId = 2;
    const CAROL: ClientId = 3;
    const ROOM: i32 = 77;

    fn look() -> AvatarLook {
        AvatarLook {
            gender: 0,
            skin: 0,
            face: 20000,
            hair: 30000,
            visible: BTreeMap::new(),
            masked: BTreeMap::new(),
            cash_weapon: None,
            pets: [0; 3],
        }
    }

    fn record(wins: i32) -> GameRecord {
        GameRecord {
            wins,
            ties: 0,
            losses: 0,
        }
    }

    /// The mode byte of a packet (with a refusal's code, a game message's
    /// id, and a LEAVE's slot and reason), or what kind of event it is.
    fn summary(out: &Outbox) -> Vec<(ClientId, String)> {
        out.iter()
            .map(|(to, message)| {
                let what = match message {
                    ServerMessage::SendPacket(packet) => describe(packet),
                    ServerMessage::MiniGame { event, .. } => match event {
                        GameEvent::Entered { kind, slot } => format!("entered {kind} {slot}"),
                        GameEvent::Started => "started".to_string(),
                        GameEvent::Ended(outcome) => format!("{outcome:?}"),
                        GameEvent::Left(Some(packet)) => format!("left {}", describe(packet)),
                        GameEvent::Left(None) => "left".to_string(),
                    },
                    other => format!("{other:?}"),
                };
                (*to, what)
            })
            .collect()
    }

    fn describe(packet: &Packet) -> String {
        let bytes = &packet.bytes;
        match bytes[2] {
            0x05 if bytes[3] == 0 => format!("0x5 refused {:#x}", bytes[4]),
            0x06 if bytes[3] == 7 => format!("message {}", bytes[4]),
            0x0a => format!("0xa {:?}", &bytes[3..]),
            mode => format!("{mode:#x}"),
        }
    }

    fn s(to: ClientId, what: &str) -> (ClientId, String) {
        (to, what.to_string())
    }

    fn both(what: &str) -> Vec<(ClientId, String)> {
        vec![s(ALICE, what), s(BOB, what)]
    }

    fn open(kind: u8, password: Option<&str>) -> Games {
        let mut games = Games::default();
        let password = password.map(str::to_string);
        games.create(
            ROOM,
            ALICE,
            "Alice",
            &look(),
            kind,
            "Hi".into(),
            password,
            0,
            record(3),
            false,
        );
        games.take_balloons();
        games
    }

    fn seated(kind: u8) -> Games {
        let mut games = open(kind, None);
        games.visit(
            BOB,
            ROOM,
            "Bob",
            &look(),
            None,
            [record(1), record(2)],
            false,
        );
        games.take_balloons();
        games
    }

    /// Bob is ready and Alice started; Bob, who waits for nobody, moves
    /// first, with black.
    fn omok() -> Games {
        let mut games = seated(OMOK);
        games.play(BOB, MiniGameRequest::Ready(true));
        games.play(ALICE, MiniGameRequest::Start);
        games.take_balloons();
        games
    }

    fn stone(x: i32, y: i32, color: u8) -> MiniGameRequest {
        MiniGameRequest::Move { x, y, color }
    }

    #[test]
    fn an_owner_opens_a_game_with_a_balloon() {
        let mut games = Games::default();
        let out = games.create(
            ROOM,
            ALICE,
            "Alice",
            &look(),
            OMOK,
            "Hi".into(),
            Some("pw".into()),
            4,
            record(3),
            false,
        );
        assert_eq!(
            summary(&out),
            vec![s(ALICE, "entered 1 0"), s(ALICE, "0x5")]
        );
        let balloons = games.take_balloons();
        let balloon = balloons[0].1.as_ref().unwrap();
        assert_eq!(
            (
                balloon.kind,
                balloon.sn,
                balloon.skin,
                balloon.users,
                balloon.max_users
            ),
            (OMOK, ROOM, 4, 1, 2)
        );
        assert!(balloon.private && !balloon.in_progress);
        // Someone busy, or at a game already, can't open one.
        let out = games.create(
            ROOM + 1,
            ALICE,
            "Alice",
            &look(),
            OMOK,
            "x".into(),
            None,
            0,
            record(0),
            false,
        );
        assert_eq!(summary(&out), vec![s(ALICE, "left")]);
        let out = games.create(
            ROOM + 1,
            BOB,
            "Bob",
            &look(),
            OMOK,
            "x".into(),
            None,
            0,
            record(0),
            true,
        );
        assert_eq!(summary(&out), vec![s(BOB, "left")]);
    }

    #[test]
    fn a_visitor_needs_the_password_and_a_free_seat() {
        let mut games = open(MATCH_CARDS_KIND, Some("pw"));
        let records = [record(1), record(2)];
        let out = games.visit(BOB, ROOM, "Bob", &look(), Some("no"), records, false);
        assert_eq!(summary(&out), vec![s(BOB, "0x5 refused 0x16")]);
        let out = games.visit(BOB, ROOM, "Bob", &look(), None, records, false);
        assert_eq!(summary(&out), vec![s(BOB, "0x5 refused 0x16")]);
        let out = games.visit(BOB, ROOM, "Bob", &look(), Some("pw"), records, true);
        assert_eq!(summary(&out), vec![s(BOB, "0x5 refused 0x1")]);
        let out = games.visit(BOB, ROOM, "Bob", &look(), Some("pw"), records, false);
        assert_eq!(
            summary(&out),
            vec![s(BOB, "entered 2 1"), s(BOB, "0x5"), s(ALICE, "0x4")]
        );
        // The match cards record goes with the visitor.
        assert_eq!(games.rooms[&ROOM].visitor.as_ref().unwrap().record.wins, 2);
        assert_eq!(games.take_balloons()[0].1.as_ref().unwrap().users, 2);
        let out = games.visit(CAROL, ROOM, "Carol", &look(), Some("pw"), records, false);
        assert_eq!(summary(&out), vec![s(CAROL, "0x5 refused 0x2")]);
        let out = games.visit(CAROL, ROOM + 1, "Carol", &look(), None, records, false);
        assert_eq!(summary(&out), vec![s(CAROL, "0x5 refused 0x1")]);
    }

    const MATCH_CARDS_KIND: u8 = net::mini_game::MATCH_CARDS;

    #[test]
    fn the_owner_starts_once_the_visitor_is_ready() {
        let mut games = seated(OMOK);
        assert!(summary(&games.play(ALICE, MiniGameRequest::Start)).is_empty());
        // Only the visitor says they are ready.
        assert!(summary(&games.play(ALICE, MiniGameRequest::Ready(true))).is_empty());
        assert_eq!(
            summary(&games.play(BOB, MiniGameRequest::Ready(true))),
            both("0x3a")
        );
        assert_eq!(
            summary(&games.play(BOB, MiniGameRequest::Ready(false))),
            both("0x3b")
        );
        games.play(BOB, MiniGameRequest::Ready(true));
        assert!(summary(&games.play(BOB, MiniGameRequest::Start)).is_empty());
        let out = games.play(ALICE, MiniGameRequest::Start);
        assert_eq!(
            summary(&out),
            vec![
                s(ALICE, "0x3d"),
                s(ALICE, "started"),
                s(BOB, "0x3d"),
                s(BOB, "started")
            ]
        );
        assert!(games.take_balloons()[0].1.as_ref().unwrap().in_progress);
        // Not twice.
        assert!(summary(&games.play(ALICE, MiniGameRequest::Start)).is_empty());
    }

    #[test]
    fn five_in_a_row_wins_and_the_loser_opens_the_next() {
        let mut games = omok();
        // Alice waits, and white is not Bob's.
        assert!(summary(&games.play(ALICE, stone(0, 0, 2))).is_empty());
        assert!(summary(&games.play(BOB, stone(0, 0, 2))).is_empty());
        for x in 0..4 {
            assert_eq!(summary(&games.play(BOB, stone(x, 0, 1))), both("0x40"));
            assert_eq!(summary(&games.play(ALICE, stone(x, 5, 2))), both("0x40"));
        }
        let out = games.play(BOB, stone(4, 0, 1));
        assert_eq!(
            summary(&out),
            vec![
                s(ALICE, "0x40"),
                s(BOB, "0x40"),
                s(ALICE, "Loss"),
                s(BOB, "Win"),
                s(ALICE, "0x3e"),
                s(BOB, "0x3e"),
            ]
        );
        let game = &games.rooms[&ROOM];
        assert_eq!(
            (
                game.owner.record.losses,
                game.visitor.as_ref().unwrap().record.wins
            ),
            (1, 2)
        );
        assert!(!games.take_balloons()[0].1.as_ref().unwrap().in_progress);
        // Bob won, so he waits and Alice opens with black.
        games.play(BOB, MiniGameRequest::Ready(true));
        games.play(ALICE, MiniGameRequest::Start);
        assert!(summary(&games.play(BOB, stone(7, 7, 1))).is_empty());
        assert_eq!(summary(&games.play(ALICE, stone(7, 7, 1))), both("0x40"));
    }

    #[test]
    fn a_refused_stone_keeps_the_turn() {
        let mut games = omok();
        for (x, y, color, who) in [
            (5, 7, 1, BOB),
            (0, 0, 2, ALICE),
            (6, 7, 1, BOB),
            (1, 0, 2, ALICE),
            (7, 5, 1, BOB),
            (2, 0, 2, ALICE),
            (7, 6, 1, BOB),
            (4, 0, 2, ALICE),
        ] {
            games.play(who, stone(x, y, color));
        }
        let out = games.play(BOB, stone(7, 7, 1));
        assert_eq!(summary(&out), vec![s(BOB, "0x41")]);
        assert_eq!(out_bytes(&out)[3], b'C');
        let out = games.play(BOB, stone(0, 0, 1));
        assert_eq!(summary(&out), vec![s(BOB, "0x41")]);
        assert_eq!(summary(&games.play(BOB, stone(9, 9, 1))), both("0x40"));
    }

    fn out_bytes(out: &Outbox) -> Vec<u8> {
        match &out[0].1 {
            ServerMessage::SendPacket(packet) => packet.bytes.clone(),
            other => panic!("not a packet: {other:?}"),
        }
    }

    #[test]
    fn a_tie_needs_the_other_to_agree() {
        let mut games = omok();
        assert!(summary(&games.play(ALICE, MiniGameRequest::AnswerTie(true))).is_empty());
        assert_eq!(
            summary(&games.play(ALICE, MiniGameRequest::AskTie)),
            vec![s(BOB, "0x32")]
        );
        // Once per turn.
        assert!(summary(&games.play(ALICE, MiniGameRequest::AskTie)).is_empty());
        assert!(summary(&games.play(ALICE, MiniGameRequest::AnswerTie(true))).is_empty());
        assert_eq!(
            summary(&games.play(BOB, MiniGameRequest::AnswerTie(false))),
            vec![s(ALICE, "0x33")]
        );
        // Still once per turn, whoever asks; the next turn may.
        assert!(summary(&games.play(BOB, MiniGameRequest::AskTie)).is_empty());
        games.play(BOB, stone(0, 0, 1));
        games.play(BOB, MiniGameRequest::AskTie);
        let out = games.play(ALICE, MiniGameRequest::AnswerTie(true));
        assert_eq!(
            summary(&out),
            vec![
                s(ALICE, "Tie"),
                s(BOB, "Tie"),
                s(ALICE, "0x3e"),
                s(BOB, "0x3e")
            ]
        );
        assert_eq!(games.rooms[&ROOM].owner.record.ties, 1);
    }

    #[test]
    fn a_stone_comes_back_once_the_other_agrees() {
        let mut games = omok();
        // Nothing to take back yet.
        assert!(summary(&games.play(BOB, MiniGameRequest::AskUndo)).is_empty());
        games.play(BOB, stone(0, 0, 1));
        games.play(ALICE, stone(1, 0, 2));
        assert_eq!(
            summary(&games.play(BOB, MiniGameRequest::AskUndo)),
            vec![s(ALICE, "0x36")]
        );
        assert_eq!(
            summary(&games.play(ALICE, MiniGameRequest::AnswerUndo(false))),
            vec![s(BOB, "0x37")]
        );
        games.play(BOB, MiniGameRequest::AskUndo);
        let out = games.play(ALICE, MiniGameRequest::AnswerUndo(true));
        assert_eq!(summary(&out), both("0x37"));
        // Bob's stone and Alice's answer went; Bob moves again.
        assert_eq!(&out_bytes(&out)[3..], &[1, 2, 1]);
        assert_eq!(summary(&games.play(BOB, stone(0, 0, 1))), both("0x40"));
        // Once a game.
        games.play(ALICE, stone(1, 0, 2));
        assert!(summary(&games.play(BOB, MiniGameRequest::AskUndo)).is_empty());
    }

    #[test]
    fn a_turn_skips_only_for_whoever_has_it_once_it_ran_out() {
        let mut games = omok();
        assert!(summary(&games.play(BOB, MiniGameRequest::Skip)).is_empty());
        time_out(&mut games);
        assert!(summary(&games.play(ALICE, MiniGameRequest::Skip)).is_empty());
        let out = games.play(BOB, MiniGameRequest::Skip);
        assert_eq!(summary(&out), both("0x3f"));
        assert_eq!(out_bytes(&out)[3], 0);
        // Alice's own skip, as her time also ran out, finds a new turn.
        assert!(summary(&games.play(ALICE, MiniGameRequest::Skip)).is_empty());
        assert_eq!(summary(&games.play(ALICE, stone(0, 0, 2))), both("0x40"));
    }

    /// Every turn under way has run out of time.
    fn time_out(games: &mut Games) {
        for game in games.rooms.values_mut() {
            if let Some(play) = game.play.as_mut() {
                play.turn_since -= OMOK_SKIP_AFTER;
            }
        }
    }

    #[test]
    fn giving_up_loses() {
        let mut games = omok();
        let out = games.play(ALICE, MiniGameRequest::GiveUp);
        assert_eq!(
            summary(&out),
            vec![
                s(ALICE, "message 2"),
                s(BOB, "message 2"),
                s(ALICE, "Loss"),
                s(BOB, "Win"),
                s(ALICE, "0x3e"),
                s(BOB, "0x3e"),
            ]
        );
    }

    #[test]
    fn leaving_a_game_loses_it_for_the_leaver_alone() {
        let mut games = omok();
        let out = games.leave(BOB);
        assert_eq!(
            summary(&out),
            vec![s(ALICE, "Win"), s(ALICE, "0x3e"), s(ALICE, "0xa [1]")]
        );
        assert!(!games.is_member(BOB));
        assert_eq!(games.rooms[&ROOM].owner.record.wins, 4);

        // The owner leaving closes the room.
        let mut games = omok();
        let out = games.leave(ALICE);
        assert_eq!(
            summary(&out),
            vec![s(BOB, "Win"), s(BOB, "0x3e"), s(BOB, "left 0xa [1, 3]")]
        );
        assert!(!games.contains(ROOM) && !games.is_member(BOB));
        assert_eq!(games.take_balloons().last().unwrap(), &(ALICE, None));
    }

    #[test]
    fn whoever_asked_leaves_after_the_game() {
        let mut games = omok();
        assert_eq!(
            summary(&games.play(BOB, MiniGameRequest::LeaveAfter(true))),
            both("message 5")
        );
        assert_eq!(
            summary(&games.play(BOB, MiniGameRequest::LeaveAfter(false))),
            both("message 6")
        );
        games.play(BOB, MiniGameRequest::LeaveAfter(true));
        let out = games.play(ALICE, MiniGameRequest::GiveUp);
        assert_eq!(
            summary(&out)[6..],
            [s(BOB, "left 0xa [1, 0]"), s(ALICE, "0xa [1]")]
        );
        assert!(games.contains(ROOM) && !games.is_member(BOB));
    }

    #[test]
    fn the_owner_expels_between_games() {
        let mut games = omok();
        assert!(summary(&games.play(ALICE, MiniGameRequest::Expel)).is_empty());
        games.play(ALICE, MiniGameRequest::GiveUp);
        assert!(summary(&games.play(BOB, MiniGameRequest::Expel)).is_empty());
        let out = games.play(ALICE, MiniGameRequest::Expel);
        assert_eq!(
            summary(&out),
            vec![s(BOB, "left 0xa [1, 5]"), s(ALICE, "0xa [1]")]
        );
        assert!(!games.is_member(BOB));
    }

    #[test]
    fn chat_goes_to_both() {
        let mut games = seated(OMOK);
        assert_eq!(summary(&games.chat(BOB, "hi")), both("0x6"));
        assert!(games.chat(CAROL, "hi").is_empty());
    }

    /// Match cards dealt from a seeded deck; Bob turns up first.
    fn cards() -> (Games, Vec<i32>) {
        let mut games = seated(MATCH_CARDS_KIND);
        games.play(BOB, MiniGameRequest::Ready(true));
        games.start(ROOM, &mut StdRng::seed_from_u64(3));
        let Some(Play {
            board: Board::Cards { deck, .. },
            ..
        }) = &games.rooms[&ROOM].play
        else {
            panic!("no cards");
        };
        let deck = deck.clone();
        (games, deck)
    }

    fn flip(first: bool, index: usize) -> MiniGameRequest {
        MiniGameRequest::Flip {
            first,
            index: index as u8,
        }
    }

    /// Where the other card of the face at `index` is.
    fn mate(deck: &[i32], index: usize) -> usize {
        (0..deck.len())
            .find(|&i| i != index && deck[i] == deck[index])
            .unwrap()
    }

    #[test]
    fn a_pair_keeps_the_turn_and_a_miss_passes_it() {
        let (mut games, deck) = cards();
        assert_eq!(deck.len(), 12);
        assert!(summary(&games.play(ALICE, flip(true, 0))).is_empty());
        // The first card shows the other player only.
        assert_eq!(
            summary(&games.play(BOB, flip(true, 0))),
            vec![s(ALICE, "0x44")]
        );
        assert!(summary(&games.play(BOB, flip(false, 0))).is_empty());
        let out = games.play(BOB, flip(false, mate(&deck, 0)));
        assert_eq!(
            summary(&out),
            vec![
                s(ALICE, "0x44"),
                s(BOB, "0x44"),
                s(ALICE, "message 9"),
                s(BOB, "message 9")
            ]
        );
        // Bob again; a miss hands the turn over.
        let miss = (1..deck.len())
            .find(|&i| deck[i] != deck[1] && deck[i] != deck[0])
            .unwrap();
        games.play(BOB, flip(true, 1));
        let out = games.play(BOB, flip(false, miss));
        assert_eq!(summary(&out), both("0x44"));
        assert_eq!(*out_bytes(&out).last().unwrap(), 1);
        // A matched card is gone.
        assert!(summary(&games.play(ALICE, flip(true, 0))).is_empty());
        assert_eq!(
            summary(&games.play(ALICE, flip(true, 1))),
            vec![s(BOB, "0x44")]
        );
    }

    #[test]
    fn a_cards_turn_runs_out_sooner_than_omok() {
        let (mut games, _) = cards();
        for game in games.rooms.values_mut() {
            if let Some(play) = game.play.as_mut() {
                play.turn_since -= CARDS_SKIP_AFTER;
            }
        }
        let out = games.play(BOB, MiniGameRequest::Skip);
        assert_eq!(summary(&out), both("0x3f"));
        assert_eq!(out_bytes(&out)[3], 0);
    }

    #[test]
    fn more_pairs_win_the_cards() {
        let (mut games, deck) = cards();
        let mut done = vec![false; deck.len()];
        let mut last = Outbox::new();
        // Bob takes every pair.
        for i in 0..deck.len() {
            if done[i] {
                continue;
            }
            let j = mate(&deck, i);
            done[i] = true;
            done[j] = true;
            games.play(BOB, flip(true, i));
            last = games.play(BOB, flip(false, j));
        }
        assert_eq!(
            summary(&last),
            vec![
                s(ALICE, "0x44"),
                s(BOB, "0x44"),
                s(ALICE, "Loss"),
                s(BOB, "Win"),
                s(ALICE, "0x3e"),
                s(BOB, "0x3e"),
            ]
        );
    }
}
