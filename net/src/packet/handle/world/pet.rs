//! Pets (report 159): summoning one from the Cash Inventory, sending it
//! back, and its moves.

use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// SPAWN_PET (0x62, FUN_00a240a2): the tick, the pet's Cash Inventory
/// slot, and whether it should lead the others. The same request sends a
/// pet out back. The client waits for the stat change before asking again.
pub struct SpawnPetHandler;

impl WorldHandler for SpawnPetHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?;
        let slot = r.read_short()?;
        let boss = r.read_byte()? != 0;
        let player = &mut *ctx.player;
        let serial = player
            .inventory
            .get(InventoryType::Cash, slot)
            .filter(|item| item.pet.is_some())
            .and_then(|item| item.cash_serial);
        let Some(serial) = serial else {
            return Ok(
                HandlerResult::empty().with(WorldAction::Reply(build_stat_update(true, &[])?))
            );
        };
        let lead = player.follows_the_lead();
        let changes = game::pet::toggle(player.pets_mut(), serial, lead, boss);
        let done = build_stat_update(true, &[StatUpdate::Pets(player.pet_serials())])?;
        Ok(HandlerResult::empty()
            .with(WorldAction::PetsChanged(changes))
            .with(WorldAction::Reply(done)))
    }
}

/// MOVE_PET (0xA7, FUN_009c4e41): the pet's cash serial, then a path as
/// MOVE_PLAYER's without its field key and CRC.
pub struct MovePetHandler;

impl WorldHandler for MovePetHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let serial = r.read_long()?;
        let start = (r.read_short()?, r.read_short()?);
        let movement = packet.bytes[r.position() as usize..].to_vec();
        let index = ctx.player.pets.iter().position(|&pet| pet == serial);
        let (Some(index), Some(&count)) = (index, movement.first()) else {
            return Ok(HandlerResult::empty());
        };
        if count == 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::PetMove {
                index: index as u8,
                start,
                movement,
            })),
        )
    }
}
