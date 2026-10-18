use super::trade::NEXT_ROOM;
use super::ClientActor;
use crate::actor::field_shops::{Sale, ShopEvent, ShopRoomRequest};
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::{InventoryType, Player};
use net::packet::build::world::mini_room::{
    build_buy_result, build_enter_error, build_room_leave, build_shop_sold, buy_result,
    enter_error, leave_reason,
};
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{inventory, stat};
use net::personal_shop::{self as rules, Listing, PersonalShopRequest};
use net::trade::{self, TradeRequest};
use packet::Packet;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{debug, info};

/// How long a buyer, already charged, waits for the owner to hand over the
/// items before taking the mesos back.
const SALE_TIMEOUT: Duration = Duration::from_secs(5);

/// The player's place in a personal shop.
pub(super) enum PersonalShop {
    /// The player's own shop. The stock is out of their inventory until it
    /// sells or the shop closes.
    Owner {
        room: i32,
        listings: Vec<Listing>,
        open: bool,
    },
    Visitor {
        room: i32,
        slot: u8,
    },
}

impl PersonalShop {
    fn room(&self) -> i32 {
        match self {
            Self::Owner { room, .. } | Self::Visitor { room, .. } => *room,
        }
    }

    /// Put the unsold stock back into a copy of the owner, so a save while
    /// the shop is open keeps it.
    pub(super) fn add_back(&self, snapshot: &mut Player) {
        if let Self::Owner { listings, .. } = self {
            trade::give(snapshot, rules::unsold(listings));
        }
    }
}

impl ClientActor {
    pub(super) async fn personal_shop_request(
        &mut self,
        request: PersonalShopRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            PersonalShopRequest::Create {
                title,
                permit_slot,
                permit_id,
            } => self.create_shop(title, permit_slot, permit_id).await,
            PersonalShopRequest::PutItem {
                kind,
                slot,
                bundles,
                per_bundle,
                price,
            } => self.list_item(kind, slot, bundles, per_bundle, price).await,
            PersonalShopRequest::TakeBack { index } => self.take_back(index).await,
            PersonalShopRequest::Open => {
                let Some(PersonalShop::Owner { listings, open, .. }) = &mut self.personal_shop
                else {
                    return Ok(());
                };
                if *open || listings.is_empty() {
                    return Ok(());
                }
                *open = true;
                self.to_shop(ShopRoomRequest::Open).await
            }
            PersonalShopRequest::Buy { index, bundles } => self.buy(index, bundles).await,
            PersonalShopRequest::Ban {
                slot,
                name,
                overstayed,
            } => {
                self.owner_to_shop(ShopRoomRequest::Ban {
                    slot,
                    name,
                    overstayed,
                })
                .await
            }
            PersonalShopRequest::Blacklist(names) => {
                self.owner_to_shop(ShopRoomRequest::Blacklist(names)).await
            }
            PersonalShopRequest::BlacklistAdd(name) => {
                self.owner_to_shop(ShopRoomRequest::BlacklistAdd(name))
                    .await
            }
            PersonalShopRequest::BlacklistRemove(name) => {
                self.owner_to_shop(ShopRoomRequest::BlacklistRemove(name))
                    .await
            }
        }
    }

    /// Chatting and leaving, shared with the trading room, from a player in
    /// a shop.
    pub(super) async fn shop_room_request(
        &mut self,
        request: TradeRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            TradeRequest::Chat { text } => self.to_shop(ShopRoomRequest::Chat { text }).await,
            TradeRequest::Leave => self.leave_personal_shop().await,
            _ => Ok(()),
        }
    }

    /// Open a shop with the store permit in the cash bag, on a Free Market
    /// map. The permit stays; the shop is empty and closed to visitors
    /// until the owner lists items and opens it.
    async fn create_shop(
        &mut self,
        title: String,
        permit_slot: i16,
        permit_id: i32,
    ) -> Result<(), RuntimeError> {
        if self.trade.is_some() || self.personal_shop.is_some() {
            return Ok(());
        }
        let player = &self.in_game()?.player;
        let has_permit = player
            .inventory
            .get(InventoryType::Cash, permit_slot)
            .is_some_and(|item| item.item_id == permit_id && rules::is_permit(permit_id));
        if !has_permit {
            return Ok(());
        }
        if !rules::allowed_on(player.stats.map_id) {
            return self
                .send(&mut build_enter_error(enter_error::NOT_FREE_MARKET)?)
                .await;
        }
        let room = NEXT_ROOM.fetch_add(1, Ordering::Relaxed);
        self.personal_shop = Some(PersonalShop::Owner {
            room,
            listings: Vec::new(),
            open: false,
        });
        self.to_shop(ShopRoomRequest::Create {
            room,
            title,
            skin: rules::permit_skin(permit_id),
        })
        .await
    }

    /// Move an item from the inventory into the shop, before it opens.
    async fn list_item(
        &mut self,
        kind: i16,
        slot: i16,
        bundles: i16,
        per_bundle: i16,
        price: i32,
    ) -> Result<(), RuntimeError> {
        let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
        let Some(PersonalShop::Owner {
            listings,
            open: false,
            ..
        }) = &mut self.personal_shop
        else {
            return self.unlock().await;
        };
        match rules::list_item(player, listings, kind, slot, bundles, per_bundle, price) {
            Ok((listing, changes)) => {
                listings.push(listing);
                let listings = listings.clone();
                self.send(&mut inventory::build_inventory_operation(true, &changes)?)
                    .await?;
                self.to_shop(ShopRoomRequest::Listings(listings)).await
            }
            Err(why) => {
                debug!(
                    self.client_id,
                    kind, slot, bundles, per_bundle, price, why, "Shop listing refused"
                );
                self.unlock().await
            }
        }
    }

    /// Take a listing back into the inventory, before the shop opens.
    async fn take_back(&mut self, index: i16) -> Result<(), RuntimeError> {
        let player = &self.game.as_ref().ok_or(RuntimeError::NotLoggedIn)?.player;
        let Some(PersonalShop::Owner {
            listings,
            open: false,
            ..
        }) = &mut self.personal_shop
        else {
            return self.unlock().await;
        };
        let Some(index) = usize::try_from(index)
            .ok()
            .filter(|&i| listings.get(i).is_some_and(|l| l.bundles > 0))
        else {
            return self.unlock().await;
        };
        let items = rules::unsold(&listings[index..=index]);
        if trade::check(player, &items).is_err() {
            self.send(&mut inventory::build_inventory_full()?).await?;
            return self.unlock().await;
        }
        listings.remove(index);
        let listings = listings.clone();
        let changes = self.receive(items).await?;
        self.send(&mut inventory::build_inventory_operation(true, &changes)?)
            .await?;
        self.to_shop(ShopRoomRequest::Listings(listings)).await
    }

    /// Buy from the shop the player visits: charge them, ask the owner for
    /// the items, and refund them if the owner can't hand them over.
    async fn buy(&mut self, index: u8, bundles: i16) -> Result<(), RuntimeError> {
        if !matches!(self.personal_shop, Some(PersonalShop::Visitor { .. })) {
            return self.unlock().await;
        }
        let (reply, quote) = oneshot::channel();
        self.to_shop(ShopRoomRequest::Quote { index, reply })
            .await?;
        let Ok(Some(listing)) = quote.await else {
            return self.refuse_purchase(buy_result::NO_STOCK).await;
        };
        let player = &mut self.in_game_mut()?.player;
        let paid = match rules::check_purchase(player, &listing, bundles) {
            Ok(paid) => paid,
            Err(result) => return self.refuse_purchase(result).await,
        };
        player.meso -= paid;
        player.mark_dirty();
        let (reply, sold) = oneshot::channel();
        let sale = Sale {
            index,
            bundles,
            item_id: listing.item.item_id,
            paid,
            buyer: player.stats.name.clone(),
            reply,
        };
        // Whatever happens to the request, the answer says whether to
        // refund.
        let sent = self.to_shop(ShopRoomRequest::Purchase(sale)).await;
        let answer = tokio::time::timeout(SALE_TIMEOUT, sold).await;
        let result = match answer {
            Ok(Ok(Ok(item))) => {
                let changes = self.receive(rules::goods(vec![item])).await?;
                let meso = self.in_game()?.player.meso;
                self.send(&mut inventory::build_inventory_operation(false, &changes)?)
                    .await?;
                self.send(&mut stat::build_stat_update(
                    true,
                    &[StatUpdate::Meso(meso)],
                )?)
                .await
            }
            Ok(Ok(Err(result))) => self.refund(paid, result).await,
            // The shop or its owner is gone, or too slow.
            Ok(Err(_)) | Err(_) => self.refund(paid, buy_result::NO_STOCK).await,
        };
        sent.and(result)
    }

    async fn refund(&mut self, paid: i32, result: u8) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        player.meso += paid;
        player.mark_dirty();
        self.refuse_purchase(result).await
    }

    async fn refuse_purchase(&mut self, result: u8) -> Result<(), RuntimeError> {
        self.send(&mut build_buy_result(result)?).await?;
        self.unlock().await
    }

    /// News from the player's shop.
    pub(super) async fn shop_event(
        &mut self,
        room: i32,
        event: ShopEvent,
    ) -> Result<(), RuntimeError> {
        if let ShopEvent::Entered { slot } = event {
            if self.personal_shop.is_none() && self.trade.is_none() {
                self.personal_shop = Some(PersonalShop::Visitor { room, slot });
            }
            return Ok(());
        }
        if self.personal_shop.as_ref().map(PersonalShop::room) != Some(room) {
            // A sale dropped here refunds the buyer.
            return Ok(());
        }
        match event {
            ShopEvent::Entered { .. } => Ok(()),
            ShopEvent::Sale(sale) => self.sell(sale).await,
            ShopEvent::Left(packet) => self.close_shop(packet).await,
        }
    }

    /// The owner sells to a visitor who has paid, if the stock is still
    /// there and the buyer still waiting.
    async fn sell(&mut self, sale: Sale) -> Result<(), RuntimeError> {
        let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
        let Some(PersonalShop::Owner { listings, .. }) = &mut self.personal_shop else {
            return Ok(());
        };
        let index = usize::from(sale.index);
        if let Err(result) = rules::check_sale(
            player,
            listings,
            index,
            sale.item_id,
            sale.bundles,
            sale.paid,
        ) {
            let _ = sale.reply.send(Err(result));
            return Ok(());
        }
        if sale
            .reply
            .send(Ok(listings[index].take(sale.bundles)))
            .is_err()
        {
            // The buyer gave up and took their mesos back.
            return Ok(());
        }
        rules::sell(player, listings, index, sale.bundles, sale.paid);
        let meso = player.meso;
        let listings = listings.clone();
        info!(
            self.client_id,
            buyer = sale.buyer,
            item_id = sale.item_id,
            bundles = sale.bundles,
            paid = sale.paid,
            "Shop sale"
        );
        self.send(&mut build_shop_sold(sale.index, sale.bundles, &sale.buyer)?)
            .await?;
        self.send(&mut stat::build_stat_update(
            false,
            &[StatUpdate::Meso(meso)],
        )?)
        .await?;
        if rules::sold_out(&listings) {
            // Nothing is left to give back; the room closes the shop for
            // everyone.
            self.personal_shop = None;
            return self.to_shop(ShopRoomRequest::SoldOut).await;
        }
        self.to_shop(ShopRoomRequest::Listings(listings)).await
    }

    /// Leave the shop, if the player is in one. An owner closes it and gets
    /// the unsold stock back. Every way off the field comes through here
    /// first.
    pub(super) async fn leave_personal_shop(&mut self) -> Result<(), RuntimeError> {
        if self.personal_shop.is_none() {
            return Ok(());
        }
        let left = self.to_shop(ShopRoomRequest::Leave).await;
        let closed = self.close_shop(None).await;
        left.and(closed)
    }

    /// The player's field died with their shop.
    pub(super) async fn abandon_personal_shop(&mut self) -> Result<(), RuntimeError> {
        let Some(shop) = &self.personal_shop else {
            return Ok(());
        };
        let slot = match shop {
            PersonalShop::Owner { .. } => 0,
            PersonalShop::Visitor { slot, .. } => *slot,
        };
        let leave = build_room_leave(slot, leave_reason::SHOP_CLOSED)?;
        self.close_shop(Some(leave)).await
    }

    /// The player is out of the shop: an owner gets the unsold stock back,
    /// and whatever no longer fits drops at their feet.
    async fn close_shop(&mut self, packet: Option<Packet>) -> Result<(), RuntimeError> {
        let Some(shop) = self.personal_shop.take() else {
            return Ok(());
        };
        info!(self.client_id, room = shop.room(), "Left personal shop");
        if let PersonalShop::Owner { listings, .. } = shop {
            let changes = self.receive(rules::unsold(&listings)).await?;
            if !changes.is_empty() {
                self.send(&mut inventory::build_inventory_operation(false, &changes)?)
                    .await?;
            }
        }
        if let Some(mut packet) = packet {
            self.send(&mut packet).await?;
        }
        Ok(())
    }

    async fn owner_to_shop(&mut self, request: ShopRoomRequest) -> Result<(), RuntimeError> {
        if !matches!(self.personal_shop, Some(PersonalShop::Owner { .. })) {
            return Ok(());
        }
        self.to_shop(request).await
    }

    async fn to_shop(&mut self, request: ShopRoomRequest) -> Result<(), RuntimeError> {
        self.send_to_field(FieldMessage::PersonalShop {
            from: self.client_id,
            request,
        })
        .await
    }
}
