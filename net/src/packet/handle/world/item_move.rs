use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::loot::Loot;
use crate::packet::build::world::{inventory, stat};
use crate::{cash_shop, items};
use game::inventory::{equip_slot, FLAG_UNTRADEABLE};
use game::pet::PetChange;
use game::{InventoryType, Item, Player};
use game_data::items::EquipInfo;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `ITEM_MOVE`: tick, inventory type, source slot, destination slot,
/// quantity. A negative destination equips; a negative source unequips;
/// destination 0 drops `quantity` of the item where the player stands.
pub struct ItemMoveHandler;

impl WorldHandler for ItemMoveHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let kind = reader.read_byte()? as i8;
        let from = reader.read_short()?;
        let to = reader.read_short()?;
        let quantity = reader.read_short()?;

        let mut actions = HandlerResult::empty();
        if to == 0 {
            match drop_item(ctx.player, kind, from, quantity) {
                Ok((changes, loot, vanish)) => {
                    return Ok(actions
                        .with(WorldAction::Reply(inventory::build_inventory_operation(
                            true, &changes,
                        )?))
                        .with(WorldAction::Field(FieldRequest::Throw { loot, vanish })));
                }
                Err(reason) => {
                    debug!(kind, from, quantity, reason, "Item drop refused");
                    return Ok(
                        actions.with(WorldAction::Reply(stat::build_stat_update(true, &[])?))
                    );
                }
            }
        }

        let result = move_item(ctx.player, kind, from, to);
        match result {
            Ok((changes, look_changed)) if !changes.is_empty() => {
                actions = actions.with(WorldAction::Reply(inventory::build_inventory_operation(
                    true, &changes,
                )?));
                if look_changed {
                    actions = actions.with(WorldAction::LookChanged(inventory::build_char_look(
                        ctx.player,
                    )?));
                    // Taking off equipment that raised max HP or MP.
                    if ctx.player.fit_hp_mp() {
                        actions = actions.with_reply(stat::build_hp_mp(&ctx.player.stats)?);
                    }
                    // A pet out with its label ring put on or taken off
                    // changes its name tag for everyone (PET_NAMECHANGE).
                    let renamed: Vec<PetChange> = [from, to]
                        .into_iter()
                        .filter_map(|slot| {
                            let index = equip_slot::PET_LABELS
                                .iter()
                                .position(|&label| label == slot % 100)?;
                            ctx.player.summoned_pet(index)?;
                            Some(PetChange::Renamed { index: index as u8 })
                        })
                        .collect();
                    if !renamed.is_empty() {
                        actions = actions.with(WorldAction::PetsChanged(renamed));
                    }
                }
            }
            outcome => {
                debug!(kind, from, to, ?outcome, "Item move refused");
                // Unlock the client's item actions.
                actions = actions.with(WorldAction::Reply(stat::build_stat_update(true, &[])?));
            }
        }
        Ok(actions)
    }
}

/// Take `quantity` of the item in bag slot `from` to throw on the ground.
/// Returns the changes, what to throw, and whether it vanishes instead of
/// landing (quest and untradeable items, and equipment made untradeable,
/// unless Scissors of Karma were used on them, and the rings of a crush or
/// friendship pair, which no one else can wear).
fn drop_item(
    player: &mut Player,
    kind: i8,
    from: i16,
    quantity: i16,
) -> Result<(Vec<game::InventoryChange>, Loot, bool), &'static str> {
    if player.stats.hp <= 0 {
        return Err("dead");
    }
    let field = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id));
    if !field.is_none_or(|field| field.may_drop_items()) {
        return Err("the map forbids drops");
    }
    let kind = InventoryType::from_i16(i16::from(kind))
        .filter(|kind| *kind != InventoryType::Equipped)
        .ok_or("not a bag")?;
    let item = player
        .inventory
        .get(kind, from)
        .ok_or("empty slot")?
        .clone();
    if item
        .cash_serial
        .is_some_and(|serial| player.pets.contains(&serial))
    {
        return Err("a pet that is out");
    }
    if item.sealed() {
        return Err("sealed");
    }
    // Throwing stars and bullets always go as a whole set.
    let quantity = if matches!(item.item_id / 10_000, 207 | 233) {
        item.quantity
    } else {
        quantity
    };
    if quantity < 1 || quantity > item.quantity {
        return Err("bad quantity");
    }
    let changes = player
        .inventory_mut()
        .take(kind, from, quantity)
        .map_err(|_| "bad quantity")?;
    let vanish = !item.has_karma()
        && (items::info(item.item_id).is_some_and(|info| info.quest || info.trade_block)
            || item.equip.is_some() && item.flags & FLAG_UNTRADEABLE != 0)
        || cash_shop::pair_ring(item.item_id);
    // A Karma item lands, its Karma used up: "Once dropped, the item cannot
    // be traded." (0x1232)
    let mut thrown = Item { quantity, ..item };
    thrown.use_karma();
    Ok((changes, Loot::Dropped(thrown), vanish))
}

/// Apply a move; returns the changes and whether the worn equipment changed.
fn move_item(
    player: &mut Player,
    kind: i8,
    from: i16,
    to: i16,
) -> Result<(Vec<game::InventoryChange>, bool), String> {
    // A rider's client gets them off before the mount or saddle (or a
    // cover, 100 below) comes off or is swapped.
    let saddled = |slot: i16| [equip_slot::MOUNT, equip_slot::SADDLE].contains(&(slot % 100));
    if player.ride_skill().is_some() && (saddled(from) || saddled(to)) {
        return Err("riding".into());
    }
    if from < 0 {
        // Equipment dropped on an occupied bag slot swaps with what is there,
        // which wears that item, so it goes the equip way (v83 FUN_004f11b4).
        if player.inventory.get(InventoryType::Equip, to).is_some() {
            return move_item(player, 1, to, from);
        }
        // HeavenClient sends EQUIPPED (-1), the original client EQUIP (1).
        let changes = player
            .inventory_mut()
            .unequip(from, to)
            .map_err(|e| e.to_string())?;
        return Ok((changes, true));
    }
    let kind = InventoryType::from_i16(i16::from(kind))
        .filter(|kind| *kind != InventoryType::Equipped)
        .ok_or("unknown inventory type")?;
    if to < 0 {
        if kind != InventoryType::Equip {
            return Err("only equipment can be worn".into());
        }
        let item = player.inventory.get(kind, from).ok_or("empty slot")?;
        let item_id = item.item_id;
        let info = items::info(item_id).ok_or("unknown equipment")?;
        let template = info.equip.ok_or("unknown equipment")?;
        // Equipment that can't trade once worn becomes untradeable, which the
        // client is told by replacing it in the bag before the move (Cosmic
        // InventoryManipulator.equip).
        let binds = info.equip_trade_block && item.flags & FLAG_UNTRADEABLE == 0;
        let allowed = match equip_slot::pet_slots(item_id) {
            // Cash items are worn 100 below their slot.
            Some(_) => pet_can_wear(player, &template, to % 100),
            None => items::can_wear(&template, player),
        };
        if !allowed {
            return Err("requirements not met".into());
        }
        let inventory = player.inventory_mut();
        let mut changes = inventory.equip(from, to).map_err(|e| e.to_string())?;
        if binds {
            let mut worn = inventory
                .get(InventoryType::Equipped, to)
                .expect("just worn")
                .clone();
            worn.flags |= FLAG_UNTRADEABLE;
            inventory.insert(InventoryType::Equipped, to, worn.clone());
            changes.splice(
                0..0,
                [
                    game::InventoryChange::Remove { kind, slot: from },
                    game::InventoryChange::Add {
                        kind,
                        slot: from,
                        item: worn,
                    },
                ],
            );
        }
        return Ok((changes, true));
    }
    let item_id = player
        .inventory
        .get(kind, from)
        .ok_or("empty slot")?
        .item_id;
    let changes = player
        .inventory_mut()
        .move_in_bag(kind, from, to, items::slot_max(item_id))
        .map_err(|e| e.to_string())?;
    Ok((changes, false))
}

/// Whether a pet equip may go in pet equip `slot`, as the client checks
/// (v83 FUN_004f1c2d). Wear, label and quote rings, item pouches, meso
/// magnets and ignore pendants need the pet they are for out. Wear must
/// suit that pet's kind and level, a label ring its name tag style and a
/// quote ring its chat balloon style. The rest need no pet.
fn pet_can_wear(player: &Player, equip: &EquipInfo, slot: i16) -> bool {
    let pet = equip_slot::pet_index(slot).and_then(|index| player.summoned_pet(index));
    let kind = |pet: &Item| items::info(pet.item_id).unwrap_or_default();
    match -slot {
        14 | 30 | 38 => pet.is_some_and(|pet| {
            equip.pets.contains(&pet.item_id)
                && pet
                    .pet
                    .as_ref()
                    .is_some_and(|data| i16::from(data.level) >= equip.req_level)
        }),
        21 | 31 | 39 => pet.is_some_and(|pet| kind(pet).pet_name_tag),
        29 | 32 | 40 => pet.is_some_and(|pet| kind(pet).pet_chat_balloon),
        22 | 23 | 46 | 33 | 34 | 47 | 41 | 42 | 48 => pet.is_some(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::inventory::FLAG_BUNDLE_KARMA;
    use game::{inventory::equip_slot, EquipStats, Item};
    use packet::io::write::PktWrite;

    fn request(kind: i8, from: i16, to: i16) -> Packet {
        drop_request(kind, from, to, 1)
    }

    fn drop_request(kind: i8, from: i16, to: i16, quantity: i16) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::ItemMove as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_byte(kind as u8).unwrap();
        p.write_short(from).unwrap();
        p.write_short(to).unwrap();
        p.write_short(quantity).unwrap();
        p
    }

    /// What a drop throws on the field, if it was accepted.
    fn thrown(actions: &[WorldAction]) -> Option<(&Loot, bool)> {
        match actions {
            [WorldAction::Reply(p), WorldAction::Field(FieldRequest::Throw { loot, vanish })]
                if p.opcode() == SendOpcode::InventoryOperation as i16 =>
            {
                Some((loot, *vanish))
            }
            _ => None,
        }
    }

    #[test]
    fn dropping_takes_part_of_a_stack_and_keeps_equipment_as_it_was() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(4000019, 10));
        let actions = handle(&mut player, drop_request(4, 1, 0, 3));
        assert_eq!(
            thrown(&actions),
            Some((&Loot::Dropped(Item::stack(4000019, 3)), false))
        );
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Etc, 1)
                .unwrap()
                .quantity,
            7
        );
        assert!(player.is_dirty());

        let scrolled = Item::equipment(
            1302000,
            EquipStats {
                watk: 20,
                ..EquipStats::default()
            },
        );
        player
            .inventory
            .insert(InventoryType::Equip, 1, scrolled.clone());
        let actions = handle(&mut player, drop_request(1, 1, 0, 1));
        assert_eq!(thrown(&actions), Some((&Loot::Dropped(scrolled), false)));
        assert!(player.inventory.get(InventoryType::Equip, 1).is_none());
    }

    #[test]
    fn stars_go_as_a_set_and_bad_quantities_are_refused() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2070000, 500));
        let actions = handle(&mut player, drop_request(2, 1, 0, 1));
        assert_eq!(
            thrown(&actions),
            Some((&Loot::Dropped(Item::stack(2070000, 500)), false))
        );

        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(4000019, 2));
        for quantity in [0, -1, 3] {
            let actions = handle(&mut player, drop_request(4, 1, 0, quantity));
            assert!(thrown(&actions).is_none());
        }
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Etc, 1)
                .unwrap()
                .quantity,
            2
        );

        player.stats.hp = 0;
        assert!(thrown(&handle(&mut player, drop_request(4, 1, 0, 1))).is_none());
    }

    #[test]
    fn refuses_drops_where_the_map_forbids_them() {
        if crate::get_game_data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = 910_000_001;
        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(4000019, 2));
        assert!(thrown(&handle(&mut player, drop_request(4, 1, 0, 1))).is_none());
        assert_eq!(player.inventory.count(4000019), 2);
    }

    #[test]
    fn quest_items_vanish() {
        // Green Apple, a quest item in v83.
        if items::info(2010007).is_none() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2010007, 1));
        let actions = handle(&mut player, drop_request(2, 1, 0, 1));
        assert_eq!(
            thrown(&actions),
            Some((&Loot::Dropped(Item::stack(2010007, 1)), true))
        );

        // With Karma it lands, and the Karma is gone.
        let karma = Item {
            flags: FLAG_BUNDLE_KARMA,
            ..Item::stack(2010007, 1)
        };
        player.inventory.insert(InventoryType::Use, 1, karma);
        let actions = handle(&mut player, drop_request(2, 1, 0, 1));
        assert_eq!(
            thrown(&actions),
            Some((&Loot::Dropped(Item::stack(2010007, 1)), false))
        );
    }

    #[test]
    fn a_ring_of_a_pair_vanishes() {
        // A Crush Ring, tradeable in the item data.
        let mut player = crate::codec::stats::sample_player();
        let ring = Item {
            cash_serial: Some(7),
            ..Item::equipment(1112001, EquipStats::default())
        };
        player
            .inventory
            .insert(InventoryType::Equip, 1, ring.clone());
        let actions = handle(&mut player, drop_request(1, 1, 0, 1));
        assert_eq!(thrown(&actions), Some((&Loot::Dropped(ring), true)));
    }

    #[test]
    fn equipment_that_binds_becomes_untradeable_once_worn() {
        // Timeless Earrings, "Trade disabled when equipped".
        if items::info(1032031).is_none() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = 120;
        let earrings = Item::equipment(1032031, EquipStats::default());
        player
            .inventory
            .insert(InventoryType::Equip, 1, earrings.clone());
        let (changes, _) = move_item(&mut player, 1, 1, -4).unwrap();
        let bound = Item {
            flags: FLAG_UNTRADEABLE,
            ..earrings
        };
        assert_eq!(
            changes,
            [
                game::InventoryChange::Remove {
                    kind: InventoryType::Equip,
                    slot: 1
                },
                game::InventoryChange::Add {
                    kind: InventoryType::Equip,
                    slot: 1,
                    item: bound.clone()
                },
                game::InventoryChange::Move {
                    kind: InventoryType::Equip,
                    from: 1,
                    to: -4
                },
            ]
        );
        assert_eq!(
            player.inventory.get(InventoryType::Equipped, -4),
            Some(&bound)
        );

        // Worn again, it is already bound; dropped, it vanishes.
        move_item(&mut player, -1, -4, 1).unwrap();
        let (changes, _) = move_item(&mut player, 1, 1, -4).unwrap();
        assert_eq!(changes.len(), 1);
        move_item(&mut player, -1, -4, 1).unwrap();
        let actions = handle(&mut player, drop_request(1, 1, 0, 1));
        assert_eq!(thrown(&actions), Some((&Loot::Dropped(bound), true)));
    }

    fn handle(player: &mut Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        ItemMoveHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn unequipping_and_moving_reply_with_inventory_changes() {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1302000, EquipStats::default()),
        );
        let actions = handle(&mut player, request(-1, equip_slot::WEAPON, 1));
        assert!(
            matches!(actions.as_slice(), [WorldAction::Reply(p), WorldAction::LookChanged(_)]
            if p.opcode() == SendOpcode::InventoryOperation as i16)
        );
        assert!(player.inventory.get(InventoryType::Equip, 1).is_some());
        assert!(player.is_dirty());

        let actions = handle(&mut player, request(1, 1, 5));
        assert!(matches!(actions.as_slice(), [WorldAction::Reply(_)]));
        assert!(player.inventory.get(InventoryType::Equip, 5).is_some());
    }

    #[test]
    fn unequipping_onto_equipment_wears_it_instead() {
        if items::info(1302000).is_none() {
            return;
        }
        let sword = |watk| {
            Item::equipment(
                1302000,
                EquipStats {
                    watk,
                    ..EquipStats::default()
                },
            )
        };
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Equipped, equip_slot::WEAPON, sword(1));
        player.inventory.insert(InventoryType::Equip, 1, sword(2));
        player.inventory.insert(
            InventoryType::Equip,
            2,
            Item::equipment(1002000, EquipStats::default()),
        );

        // A hat can't take the sword's place.
        assert!(!worn(&handle(
            &mut player,
            request(-1, equip_slot::WEAPON, 2)
        )));

        assert!(worn(&handle(
            &mut player,
            request(1, equip_slot::WEAPON, 1)
        )));
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Equipped, equip_slot::WEAPON),
            Some(&sword(2))
        );
        assert_eq!(
            player.inventory.get(InventoryType::Equip, 1),
            Some(&sword(1))
        );
    }

    /// A player with a Brown Kitty out first and a Baby Tiger second, and
    /// `equips` in the Equip bag from slot 1, bought in the Cash Shop.
    fn pet_owner(equips: &[i32]) -> Player {
        let mut player = crate::codec::stats::sample_player();
        for (slot, (serial, item_id)) in (1..).zip([(76, 5000000), (77, 5000066)]) {
            let mut pet = Item::stack(item_id, 1);
            pet.cash_serial = Some(serial);
            pet.pet = Some(game::PetData::new("Pet"));
            player.inventory.insert(InventoryType::Cash, slot, pet);
            player.pets.push(serial);
        }
        for (slot, &item_id) in (1..).zip(equips) {
            let mut equip = Item::equipment(item_id, EquipStats::default());
            equip.cash_serial = Some(100 + i64::from(slot));
            player.inventory.insert(InventoryType::Equip, slot, equip);
        }
        player
    }

    fn worn(actions: &[WorldAction]) -> bool {
        matches!(actions, [WorldAction::Reply(p), WorldAction::LookChanged(_), ..]
            if p.opcode() == SendOpcode::InventoryOperation as i16)
    }

    #[test]
    fn a_label_ring_shows_its_pets_name_tag() {
        if items::data().is_err() {
            return;
        }
        let mut player = pet_owner(&[1822000]);
        let renamed = |actions: &[WorldAction]| {
            actions.iter().any(|action| {
                matches!(action, WorldAction::PetsChanged(changes)
                    if changes[..] == [PetChange::Renamed { index: 1 }])
            })
        };
        let tagged = |player: &Player| {
            let avatar = player.field_avatar(0, 0, 0);
            avatar
                .pets
                .iter()
                .map(|pet| pet.name_tag)
                .collect::<Vec<_>>()
        };
        assert!(renamed(&handle(&mut player, request(1, 1, -131))));
        assert_eq!(tagged(&player), [false, true]);
        assert!(renamed(&handle(&mut player, request(-1, -131, 1))));
        assert_eq!(tagged(&player), [false, false]);
    }

    #[test]
    fn pet_equips_need_the_pet_they_are_for() {
        if items::data().is_err() {
            return;
        }
        // Tiger wings, a label ring, a meso magnet, an HP pouch and a sweeper.
        let mut player = pet_owner(&[1802063, 1822000, 1812000, 1812002, 1812004]);
        // The kitty can't wear the tiger's wings, nor show a label.
        assert!(!worn(&handle(&mut player, request(1, 1, -114))));
        assert!(!worn(&handle(&mut player, request(1, 2, -121))));
        assert!(worn(&handle(&mut player, request(1, 1, -130))));
        assert!(worn(&handle(&mut player, request(1, 2, -131))));
        // There is no third pet for the magnet, but the pouch and the
        // sweeper need none.
        assert!(!worn(&handle(&mut player, request(1, 3, -142))));
        assert!(worn(&handle(&mut player, request(1, 4, -124))));
        assert!(worn(&handle(&mut player, request(1, 5, -143))));
        let worn_ids: Vec<i32> = [-130, -131, -124, -143]
            .map(|position| {
                let item = player.inventory.get(InventoryType::Equipped, position);
                item.map_or(0, |item| item.item_id)
            })
            .into();
        assert_eq!(worn_ids, [1802063, 1822000, 1812002, 1812004]);
        let look = player.look();
        assert_eq!(look.visible.get(&30), Some(&1802063));
    }

    #[test]
    fn refused_moves_only_unlock_the_client() {
        let mut player = crate::codec::stats::sample_player();
        for request in [request(2, 1, 2), request(1, 1, 0), request(9, 1, 2)] {
            let actions = handle(&mut player, request);
            assert!(matches!(actions.as_slice(), [WorldAction::Reply(p)]
                if p.opcode() == SendOpcode::StatChange as i16));
        }
        assert!(!player.is_dirty());
    }

    #[test]
    fn riders_keep_their_mount_and_saddle_on() {
        let mut player = crate::codec::stats::sample_player();
        let saddle = Item::equipment(1912000, EquipStats::default());
        player
            .inventory
            .insert(InventoryType::Equipped, equip_slot::SADDLE, saddle);
        let riding = game::Buff::new(
            1004,
            game::BuffStat::Riding,
            0,
            std::time::Instant::now(),
            game::mount::RIDE_DURATION,
        );
        player.buffs.give(riding);
        assert_eq!(
            move_item(&mut player, -1, equip_slot::SADDLE, 1),
            Err("riding".into())
        );
        assert_eq!(
            move_item(&mut player, 1, 1, equip_slot::MOUNT - 100),
            Err("riding".into()),
            "nor its cover"
        );

        player.buffs.cancel(1004);
        assert!(move_item(&mut player, -1, equip_slot::SADDLE, 1).is_ok());
    }
}
