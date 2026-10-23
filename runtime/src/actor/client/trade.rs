use super::ClientActor;
use crate::actor::field_drops::Ownership;
use crate::actor::field_trades::{RoomRequest, TradeEvent};
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::{InventoryChange, Player};
use net::packet::build::world::mini_room::{
    build_room_leave, build_trade_meso_limit, leave_reason,
};
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{inventory, stat};
use net::trade::{self, Offer, TradeRequest};
use packet::Packet;
use std::sync::atomic::{AtomicI32, Ordering};
use tokio::sync::oneshot;
use tracing::{debug, error, info};

/// Trading room and personal shop ids, unique across fields so a stale
/// invitation can't reach a newer room.
pub(super) static NEXT_ROOM: AtomicI32 = AtomicI32::new(1);

/// The player's side of a trade: what they put up, held out of their
/// inventory until the trade ends. Every way it ends goes through
/// `finish_trade`, which gives this back or, after a trade, hands over the
/// partner's offer instead.
pub(super) struct Trade {
    room: i32,
    slot: u8,
    /// The partner is in.
    started: bool,
    /// The player pressed Trade.
    confirmed: bool,
    offer: Offer,
    /// The partner's offer, after the fee, and the room's decision on it.
    pending: Option<(Offer, oneshot::Receiver<bool>)>,
}

impl Trade {
    fn new(room: i32, slot: u8) -> Self {
        Self {
            room,
            slot,
            started: false,
            confirmed: false,
            offer: Offer::default(),
            pending: None,
        }
    }

    /// Put what the player has up back into a copy of them, so a save
    /// mid-trade keeps it.
    pub(super) fn add_back(&self, snapshot: &mut Player) {
        trade::give(snapshot, self.offer.clone());
    }
}

impl ClientActor {
    pub(super) async fn trade_request(
        &mut self,
        request: TradeRequest,
    ) -> Result<(), RuntimeError> {
        let trading = self.trade.as_ref();
        match request {
            TradeRequest::Create => {
                if trading.is_some() {
                    return Ok(());
                }
                let room = NEXT_ROOM.fetch_add(1, Ordering::Relaxed);
                self.trade = Some(Trade::new(room, 0));
                self.to_room(RoomRequest::Create { room }).await
            }
            TradeRequest::Invite { target } => {
                if !trading.is_some_and(|t| t.slot == 0 && !t.started) {
                    return Ok(());
                }
                self.to_room(RoomRequest::Invite { target }).await
            }
            TradeRequest::Decline { sn, reason } => {
                self.to_room(RoomRequest::Decline { room: sn, reason })
                    .await
            }
            // The room may be a trading room, a personal shop or a game: it
            // says which by starting a trade or seating the player.
            TradeRequest::Visit { sn, password } => {
                if trading.is_some()
                    || self.personal_shop.is_some()
                    || self.merchant_field.is_some()
                    || self.mini_game.is_some()
                {
                    return Ok(());
                }
                if let Some(owner) = crate::actor::field_merchants::owner_of(sn) {
                    if !self.merchant_visits.contains(&owner) {
                        self.merchant_visits.push(owner);
                    }
                }
                let records = self.game_records().await;
                self.to_room(RoomRequest::Visit {
                    room: sn,
                    password,
                    records,
                })
                .await
            }
            TradeRequest::Chat { text } => {
                if trading.is_none() {
                    return Ok(());
                }
                self.to_room(RoomRequest::Chat { text }).await
            }
            TradeRequest::PutItem {
                kind,
                slot,
                count,
                trade_slot,
            } => self.put_up_item(kind, slot, count, trade_slot).await,
            TradeRequest::Meso(amount) => self.put_up_meso(amount).await,
            TradeRequest::Confirm => {
                let Some(trade) = self.trade.as_mut().filter(|t| t.started && !t.confirmed) else {
                    return Ok(());
                };
                trade.confirmed = true;
                self.to_room(RoomRequest::Confirm).await
            }
            TradeRequest::Leave => self.leave_trade().await,
        }
    }

    /// Move an item from the inventory into the offer; the room shows it to
    /// both sides.
    async fn put_up_item(
        &mut self,
        kind: i16,
        slot: i16,
        count: i16,
        trade_slot: u8,
    ) -> Result<(), RuntimeError> {
        let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
        let Some(trade) = self.trade.as_mut().filter(|t| t.started && !t.confirmed) else {
            return self.unlock().await;
        };
        match trade::put_item(player, &mut trade.offer, kind, slot, count, trade_slot) {
            Ok((item, changes)) => {
                self.send(&mut inventory::build_inventory_operation(true, &changes)?)
                    .await?;
                self.to_room(RoomRequest::PutItem {
                    slot: trade_slot,
                    item,
                })
                .await
            }
            Err(why) => {
                debug!(
                    self.client_id,
                    kind, slot, count, trade_slot, why, "Trade item refused"
                );
                self.unlock().await
            }
        }
    }

    /// Move mesos into the offer. Up to level 15 a player may put up at most
    /// a million.
    async fn put_up_meso(&mut self, amount: i32) -> Result<(), RuntimeError> {
        let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
        let Some(trade) = self.trade.as_mut().filter(|t| t.started && !t.confirmed) else {
            return self.unlock().await;
        };
        if trade::over_meso_limit(player.stats.level, trade.offer.meso, amount) {
            self.send(&mut build_trade_meso_limit()?).await?;
            return self.unlock().await;
        }
        if let Err(why) = trade::put_meso(player, &mut trade.offer, amount) {
            debug!(self.client_id, amount, why, "Trade mesos refused");
            return self.unlock().await;
        }
        let (meso, total) = (player.meso, trade.offer.meso);
        self.send(&mut stat::build_stat_update(
            true,
            &[StatUpdate::Meso(meso)],
        )?)
        .await?;
        self.to_room(RoomRequest::Meso { total }).await
    }

    /// The client waits for an answer before it lets the player act again.
    pub(super) async fn unlock(&mut self) -> Result<(), RuntimeError> {
        self.send(&mut stat::build_stat_update(true, &[])?).await
    }

    /// News from the player's trading room.
    pub(super) async fn trade_event(
        &mut self,
        room: i32,
        event: TradeEvent,
    ) -> Result<(), RuntimeError> {
        // A visitor's trade starts once the room lets them in.
        if self.trade.is_none() && matches!(event, TradeEvent::Started) {
            self.trade = Some(Trade::new(room, 1));
        }
        let Some(trade) = self.trade.as_mut().filter(|t| t.room == room) else {
            return Ok(());
        };
        match event {
            TradeEvent::Started => trade.started = true,
            TradeEvent::Check(incoming) => {
                if trade.pending.is_some() {
                    return Ok(());
                }
                let incoming = trade::received(&incoming);
                let result = trade::check(&self.in_game()?.player, &incoming);
                let (decision, answer) = oneshot::channel();
                let trade = self.trade.as_mut().expect("checked above");
                trade.pending = Some((incoming, answer));
                self.to_room(RoomRequest::Answer { result, decision })
                    .await?;
            }
            TradeEvent::Ended(packet) => self.finish_trade(packet).await?,
        }
        Ok(())
    }

    /// Leave the trading room, if the player is in one, and finish the
    /// trade. Every way off the field comes through here first.
    pub(super) async fn leave_trade(&mut self) -> Result<(), RuntimeError> {
        if self.trade.is_none() {
            return Ok(());
        }
        let left = self.to_room(RoomRequest::Leave).await;
        let finished = self.finish_trade(None).await;
        left.and(finished)
    }

    /// The player's field died with their trading room: the trade failed.
    pub(super) async fn abandon_trade(&mut self) -> Result<(), RuntimeError> {
        let Some(trade) = self.trade.as_ref() else {
            return Ok(());
        };
        let leave = build_room_leave(trade.slot, leave_reason::FAILED)?;
        self.finish_trade(Some(leave)).await
    }

    /// End the trade: the player gets the partner's offer if the room
    /// decided the trade goes through, else their own back. Whatever no
    /// longer fits drops at their feet. The meso update goes before `packet`:
    /// the client words its success message by how its mesos changed.
    pub(super) async fn finish_trade(
        &mut self,
        packet: Option<Packet>,
    ) -> Result<(), RuntimeError> {
        let Some(trade) = self.trade.take() else {
            return Ok(());
        };
        let traded = match trade.pending {
            // The room resolves the decision before it says the trade ended;
            // if the room is gone, so is the decision.
            Some((incoming, decision)) => decision.await.unwrap_or(false).then_some(incoming),
            None => None,
        };
        let done = traded.is_some();
        let back = traded.unwrap_or(trade.offer);
        let meso = back.meso;
        let changes = self.receive(back).await?;
        let player_meso = self.in_game()?.player.meso;
        info!(self.client_id, room = trade.room, done, "Trade ended");

        if !changes.is_empty() {
            self.send(&mut inventory::build_inventory_operation(false, &changes)?)
                .await?;
        }
        if meso > 0 {
            self.send(&mut stat::build_stat_update(
                false,
                &[StatUpdate::Meso(player_meso)],
            )?)
            .await?;
        }
        if let Some(mut packet) = packet {
            self.send(&mut packet).await?;
        }
        Ok(())
    }

    /// Give the player `offer`; whatever no longer fits drops at their
    /// feet. Returns the inventory changes, for the caller to send.
    pub(super) async fn receive(
        &mut self,
        offer: Offer,
    ) -> Result<Vec<InventoryChange>, RuntimeError> {
        let game = self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?;
        let given = trade::give(&mut game.player, offer);
        if !given.overflow.is_empty() {
            let (x, y) = game.position;
            let spill = FieldMessage::SpawnLoot {
                owner: self.client_id,
                ownership: Ownership::Owner,
                loot: given.overflow,
                x,
                y,
                dropper: 0,
            };
            if game.field.tx.send(spill).await.is_err() {
                error!(self.client_id, "Field gone; overflow lost");
            }
        }
        Ok(given.changes)
    }

    async fn to_room(&mut self, request: RoomRequest) -> Result<(), RuntimeError> {
        self.send_to_field(FieldMessage::Trade {
            from: self.client_id,
            request,
        })
        .await
    }
}
