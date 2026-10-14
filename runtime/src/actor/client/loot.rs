use super::ClientActor;
use crate::actor::field_drops::Drop;
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::monster_book::{self, MAX_CARD_LEVEL};
use net::loot::{self, Loot};
use net::packet::build::world::effect::{self, CharEffect};
use net::packet::build::world::stat::{self, StatUpdate};
use net::packet::build::world::{drop, inventory, monster_book as book};
use net::FieldRequest;

impl ClientActor {
    /// Roll this player's drops from a mob they helped kill and put them on
    /// the ground where it died.
    pub(super) async fn drop_loot(
        &mut self,
        mob_id: i32,
        object_id: i32,
        x: i16,
        y: i16,
    ) -> Result<(), RuntimeError> {
        let loot = loot::roll(mob_id, &self.in_game()?.player, &mut rand::thread_rng());
        if loot.is_empty() {
            return Ok(());
        }
        self.send_to_field(FieldMessage::SpawnLoot {
            owner: self.client_id,
            loot,
            x,
            y,
            dropper: object_id,
        })
        .await
    }

    /// The field handed over a drop: put it in the inventory, or give it back
    /// if it doesn't fit.
    pub(super) async fn picked_up(&mut self, drop: Drop) -> Result<(), RuntimeError> {
        if let Loot::Item { item_id, .. } = drop.loot {
            if monster_book::is_card(item_id) {
                return self.collect_card(item_id).await;
            }
        }
        let mut packets = Vec::new();
        let player = &mut self.in_game_mut()?.player;
        let (given, item_id, quantity) = match &drop.loot {
            Loot::Mesos(amount) => {
                player.meso = player.meso.saturating_add(*amount);
                player.mark_dirty();
                let meso = player.meso;
                packets.push(stat::build_stat_update(true, &[StatUpdate::Meso(meso)])?);
                packets.push(drop::build_meso_gain(*amount)?);
                return self.send_all(&mut packets).await;
            }
            Loot::Item {
                item_id, quantity, ..
            } => (
                net::items::give(&mut player.inventory, *item_id, *quantity),
                *item_id,
                *quantity,
            ),
            Loot::Dropped(item) => (
                player
                    .inventory
                    .add(item.clone(), net::items::slot_max(item.item_id)),
                item.item_id,
                item.quantity,
            ),
        };
        match given {
            Ok(changes) => {
                player.mark_dirty();
                packets.push(inventory::build_inventory_operation(true, &changes)?);
                packets.push(inventory::build_item_gain(item_id, i32::from(quantity))?);
            }
            Err(_) => {
                // "You can't get anymore items." and back on the ground.
                packets.push(inventory::build_inventory_full()?);
                packets.push(stat::build_stat_update(true, &[])?);
                self.send_all(&mut packets).await?;
                return self.send_to_field(FieldMessage::ReturnDrop { drop }).await;
            }
        }
        self.send_all(&mut packets).await
    }

    /// A picked-up card goes straight into the Monster Book; once its page
    /// holds five, more copies are wasted.
    async fn collect_card(&mut self, card: i32) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let character_id = player.id();
        let level = player.monster_book_mut().add(card);
        let mut packets = vec![
            book::build_set_card(level.is_none(), card, level.unwrap_or(MAX_CARD_LEVEL))?,
            // Unlock the client's pickup.
            stat::build_stat_update(true, &[])?,
        ];
        if level.is_none() {
            return self.send_all(&mut packets).await;
        }
        packets.push(effect::build_own_effect(CharEffect::MonsterCard)?);
        self.send_all(&mut packets).await?;
        let shown = effect::build_foreign_effect(character_id, CharEffect::MonsterCard)?;
        self.send_to_field(FieldMessage::FromClient {
            from: self.client_id,
            request: FieldRequest::Broadcast(shown),
        })
        .await
    }
}
