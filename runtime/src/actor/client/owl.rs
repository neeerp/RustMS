//! The Owl of Minerva (report c024): searches every channel's Free Market
//! shops for an item, and GO takes the player into one on their channel.

use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldKey, FieldMessage};
use game::InventoryType;
use net::owl::{self, go_result, OwlRequest, ShopHit};
use net::packet::build::world::owl::{build_owl_go_result, build_owl_popular, build_owl_results};
use net::packet::build::world::{inventory, stat};
use net::packet::handle::world::warp_to_spawn_point;
use net::trade::TradeRequest;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::{timeout, timeout_at, Instant};

/// How long the Free Market rooms have to answer a search or GO.
const ASK_TIMEOUT: Duration = Duration::from_secs(1);

impl ClientActor {
    pub(super) async fn owl_request(&mut self, request: OwlRequest) -> Result<(), RuntimeError> {
        match request {
            OwlRequest::Popular => {
                let (reply, popular) = oneshot::channel();
                self.send_to_directory(DirectoryMessage::OwlPopular { reply })
                    .await?;
                let popular = popular.await.unwrap_or_default();
                self.send(&mut build_owl_popular(&popular)?).await
            }
            OwlRequest::Search {
                inventory,
                slot,
                owl_id,
                item_id,
                descending,
            } => {
                self.owl_search(inventory, slot, owl_id, item_id, descending)
                    .await
            }
            OwlRequest::Go { room, map_id } => self.owl_go(room, map_id).await,
        }
    }

    /// Search with the Owl in `slot` of `inventory`, used up only if
    /// something turns up. The answer, even an empty one, lets the client
    /// make requests again.
    async fn owl_search(
        &mut self,
        inventory: InventoryType,
        slot: i16,
        owl_id: i32,
        item_id: i32,
        descending: bool,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let may = owl::owl_in(inventory) == Some(owl_id)
            && player.stats.hp > 0
            && owl::in_free_market(player.stats.map_id)
            && player
                .inventory
                .get(inventory, slot)
                .is_some_and(|item| item.item_id == owl_id)
            && self.trade.is_none()
            && self.personal_shop.is_none();
        if !may {
            return self.send(&mut stat::build_stat_update(true, &[])?).await;
        }
        // Only real items count towards the list every client sees.
        if net::items::info(item_id).is_some() {
            self.send_to_directory(DirectoryMessage::OwlSearched { item_id })
                .await?;
        }
        let hits = owl::rank(self.search_shops(item_id).await, descending);
        let mut packets = vec![build_owl_results(item_id, &hits)?];
        if !hits.is_empty() {
            let player = &mut self.in_game_mut()?.player;
            match player.inventory_mut().take(inventory, slot, 1) {
                Ok(changes) => packets.push(inventory::build_inventory_operation(true, &changes)?),
                Err(error) => tracing::warn!(slot, ?error, "Owl not taken after a search"),
            }
        }
        self.send_all(&mut packets).await
    }

    /// The open shops selling `item_id` in every channel's Free Market
    /// rooms that are running; a room too busy to answer in time is left
    /// out.
    async fn search_shops(&self, item_id: i32) -> Vec<ShopHit> {
        let rooms = self
            .services
            .fields
            .running(|key| key.instance_id == 0 && owl::is_market_room(key.map_id));
        let replies: Vec<_> = rooms
            .into_iter()
            .filter_map(|room| {
                let (reply, hits) = oneshot::channel();
                room.tx
                    .try_send(FieldMessage::ShopSearch { item_id, reply })
                    .ok()
                    .map(|()| hits)
            })
            .collect();
        let deadline = Instant::now() + ASK_TIMEOUT;
        let mut hits = Vec::new();
        for reply in replies {
            if let Ok(Ok(found)) = timeout_at(deadline, reply).await {
                hits.extend(found);
            }
        }
        hits
    }

    /// GO on the results' row of shop `room` on `map_id`: on the player's
    /// channel, they go to its map and into the shop.
    async fn owl_go(&mut self, room: i32, map_id: i32) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let code = if player.stats.hp <= 0 {
            go_result::DEAD
        } else if self.trade.is_some() || self.personal_shop.is_some() {
            go_result::BUSY
        } else if !owl::in_free_market(player.stats.map_id) || !owl::is_market_room(map_id) {
            go_result::CLOSED
        } else {
            let name = player.name().to_string();
            self.shop_go_result(room, map_id, name).await
        };
        self.send(&mut build_owl_go_result(code)?).await?;
        if code != go_result::GO {
            return Ok(());
        }
        let channel_id = self.channel_id;
        let player = &mut self.in_game_mut()?.player;
        if player.stats.map_id != map_id {
            for action in warp_to_spawn_point(player, map_id, 0, channel_id)? {
                Box::pin(self.apply(action)).await?;
            }
        }
        self.trade_request(TradeRequest::Visit { sn: room }).await
    }

    /// What shop `room` on this channel's `map_id` says to `name` coming in.
    async fn shop_go_result(&self, room: i32, map_id: i32, name: String) -> u8 {
        let key = FieldKey::new(self.channel_id, map_id);
        let Some(field) = self.services.fields.running(|k| *k == key).pop() else {
            return go_result::CLOSED;
        };
        let (reply, code) = oneshot::channel();
        if field
            .tx
            .try_send(FieldMessage::ShopGo { room, name, reply })
            .is_err()
        {
            return go_result::BUSY;
        }
        match timeout(ASK_TIMEOUT, code).await {
            Ok(Ok(code)) => code,
            _ => go_result::BUSY,
        }
    }
}
