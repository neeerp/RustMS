//! Pets (report 159): summoning one from the Cash Inventory, sending it
//! back, its moves, commands and chat.

use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::items;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::pet::{build_pet_chat, build_pet_command, build_pet_level_up};
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use game::pet::PetCommand;
use game::{InventoryChange, InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use rand::Rng;
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

/// PET_COMMAND (0xA9, FUN_00704d5d): the pet's cash serial, whether the
/// words began with its name, and the command (`interact/<n>`) they
/// matched. The server rolls whether the pet obeys; obeying earns
/// closeness, which re-sends the pet's item and may raise its level.
pub struct PetCommandHandler;

impl WorldHandler for PetCommandHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let serial = r.read_long()?;
        let named = r.read_byte()? != 0;
        let command = r.read_byte()?;
        let roll = rand::thread_rng().gen_range(0, 100);
        let known = |item_id| {
            items::info(item_id)
                .and_then(|info| info.pet_commands.get(usize::from(command)).copied())
        };
        let Some(done) = command_pet(ctx.player, serial, known, roll) else {
            return Ok(HandlerResult::empty());
        };
        ctx.player.pets_named.retain(|&pet| pet != serial);
        if named {
            ctx.player.pets_named.push(serial);
        }
        let owner = ctx.player.id();
        // The answer goes first: the client checks the command's levels
        // against the pet's level before the item update raises it.
        let mut result = HandlerResult::reply(build_pet_command(
            owner,
            done.index,
            command,
            done.obeyed,
            named,
        )?);
        if !done.changes.is_empty() {
            result = result.with_reply(build_inventory_operation(false, &done.changes)?);
        }
        if done.levelled {
            result = result
                .with_reply(build_pet_level_up(None, done.index)?)
                .with(WorldAction::Field(FieldRequest::Broadcast(
                    build_pet_level_up(Some(owner), done.index)?,
                )));
        }
        Ok(result)
    }
}

struct Commanded {
    index: u8,
    obeyed: bool,
    /// The pet's item, re-sent when its closeness changed.
    changes: Vec<InventoryChange>,
    levelled: bool,
}

/// Give the summoned pet `serial` the command `known` finds for its item,
/// with `roll` in 0..100. `None` if the pet isn't out or doesn't know it.
fn command_pet(
    player: &mut Player,
    serial: i64,
    known: impl FnOnce(i32) -> Option<PetCommand>,
    roll: i32,
) -> Option<Commanded> {
    let index = player.pets.iter().position(|&pet| pet == serial)? as u8;
    let (slot, item) = player
        .inventory
        .items(InventoryType::Cash)
        .find(|(_, item)| item.cash_serial == Some(serial))?;
    let mut item = item.clone();
    let pet = item.pet.as_mut()?;
    let command = known(item.item_id)?;
    let obeyed = command.obeyed(pet, roll);
    let closeness = pet.closeness;
    let levelled = obeyed && pet.add_closeness(command.inc);
    let mut changes = Vec::new();
    if pet.closeness != closeness {
        changes.push(InventoryChange::Remove {
            kind: InventoryType::Cash,
            slot,
        });
        changes.push(InventoryChange::Add {
            kind: InventoryType::Cash,
            slot,
            item: item.clone(),
        });
        player.inventory.insert(InventoryType::Cash, slot, item);
        player.mark_dirty();
    }
    Some(Commanded {
        index,
        obeyed,
        changes,
        levelled,
    })
}

/// PET_CHAT (0xA8, FUN_007055e2): what the owner's client showed its pet
/// do and say, as `kind` (0 a command, 1 food), `action` and `text`. The
/// others on the map see it.
pub struct PetChatHandler;

/// The longest line a pet says; PetDialog's are well under it.
const PET_CHAT_MAX: usize = 127;

impl WorldHandler for PetChatHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let serial = r.read_long()?;
        let kind = r.read_byte()?;
        let action = r.read_byte()?;
        let text = r.read_str_with_length()?;
        let index = ctx.player.pets.iter().position(|&pet| pet == serial);
        let Some(index) = index.filter(|_| text.len() <= PET_CHAT_MAX) else {
            return Ok(HandlerResult::empty());
        };
        // 0xA8 doesn't say how the owner's client showed the line; a
        // command's answer (kind 0) takes the style its 174 had.
        let styled = kind == 0 && ctx.player.pets_named.contains(&serial);
        let shown = build_pet_chat(ctx.player.id(), index as u8, kind, action, &text, styled)?;
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::Broadcast(shown))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{Item, PetData};

    const TIGER: i32 = 5000066;

    /// A player with a Baby Tiger out, serial 77, in Cash slot 3.
    fn owner() -> Player {
        let mut player = crate::codec::stats::sample_player();
        let mut tiger = Item::stack(TIGER, 1);
        tiger.cash_serial = Some(77);
        tiger.pet = Some(PetData::new("Baby Tiger"));
        player.inventory.insert(InventoryType::Cash, 3, tiger);
        player.pets = vec![77];
        player
    }

    fn sit() -> Option<PetCommand> {
        Some(PetCommand {
            levels: (1, 9),
            prob: 85,
            inc: 3,
        })
    }

    fn tiger(player: &Player) -> PetData {
        let item = player.inventory.get(InventoryType::Cash, 3).unwrap();
        item.pet.clone().unwrap()
    }

    #[test]
    fn an_obeying_pet_grows_closer() {
        let mut player = owner();
        let done = command_pet(&mut player, 77, |_| sit(), 84).unwrap();
        assert!(done.obeyed && done.levelled);
        assert_eq!((tiger(&player).closeness, tiger(&player).level), (3, 3));
        assert!(matches!(
            done.changes[..],
            [
                InventoryChange::Remove { slot: 3, .. },
                InventoryChange::Add { slot: 3, .. }
            ]
        ));
        assert!(player.is_dirty());
    }

    #[test]
    fn a_pet_that_refuses_stays_as_it_was() {
        let mut player = owner();
        let done = command_pet(&mut player, 77, |_| sit(), 85).unwrap();
        assert!(!done.obeyed && !done.levelled && done.changes.is_empty());
        assert_eq!(tiger(&player).closeness, 0);
    }

    #[test]
    fn only_a_pet_that_is_out_and_knows_the_command_answers() {
        let mut player = owner();
        assert!(command_pet(&mut player, 78, |_| sit(), 0).is_none());
        assert!(command_pet(&mut player, 77, |_| None, 0).is_none());
        player.pets.clear();
        assert!(command_pet(&mut player, 77, |_| sit(), 0).is_none());
    }
}
