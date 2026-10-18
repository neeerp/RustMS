//! The player's pets: showing them to the player, who keeps them in a list
//! that closes up, and to the others on the map.

use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::pet::PetChange;
use game::FieldAvatar;
use net::packet::build::world::pet::{
    build_pet_exception_list, build_remove_pet, build_spawn_pet, PetGone,
};
use packet::Packet;

impl ClientActor {
    /// The player summoned or sent back pets: show everyone on the map, and
    /// keep the field avatar for those who arrive later.
    pub(super) async fn pets_changed(
        &mut self,
        changes: Vec<PetChange>,
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
                    let packet = build_remove_pet(avatar.id, index, PetGone::Dismissed)?;
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
