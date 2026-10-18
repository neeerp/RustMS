//! The player's pets: showing them to the player, who keeps them in a list
//! that closes up, and to the others on the map; and their hunger.

use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::pet::PetChange;
use game::FieldAvatar;
use net::packet::build::world::inventory::build_inventory_operation;
use net::packet::build::world::pet::{
    build_pet_exception_list, build_pet_name, build_remove_pet, build_spawn_pet, PetGone,
};
use net::packet::build::world::stat::{build_stat_update, StatUpdate};
use net::packet::handle::world::pets_grow_hungry;
use packet::Packet;
use std::time::Duration;

/// How often the pets out grow hungry (Cosmic: PET_EXHAUST_COUNT, 3, of
/// its one-minute ticks).
pub(super) const PET_HUNGER_INTERVAL: Duration = Duration::from_secs(180);

impl ClientActor {
    /// The player summoned, sent back or renamed pets: show everyone on the
    /// map, and keep the field avatar for those who arrive later. The owner
    /// reads why those sent back went.
    pub(super) async fn pets_changed(
        &mut self,
        changes: Vec<PetChange>,
        why: PetGone,
    ) -> Result<(), RuntimeError> {
        let game = self.in_game()?;
        let (x, y) = game.position;
        let avatar = game.player.field_avatar(x, y, 0);
        let mut announce = Vec::new();
        let mut own = Vec::new();
        for change in changes {
            match change {
                PetChange::Summoned { index, serial } => {
                    let Some(pet) = avatar.pets.iter().find(|pet| pet.serial == serial) else {
                        continue;
                    };
                    let packet = build_spawn_pet(avatar.id, index, pet)?;
                    own.push(packet.clone());
                    own.push(build_pet_exception_list(avatar.id, index, serial, &[])?);
                    announce.push(packet);
                }
                PetChange::Dismissed { index } => {
                    let packet = build_remove_pet(avatar.id, index, why)?;
                    own.push(packet.clone());
                    announce.push(packet);
                }
                PetChange::Renamed { index } => {
                    let Some(pet) = avatar.pets.get(usize::from(index)) else {
                        continue;
                    };
                    let packet = build_pet_name(avatar.id, index, &pet.name)?;
                    own.push(packet.clone());
                    announce.push(packet);
                }
            }
        }
        self.send_all(&mut own).await?;
        self.send_to_field(FieldMessage::UpdateAvatar {
            client_id: self.client_id,
            avatar,
            announce,
        })
        .await
    }

    /// The pets out grow hungry: their owner's client reads their new
    /// fullness, and those starving go home.
    pub(super) async fn pets_grow_hungry(&mut self) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        let hunger = |item_id| net::items::info(item_id).map_or(1, |info| info.pet_hunger);
        let hungry = pets_grow_hungry(&mut game.player, hunger);
        if hungry.changes.is_empty() {
            return Ok(());
        }
        self.send(&mut build_inventory_operation(false, &hungry.changes)?)
            .await?;
        if hungry.gone.is_empty() {
            return Ok(());
        }
        self.pets_changed(hungry.gone, PetGone::Hungry).await?;
        let serials = self.in_game()?.player.pet_serials();
        self.send(&mut build_stat_update(false, &[StatUpdate::Pets(serials)])?)
            .await
    }

    /// On a new map the pets come along: the player's client shows only
    /// those it is told about.
    pub(super) async fn bring_pets(&mut self, avatar: &FieldAvatar) -> Result<(), RuntimeError> {
        let mut packets: Vec<Packet> = Vec::new();
        for (index, pet) in avatar.pets.iter().enumerate() {
            packets.push(build_spawn_pet(avatar.id, index as u8, pet)?);
            packets.push(build_pet_exception_list(
                avatar.id,
                index as u8,
                pet.serial,
                &[],
            )?);
        }
        self.send_all(&mut packets).await
    }
}
