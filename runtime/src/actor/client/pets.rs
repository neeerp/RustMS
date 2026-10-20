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
use net::packet::handle::world::{next_dry_up, pets_dry_up, pets_grow_hungry, PetsAged};
use packet::Packet;
use std::time::{Duration, SystemTime};

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
                    own.push(build_pet_exception_list(
                        avatar.id,
                        index,
                        serial,
                        game.player.pet_ignored(serial),
                    )?);
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
        self.pets_aged(hungry, PetGone::Hungry).await
    }

    /// When the next of the player's pets dries up; not in the Cash Shop.
    pub(super) fn pets_due(&self) -> Option<tokio::time::Instant> {
        if self.in_cash_shop {
            return None;
        }
        let wait = next_dry_up(&self.game.as_ref()?.player)?
            .duration_since(SystemTime::now())
            .unwrap_or_default();
        Some(tokio::time::Instant::now() + wait)
    }

    /// The pets whose Water of Life dried up turn back into dolls: their
    /// owner's client reads them dated 2079, and those out go home.
    pub(super) async fn pets_dry_up(&mut self) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let dried = pets_dry_up(player, SystemTime::now());
        self.pets_aged(dried, PetGone::Expired).await
    }

    /// Show the player's client its pets' new items, and send home those
    /// that went for `why`.
    async fn pets_aged(&mut self, aged: PetsAged, why: PetGone) -> Result<(), RuntimeError> {
        if aged.changes.is_empty() {
            return Ok(());
        }
        self.send(&mut build_inventory_operation(false, &aged.changes)?)
            .await?;
        if aged.gone.is_empty() {
            return Ok(());
        }
        self.pets_changed(aged.gone, why).await?;
        let serials = self.in_game()?.player.pet_serials();
        self.send(&mut build_stat_update(false, &[StatUpdate::Pets(serials)])?)
            .await
    }

    /// On a new map the pets come along: the player's client shows only
    /// those it is told about.
    pub(super) async fn bring_pets(&mut self, avatar: &FieldAvatar) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let mut packets: Vec<Packet> = Vec::new();
        for (index, pet) in avatar.pets.iter().enumerate() {
            packets.push(build_spawn_pet(avatar.id, index as u8, pet)?);
            packets.push(build_pet_exception_list(
                avatar.id,
                index as u8,
                pet.serial,
                player.pet_ignored(pet.serial),
            )?);
        }
        self.send_all(&mut packets).await
    }
}
