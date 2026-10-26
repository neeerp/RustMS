//! The player's side of Omok and match cards. The game itself is the
//! field's (`field_games`); the player keeps their records, which the
//! games they sit down to show, and counts each game they finish.

use super::trade::NEXT_ROOM;
use super::ClientActor;
use crate::actor::field_games::{GameEvent, GameRoomRequest};
use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::{GameOutcome, GameRecord};
use net::mini_game::{self, MiniGameRequest};
use net::packet::build::world::mini_room::{build_room_leave, leave_reason};
use net::trade::TradeRequest;
use packet::Packet;
use std::sync::atomic::Ordering;
use tracing::{info, warn};

/// The game the player sits at.
pub(super) struct MiniGame {
    room: i32,
    kind: u8,
    slot: u8,
    /// A game is under way: leaving loses it.
    playing: bool,
}

impl ClientActor {
    pub(super) async fn mini_game_request(
        &mut self,
        request: MiniGameRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            MiniGameRequest::Create {
                kind,
                title,
                password,
                piece,
            } => self.create_game(kind, title, password, piece).await,
            request if self.mini_game.is_some() => {
                self.to_game(GameRoomRequest::Play(request)).await
            }
            _ => Ok(()),
        }
    }

    /// Chatting and leaving, shared with the trading room, from a player at
    /// a game.
    pub(super) async fn game_room_request(
        &mut self,
        request: TradeRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            TradeRequest::Chat { text } => self.to_game(GameRoomRequest::Chat { text }).await,
            TradeRequest::Leave => self.leave_mini_game().await,
            _ => Ok(()),
        }
    }

    /// Open a game with a game set from the Etc bag, which stays, where
    /// the map allows it.
    async fn create_game(
        &mut self,
        kind: u8,
        title: String,
        password: Option<String>,
        piece: u8,
    ) -> Result<(), RuntimeError> {
        if self.trade.is_some()
            || self.personal_shop.is_some()
            || self.hired_merchant.is_some()
            || self.merchant_field.is_some()
            || self.mini_game.is_some()
        {
            return Ok(());
        }
        let Some(set) = mini_game::game_set(kind, piece) else {
            return Ok(());
        };
        let player = &self.in_game()?.player;
        if player.inventory.count(set) <= 0 || title.is_empty() {
            return Ok(());
        }
        // The client refuses here first (fieldLimit 0x80).
        if !mini_game::may_play_here(player.stats.map_id) {
            return Ok(());
        }
        let record = self.game_records().await[usize::from(kind - 1)];
        let room = NEXT_ROOM.fetch_add(1, Ordering::Relaxed);
        self.mini_game = Some(MiniGame {
            room,
            kind,
            slot: 0,
            playing: false,
        });
        self.to_game(GameRoomRequest::Create {
            room,
            kind,
            title,
            password,
            piece,
            record,
        })
        .await
    }

    /// The player's records at Omok and match cards, read once. Every
    /// visit asks, so a failed read shows none rather than ending the
    /// session, and is tried again next time.
    pub(super) async fn game_records(&mut self) -> [GameRecord; 2] {
        if let Some(records) = self.game_records {
            return records;
        }
        let cid = self.client_id;
        match spawn_db(move || db::mini_game::records(cid)).await {
            Ok(records) => {
                self.game_records = Some(records);
                records
            }
            Err(error) => {
                warn!(self.client_id, %error, "Failed to read the mini game records");
                Default::default()
            }
        }
    }

    /// Count a game of `kind` the player finished.
    async fn record_game(&mut self, kind: u8, outcome: GameOutcome) {
        let mut records = self.game_records().await;
        records[usize::from(kind - 1)].add(outcome);
        self.game_records = Some(records);
        let cid = self.client_id;
        if let Err(error) = spawn_db(move || db::mini_game::add(cid, kind, outcome)).await {
            warn!(self.client_id, %error, ?outcome, "Failed to count a mini game");
        }
    }

    /// News from the player's game.
    pub(super) async fn game_event(
        &mut self,
        room: i32,
        event: GameEvent,
    ) -> Result<(), RuntimeError> {
        if let GameEvent::Entered { kind, slot } = event {
            let free = self.trade.is_none()
                && self.personal_shop.is_none()
                && self.hired_merchant.is_none();
            if self.mini_game.is_none() && free {
                self.mini_game = Some(MiniGame {
                    room,
                    kind,
                    slot,
                    playing: false,
                });
            }
            return Ok(());
        }
        let Some(game) = self.mini_game.as_mut().filter(|g| g.room == room) else {
            return Ok(());
        };
        match event {
            GameEvent::Entered { .. } => Ok(()),
            GameEvent::Started => {
                game.playing = true;
                Ok(())
            }
            GameEvent::Ended(outcome) => {
                game.playing = false;
                let kind = game.kind;
                info!(self.client_id, room, ?outcome, "Mini game ended");
                self.record_game(kind, outcome).await;
                Ok(())
            }
            GameEvent::Left(packet) => self.close_game(packet).await,
        }
    }

    async fn close_game(&mut self, packet: Option<Packet>) -> Result<(), RuntimeError> {
        let Some(game) = self.mini_game.take() else {
            return Ok(());
        };
        info!(self.client_id, room = game.room, "Left mini game");
        if let Some(mut packet) = packet {
            self.send(&mut packet).await?;
        }
        Ok(())
    }

    /// Leave the game, if the player is at one, losing a game under way.
    /// An owner closes it. Every way off the field comes through here
    /// first.
    pub(super) async fn leave_mini_game(&mut self) -> Result<(), RuntimeError> {
        let Some(game) = self.mini_game.take() else {
            return Ok(());
        };
        info!(self.client_id, room = game.room, "Left mini game");
        let left = self.to_game(GameRoomRequest::Leave).await;
        // The field counts nothing for the leaver, who may be gone by the
        // time it ends the game. (A game ending as the player leaves may be
        // counted a loss.)
        if game.playing {
            self.record_game(game.kind, GameOutcome::Loss).await;
        }
        left
    }

    /// The player's field died with their game, which counts for nobody.
    pub(super) async fn abandon_mini_game(&mut self) -> Result<(), RuntimeError> {
        let Some(slot) = self.mini_game.as_ref().map(|g| g.slot) else {
            return Ok(());
        };
        let leave = build_room_leave(slot, leave_reason::SHOP_CLOSED)?;
        self.close_game(Some(leave)).await
    }

    async fn to_game(&mut self, request: GameRoomRequest) -> Result<(), RuntimeError> {
        self.send_to_field(FieldMessage::MiniGame {
            from: self.client_id,
            request,
        })
        .await
    }
}
