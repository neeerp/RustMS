//! Pets (report 159): summoning one from the Cash Inventory, sending it
//! back, its moves, commands, chat, food and hunger.

use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::items;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::pet::{
    build_cash_pet_food, build_pet_chat, build_pet_command, build_pet_food, build_pet_level_up,
};
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use game::pet::{PetChange, PetCommand};
use game::{InventoryChange, InventoryType, Item, Player};
use game_data::items::ConsumeSpec;
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
            result = level_up(result, owner, done.index)?;
        }
        Ok(result)
    }
}

/// Show the pet at `index` levelling up to its owner and the map.
fn level_up(result: HandlerResult<WorldAction>, owner: i32, index: u8) -> WorldResult {
    Ok(result
        .with_reply(build_pet_level_up(None, index)?)
        .with(WorldAction::Field(FieldRequest::Broadcast(
            build_pet_level_up(Some(owner), index)?,
        ))))
}

/// Each pet out: its index, its Cash slot and a copy of its item.
fn pets_out(player: &Player) -> impl Iterator<Item = (usize, i16, Item)> + '_ {
    player
        .pets
        .iter()
        .enumerate()
        .filter_map(|(index, &serial)| {
            let (slot, item) = pet_item(player, serial)?;
            Some((index, slot, item))
        })
}

/// The Cash slot of the pet `serial` and a copy of its item.
fn pet_item(player: &Player, serial: i64) -> Option<(i16, Item)> {
    player
        .inventory
        .items(InventoryType::Cash)
        .find(|(_, item)| item.cash_serial == Some(serial) && item.pet.is_some())
        .map(|(slot, item)| (slot, item.clone()))
}

/// Put a pet's changed `item` back in Cash `slot`: its owner's client sees
/// it go and come back, and reads the pet's new values.
fn put_back(player: &mut Player, slot: i16, item: Item) -> [InventoryChange; 2] {
    let kind = InventoryType::Cash;
    let changes = [
        InventoryChange::Remove { kind, slot },
        InventoryChange::Add {
            kind,
            slot,
            item: item.clone(),
        },
    ];
    player.inventory.insert(kind, slot, item);
    player.mark_dirty();
    changes
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
    let (slot, mut item) = pet_item(player, serial)?;
    let command = known(item.item_id)?;
    let pet = item.pet.as_mut()?;
    let obeyed = command.obeyed(pet, roll);
    let closeness = pet.closeness;
    let levelled = obeyed && pet.add_closeness(command.inc);
    let mut changes = Vec::new();
    if pet.closeness != closeness {
        changes.extend(put_back(player, slot, item));
    }
    Some(Commanded {
        index,
        obeyed,
        changes,
        levelled,
    })
}

/// PET_FOOD (0x4C, FUN_00a09905): the tick, the USE slot and the food
/// (212xxxx). The client doesn't check that a pet is out. The hungriest
/// pet out eats it, the one the client played the eating sound for.
pub struct PetFoodHandler;

impl WorldHandler for PetFoodHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?;
        let slot = r.read_short()?;
        let item_id = r.read_int()?;
        let fullness = |food| items::info(food)?.spec.map(|spec| spec.inc);
        let Some(fed) = feed_pet(ctx.player, slot, item_id, fullness) else {
            return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
        };
        let owner = ctx.player.id();
        // As with commands, the answer goes before the item update.
        let result = HandlerResult::reply(build_pet_food(owner, fed.index, fed.enjoyed)?)
            .with_reply(build_inventory_operation(true, &fed.changes)?);
        if fed.levelled {
            return level_up(result, owner, fed.index);
        }
        Ok(result)
    }
}

struct FedPet {
    index: u8,
    /// It ate; a full pet refuses.
    enjoyed: bool,
    /// One food fewer, and the pet's item.
    changes: Vec<InventoryChange>,
    levelled: bool,
}

/// Feed the food `item_id` in USE `slot`, worth `fullness` says, to the
/// hungriest pet out (the first of equals). `None` if no pet is out or the
/// slot holds no such food.
fn feed_pet(
    player: &mut Player,
    slot: i16,
    item_id: i32,
    fullness: impl FnOnce(i32) -> Option<i32>,
) -> Option<FedPet> {
    if item_id / 10_000 != 212 || player.inventory.get(InventoryType::Use, slot)?.item_id != item_id
    {
        return None;
    }
    let (index, pet_slot, mut item) =
        pets_out(player).min_by_key(|(_, _, item)| item.pet.as_ref().map(|pet| pet.fullness))?;
    let fed = item.pet.as_mut()?.feed(fullness(item_id)?);
    let mut changes = player
        .inventory_mut()
        .take(InventoryType::Use, slot, 1)
        .ok()?;
    changes.extend(put_back(player, pet_slot, item));
    Some(FedPet {
        index: index as u8,
        enjoyed: fed.enjoyed,
        changes,
        levelled: fed.levelled,
    })
}

/// Cash pet food (524xxxx) in Cash `slot`, used through USE_CASH_ITEM
/// (0x4F type 0x1e, which needs a pet out). The first pet out it suits eats
/// it, and 76 tells the owner's client which pet to play the eating sound
/// for. The rest is as for PET_FOOD; 76 doesn't unlock the client, so a
/// refusal also sends an empty stat change.
pub fn cash_pet_food(ctx: &mut WorldContext, slot: i16, item_id: i32) -> WorldResult {
    let spec = items::info(item_id).and_then(|info| info.spec);
    let Some(fed) = spec.and_then(|spec| feed_cash_pet(ctx.player, slot, &spec)) else {
        return Ok(HandlerResult::reply(build_cash_pet_food(None)?)
            .with_reply(build_stat_update(true, &[])?));
    };
    let owner = ctx.player.id();
    let result = HandlerResult::reply(build_cash_pet_food(Some(fed.index))?)
        .with_reply(build_pet_food(owner, fed.index, fed.enjoyed)?)
        .with_reply(build_inventory_operation(true, &fed.changes)?);
    if fed.levelled {
        return level_up(result, owner, fed.index);
    }
    Ok(result)
}

/// Feed the cash food in Cash `slot`, worth what `spec` says, to the first
/// pet out it suits (Cosmic's UseCashItemHandler). `None` if it suits none.
fn feed_cash_pet(player: &mut Player, slot: i16, spec: &ConsumeSpec) -> Option<FedPet> {
    let (index, pet_slot, mut item) =
        pets_out(player).find(|(_, _, item)| spec.pets.contains(&item.item_id))?;
    let closeness = i16::try_from(spec.inc).unwrap_or(i16::MAX);
    let levelled = item.pet.as_mut()?.feed_cash(closeness);
    let mut changes = player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
        .ok()?;
    changes.extend(put_back(player, pet_slot, item));
    Some(FedPet {
        index: index as u8,
        enjoyed: true,
        changes,
        levelled,
    })
}

/// What growing hungry did to a player's pets.
#[derive(Debug, Default)]
pub struct Hungry {
    /// The pets' items, with their new fullness.
    pub changes: Vec<InventoryChange>,
    /// The pets that went home, starving.
    pub gone: Vec<PetChange>,
}

/// Every pet out loses the fullness `hunger` gives for its kind (`info/
/// hungry`); one left at 5 or less goes home (Cosmic's
/// runFullnessSchedule).
pub fn pets_grow_hungry(player: &mut Player, hunger: impl Fn(i32) -> i32) -> Hungry {
    let mut hungry = Hungry::default();
    let mut index = 0;
    while let Some(&serial) = player.pets.get(index) {
        let Some((slot, mut item)) = pet_item(player, serial) else {
            index += 1;
            continue;
        };
        let hunger = hunger(item.item_id);
        let home = item.pet.as_mut().is_some_and(|pet| pet.grow_hungry(hunger));
        hungry.changes.extend(put_back(player, slot, item));
        if home {
            player.pets_mut().remove(index);
            player.pets_named.retain(|&pet| pet != serial);
            hungry
                .gone
                .push(PetChange::Dismissed { index: index as u8 });
        } else {
            index += 1;
        }
    }
    hungry
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

    const FOOD: i32 = 2120000;

    /// The owner with a second pet out, serial 78 in Cash slot 4, and food
    /// in USE slot 1.
    fn two_pets(tiger: u8, kitty: u8) -> Player {
        let mut player = owner();
        let mut item = Item::stack(5000000, 1);
        item.cash_serial = Some(78);
        item.pet = Some(PetData::new("Brown Kitty"));
        player.inventory.insert(InventoryType::Cash, 4, item);
        player.pets.push(78);
        for (slot, fullness) in [(3, tiger), (4, kitty)] {
            let mut item = player
                .inventory
                .get(InventoryType::Cash, slot)
                .unwrap()
                .clone();
            item.pet.as_mut().unwrap().fullness = fullness;
            player.inventory.insert(InventoryType::Cash, slot, item);
        }
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(FOOD, 2));
        player
    }

    fn fullness(player: &Player, slot: i16) -> u8 {
        let item = player.inventory.get(InventoryType::Cash, slot).unwrap();
        item.pet.as_ref().unwrap().fullness
    }

    #[test]
    fn the_hungriest_pet_eats_one_food() {
        let mut player = two_pets(80, 40);
        let fed = feed_pet(&mut player, 1, FOOD, |_| Some(30)).unwrap();
        assert_eq!((fed.index, fed.enjoyed, fed.levelled), (1, true, true));
        assert_eq!((fullness(&player, 3), fullness(&player, 4)), (80, 70));
        assert!(matches!(
            fed.changes[..],
            [
                InventoryChange::Quantity {
                    slot: 1,
                    quantity: 1,
                    ..
                },
                InventoryChange::Remove { slot: 4, .. },
                InventoryChange::Add { slot: 4, .. }
            ]
        ));
    }

    #[test]
    fn of_equally_hungry_pets_the_first_eats() {
        let mut player = two_pets(100, 100);
        let fed = feed_pet(&mut player, 1, FOOD, |_| Some(30)).unwrap();
        assert_eq!((fed.index, fed.enjoyed), (0, false));
    }

    #[test]
    fn food_needs_a_pet_out_and_the_food_in_its_slot() {
        let mut player = two_pets(50, 50);
        assert!(feed_pet(&mut player, 2, FOOD, |_| Some(30)).is_none());
        assert!(feed_pet(&mut player, 1, 2000000, |_| Some(30)).is_none());
        player.pets.clear();
        assert!(feed_pet(&mut player, 1, FOOD, |_| Some(30)).is_none());
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Use, 1)
                .unwrap()
                .quantity,
            2
        );
    }

    /// A cash food in Cash slot 5 for the Brown Kitty and others.
    fn kitty_food(player: &mut Player) -> ConsumeSpec {
        player
            .inventory
            .insert(InventoryType::Cash, 5, Item::stack(5240004, 2));
        ConsumeSpec {
            inc: 100,
            pets: vec![5000004, 5000000],
            ..ConsumeSpec::default()
        }
    }

    #[test]
    fn cash_food_goes_to_the_first_pet_it_suits() {
        let mut player = two_pets(20, 100);
        let spec = kitty_food(&mut player);
        let fed = feed_cash_pet(&mut player, 5, &spec).unwrap();
        assert_eq!((fed.index, fed.enjoyed, fed.levelled), (1, true, true));
        let kitty = player.inventory.get(InventoryType::Cash, 4).unwrap();
        let kitty = kitty.pet.as_ref().unwrap();
        assert_eq!((kitty.fullness, kitty.closeness), (100, 100));
        assert_eq!(fullness(&player, 3), 20);
        assert!(matches!(
            fed.changes[..],
            [
                InventoryChange::Quantity {
                    slot: 5,
                    quantity: 1,
                    ..
                },
                InventoryChange::Remove { slot: 4, .. },
                InventoryChange::Add { slot: 4, .. }
            ]
        ));
    }

    #[test]
    fn cash_food_no_pet_out_suits_stays() {
        let mut player = two_pets(20, 20);
        let spec = kitty_food(&mut player);
        player.pets.pop();
        assert!(feed_cash_pet(&mut player, 5, &spec).is_none());
        let food = player.inventory.get(InventoryType::Cash, 5).unwrap();
        assert_eq!(food.quantity, 2);
    }

    #[test]
    fn starving_pets_go_home_and_the_rest_close_up() {
        let mut player = two_pets(8, 30);
        player.pets_named = vec![77];
        let hungry = pets_grow_hungry(&mut player, |id| if id == TIGER { 3 } else { 2 });
        assert_eq!(hungry.gone, [PetChange::Dismissed { index: 0 }]);
        assert_eq!(player.pets, [78]);
        assert!(player.pets_named.is_empty());
        assert_eq!((fullness(&player, 3), fullness(&player, 4)), (15, 28));
        assert_eq!(hungry.changes.len(), 4);
    }
}
