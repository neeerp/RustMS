use super::ClientActor;
use crate::actor::field_drops::{Drop, Ownership};
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldMessage};
use game::monster_book::{self, MAX_CARD_LEVEL};
use net::loot::{self, Loot};
use net::packet::build::world::effect::{self, CharEffect};
use net::packet::build::world::stat::{self, StatUpdate};
use net::packet::build::world::{drop, inventory, monster_book as book};
use net::packet::handle::world::{apply_item_effect, item_effect_packets, trade_catch};
use net::reactors::Action;
use net::FieldRequest;
use std::time::Instant;
use tracing::debug;

impl ClientActor {
    /// Roll this player's drops from a mob they helped kill (`showdown`
    /// percent more often under Taunt) and put them on the ground where it
    /// died.
    pub(super) async fn drop_loot(
        &mut self,
        mob_id: i32,
        showdown: i32,
        object_id: i32,
        x: i16,
        y: i16,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let now = std::time::SystemTime::now();
        let in_party = self.party_id.is_some();
        let loot = loot::roll(
            mob_id,
            showdown,
            player,
            in_party,
            now,
            &mut rand::thread_rng(),
        );
        let ownership = match net::mobs::info(mob_id) {
            Some(mob) if mob.explosive_reward => Ownership::Explosive,
            Some(mob) if mob.public_reward => Ownership::FreeForAll,
            _ => self.ownership(),
        };
        self.spawn_loot(loot, ownership, object_id, x, y).await
    }

    /// A mob this player stole from drops one of its items for them.
    pub(super) async fn drop_stolen(
        &mut self,
        mob_id: i32,
        object_id: i32,
        x: i16,
        y: i16,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let now = std::time::SystemTime::now();
        let loot = loot::steal(mob_id, player, now, &mut rand::thread_rng());
        self.spawn_loot(
            loot.into_iter().collect(),
            self.ownership(),
            object_id,
            x,
            y,
        )
        .await
    }

    /// A reactor this player broke drops its loot for them, or sends them
    /// where it leads (`net::reactors::action`).
    pub(super) async fn drop_reactor_loot(
        &mut self,
        reactor_id: i32,
        object_id: i32,
        x: i16,
        y: i16,
    ) -> Result<(), RuntimeError> {
        let warp = match net::reactors::action(reactor_id) {
            Action::Warp(warp) | Action::WarpMap { warp, .. } => Some(warp),
            Action::DropsOrWarp { percent, warp } => {
                (rand::random::<u32>() % 100 >= percent).then_some(warp)
            }
            _ => None,
        };
        if let Some(warp) = warp {
            let (map_id, portal) = warp.to();
            let channel_id = self.channel_id;
            let player = &mut self.in_game_mut()?.player;
            let actions = net::packet::handle::world::warp_to_spawn_point(
                player, map_id, portal, channel_id,
            )?;
            for action in actions {
                Box::pin(self.apply(action)).await?;
            }
            return Ok(());
        }
        let in_party = self.party_id.is_some();
        let player = &self.in_game()?.player;
        let loot = {
            let rng = &mut rand::thread_rng();
            let loot = loot::roll_reactor(reactor_id, player, in_party, rng);
            match net::reactors::action(reactor_id) {
                Action::Spray { at_least, mesos } => loot::spray(loot, at_least, mesos, rng),
                _ => loot,
            }
        };
        self.spawn_loot(loot, self.ownership(), object_id, x, y)
            .await
    }

    /// This player's loot is kept for them, or their party, at first.
    fn ownership(&self) -> Ownership {
        self.party_id.map_or(Ownership::Owner, Ownership::Party)
    }

    async fn spawn_loot(
        &mut self,
        loot: Vec<Loot>,
        ownership: Ownership,
        dropper: i32,
        x: i16,
        y: i16,
    ) -> Result<(), RuntimeError> {
        if loot.is_empty() {
            return Ok(());
        }
        self.send_to_field(FieldMessage::SpawnLoot {
            owner: self.client_id,
            ownership,
            loot,
            x,
            y,
            dropper,
        })
        .await
    }

    /// The field handed over a drop: put it in the inventory, or give it back
    /// if it doesn't fit.
    pub(super) async fn picked_up(&mut self, drop: Drop) -> Result<(), RuntimeError> {
        if !self.take_loot(&drop).await? {
            return self.send_to_field(FieldMessage::ReturnDrop { drop }).await;
        }
        Ok(())
    }

    /// The field handed this player's pet at `pet` a drop it still shows:
    /// put it in the inventory, and have the field show the pet taking it or
    /// leave it lying.
    pub(super) async fn pet_picked_up(&mut self, drop: Drop, pet: u8) -> Result<(), RuntimeError> {
        let taken = self.take_loot(&drop).await?;
        let looter = self.in_game()?.player.id();
        self.send_to_field(FieldMessage::PetLooted {
            looter,
            pet,
            drop,
            taken,
        })
        .await
    }

    /// Put a drop in the inventory. Returns false, having told the player,
    /// when it doesn't fit.
    async fn take_loot(&mut self, drop: &Drop) -> Result<bool, RuntimeError> {
        if let Loot::Item { item_id, .. } = drop.loot {
            if monster_book::is_card(item_id) {
                self.collect_card(item_id).await?;
                return Ok(true);
            }
        }
        let picked = match &drop.loot {
            Loot::Item { item_id, .. } => Some(*item_id),
            Loot::Dropped(item) => Some(item.item_id),
            Loot::Mesos(_) => None,
        };
        if let Some(item_id) = picked {
            if self.consume_on_pickup(item_id).await? {
                return Ok(true);
            }
        }
        let mut packets = Vec::new();
        let player = &mut self.in_game_mut()?.player;
        let incoming = match &drop.loot {
            Loot::Item {
                item_id, quantity, ..
            } => Some((*item_id, *quantity)),
            Loot::Dropped(item) => Some((item.item_id, item.quantity)),
            Loot::Mesos(_) => None,
        };
        if incoming.is_some_and(|(id, quantity)| {
            net::items::refuses_only(&player.inventory, id, quantity.into())
        }) {
            // A second one-of-a-kind item: "This item is unavailable for
            // the pick-up." and back on the ground (report c005).
            packets.push(inventory::build_item_unavailable()?);
            packets.push(stat::build_stat_update(true, &[])?);
            self.send_all(&mut packets).await?;
            return Ok(false);
        }
        let (given, item_id, quantity) = match &drop.loot {
            Loot::Mesos(amount) => {
                player.meso = player.meso.saturating_add(*amount);
                player.mark_dirty();
                let meso = player.meso;
                packets.push(stat::build_stat_update(true, &[StatUpdate::Meso(meso)])?);
                packets.push(drop::build_meso_gain(*amount)?);
                self.send_all(&mut packets).await?;
                return Ok(true);
            }
            Loot::Item {
                item_id, quantity, ..
            } => (
                net::items::give_dropped(
                    &mut player.inventory,
                    *item_id,
                    *quantity,
                    &mut rand::thread_rng(),
                ),
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
                return Ok(false);
            }
        }
        self.send_all(&mut packets).await?;
        Ok(true)
    }

    /// The field let this player catch a mob with the catch item `item_id`
    /// in USE `slot`: it turns into the item it makes, `create`.
    pub(super) async fn caught(
        &mut self,
        slot: i16,
        item_id: i32,
        create: i32,
    ) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let mut inventory = player.inventory.clone();
        let traded = trade_catch(&mut inventory, slot, item_id, create);
        let Some(changes) = traded else {
            // It moved while the mob was being caught, which is gone all
            // the same.
            debug!(slot, item_id, "Caught a mob without the catch item");
            return self.send(&mut stat::build_stat_update(true, &[])?).await;
        };
        *player.inventory_mut() = inventory;
        self.send(&mut inventory::build_inventory_operation(true, &changes)?)
            .await
    }

    /// Apply an item that works the moment it is picked up, such as the
    /// fruits 2022086-88, rather than bag it (report c036); `false` if
    /// `item_id` isn't one or does nothing the server supports yet.
    async fn consume_on_pickup(&mut self, item_id: i32) -> Result<bool, RuntimeError> {
        let Some(spec) = net::items::info(item_id)
            .and_then(|info| info.spec)
            .filter(|spec| spec.consume_on_pickup)
        else {
            return Ok(false);
        };
        if !self.take_item_effect(item_id, true).await? {
            return Ok(false);
        }
        if spec.party {
            let from = self.client_id;
            self.send_to_directory(DirectoryMessage::PartyItem { from, item_id })
                .await?;
        }
        Ok(true)
    }

    /// A party member on `map_id` picked up the party potion `item_id`: it
    /// works on this player too while they share the map and are alive.
    pub(super) async fn party_item_reached(
        &mut self,
        item_id: i32,
        map_id: i32,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        if game.player.stats.map_id != map_id || game.player.stats.hp <= 0 {
            return Ok(());
        }
        self.take_item_effect(item_id, false).await?;
        Ok(())
    }

    /// Give the player `item_id`'s effect and tell them and the map;
    /// `false` if it has none supported. `exclusive` unlocks the client's
    /// pickup with the HP and MP update.
    async fn take_item_effect(
        &mut self,
        item_id: i32,
        exclusive: bool,
    ) -> Result<bool, RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let Some(effect) = apply_item_effect(player, item_id, Instant::now()) else {
            return Ok(false);
        };
        let (mut own, seen) = item_effect_packets(player, item_id, effect, exclusive)?;
        self.send_all(&mut own).await?;
        for packet in seen {
            self.show_buffs(packet).await?;
        }
        Ok(true)
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
