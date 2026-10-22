use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::items;
use crate::packet::build::world::{effect, inventory, stat};
use game::scroll::{self, ScrollOutcome, WHITE_SCROLL};
use game::{InventoryChange, InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use rand::Rng;
use std::io::Cursor;
use tracing::debug;

/// `USE_UPGRADE_SCROLL` (v83 FUN_00a09221): tick, scroll slot in the USE bag,
/// target equip slot (negative: worn), White Scroll (2 = use one, 1 = none),
/// Legendary Spirit. The client sends it when a scroll is dropped on the
/// Equip window, or from Legendary Spirit's dialog for an equip in the Equip
/// tab (positive slot, last byte 1), and only checks the category, so every
/// rule is checked here. The reply unlocks the client's item actions, and
/// everyone on the map sees the scroll's glow (SHOW_SCROLL_EFFECT).
pub struct UseUpgradeScrollHandler;

impl WorldHandler for UseUpgradeScrollHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let scroll_slot = reader.read_short()?;
        let equip_slot = reader.read_short()?;
        let white = reader.read_short()? & 2 != 0;
        let legendary = reader.read_byte().unwrap_or(0) != 0;
        let unlock = stat::build_stat_update(true, &[])?;
        let target = if legendary {
            (equip_slot > 0 && has_legendary_spirit(ctx.player)).then_some(InventoryType::Equip)
        } else {
            (equip_slot < 0).then_some(InventoryType::Equipped)
        };
        let used = target.and_then(|target| {
            use_scroll(
                ctx.player,
                scroll_slot,
                (target, equip_slot),
                white,
                &mut |lo, hi| rand::thread_rng().gen_range(lo, hi),
            )
        });
        let Some(used) = used else {
            debug!(
                scroll_slot,
                equip_slot, white, legendary, "Scroll use refused"
            );
            let result = HandlerResult::reply(unlock);
            // Legendary Spirit's dialog waits for an answer and shows "You
            // cannot use a Scroll with this item." for -1.
            return Ok(if legendary {
                result.with_reply(effect::build_scroll_effect(
                    ctx.player.id(),
                    -1,
                    false,
                    true,
                    false,
                )?)
            } else {
                result
            });
        };
        let shown = effect::build_scroll_effect(
            ctx.player.id(),
            i8::from(used.outcome == ScrollOutcome::Success),
            used.outcome == ScrollOutcome::Cursed,
            legendary,
            used.white,
        )?;
        let mut result =
            HandlerResult::reply(inventory::build_inventory_operation(true, &used.changes)?)
                .with_reply(shown.clone())
                .with(WorldAction::Field(FieldRequest::Broadcast(shown)));
        if !legendary && used.outcome != ScrollOutcome::Fail {
            result = result.with(WorldAction::LookChanged(inventory::build_char_look(
                ctx.player,
            )?));
        }
        Ok(result)
    }
}

/// Legendary Spirit for Beginners, Noblesses and Legends.
const LEGENDARY_SPIRITS: [i32; 3] = [1003, 10001003, 20001003];

/// Whether the player has learned Legendary Spirit, which lets them scroll
/// equips in the Equip tab.
fn has_legendary_spirit(player: &Player) -> bool {
    LEGENDARY_SPIRITS
        .iter()
        .any(|&skill| player.skills.level(skill) > 0)
}

struct ScrollUse {
    changes: Vec<InventoryChange>,
    outcome: ScrollOutcome,
    /// A White Scroll was used up.
    white: bool,
}

/// Use the scroll in USE `scroll_slot` on the equip at `(kind, equip_slot)`
/// (worn, or in the Equip tab for Legendary Spirit), with a White Scroll if
/// `white` and the player has one; `roll` is `scroll::apply`'s. `None`
/// (nothing changes) unless the player is alive, the scroll and the equip
/// exist, the scroll fits it (category, `req` list, slots left) and it is not
/// a cash item (in the Equip tab, pet equipment may be: the dialog lets
/// 180xxxx through). The changes are the scroll and White Scroll taken, then
/// the equip removed and, unless destroyed, added back with its new stats.
fn use_scroll(
    player: &mut Player,
    scroll_slot: i16,
    (kind, equip_slot): (InventoryType, i16),
    white: bool,
    roll: &mut impl FnMut(i32, i32) -> i32,
) -> Option<ScrollUse> {
    if player.stats.hp <= 0 {
        return None;
    }
    let scroll_id = player
        .inventory
        .get(InventoryType::Use, scroll_slot)?
        .item_id;
    let scroll = items::info(scroll_id)?.scroll?;
    let mut item = player.inventory.get(kind, equip_slot)?.clone();
    let base_slots = items::info(item.item_id)?.equip?.upgrade_slots;
    let fits = scroll::can_apply(
        scroll_id,
        &scroll,
        item.item_id,
        item.equip.as_ref()?,
        base_slots,
    );
    let pet_equip = kind == InventoryType::Equip && item.item_id / 10_000 == 180;
    if item.is_cash() && !pet_equip || !fits {
        return None;
    }
    let white_slot = if white && scroll.kind.takes_white_scroll() {
        player
            .inventory
            .items(InventoryType::Use)
            .find(|(_, item)| item.item_id == WHITE_SCROLL)
            .map(|(slot, _)| slot)
    } else {
        None
    };
    let equip = item.equip.as_mut()?;
    let outcome = scroll::apply(equip, &mut item.flags, &scroll, white_slot.is_some(), roll);

    let mut next = player.inventory.clone();
    let mut changes = next.take(InventoryType::Use, scroll_slot, 1).ok()?;
    if let Some(slot) = white_slot {
        changes.extend(next.take(InventoryType::Use, slot, 1).ok()?);
    }
    changes.extend(next.take(kind, equip_slot, 1).ok()?);
    if outcome != ScrollOutcome::Cursed {
        next.insert(kind, equip_slot, item.clone());
        changes.push(InventoryChange::Add {
            kind,
            slot: equip_slot,
            item,
        });
    }
    *player.inventory_mut() = next;
    Some(ScrollUse {
        changes,
        outcome,
        white: white_slot.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::inventory::equip_slot;
    use game::Item;
    use packet::io::write::PktWrite;

    const SWORD: i32 = 1302000;
    const WORN: (InventoryType, i16) = (InventoryType::Equipped, equip_slot::WEAPON);

    fn request(scroll_slot: i16, equip_slot: i16, white: i16, legendary: u8) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseUpgradeScroll as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_short(scroll_slot).unwrap();
        p.write_short(equip_slot).unwrap();
        p.write_short(white).unwrap();
        p.write_byte(legendary).unwrap();
        p
    }

    fn handle(player: &mut Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseUpgradeScrollHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    /// A player wearing a new sword, with `scrolls` in USE slots 1, 2, ...
    fn armed_player(scrolls: &[(i32, i16)]) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            items::create(SWORD, 1).unwrap(),
        );
        for (slot, &(id, quantity)) in (1..).zip(scrolls) {
            player
                .inventory
                .insert(InventoryType::Use, slot, Item::stack(id, quantity));
        }
        player
    }

    fn sword(player: &Player) -> Option<game::EquipStats> {
        player
            .inventory
            .get(InventoryType::Equipped, equip_slot::WEAPON)?
            .equip
            .clone()
    }

    #[test]
    fn a_sure_scroll_upgrades_the_sword_and_everyone_sees_it() {
        if items::data().is_err() {
            return;
        }
        let mut player = armed_player(&[(2043000, 1)]);
        let before = sword(&player).unwrap();
        let actions = handle(&mut player, request(1, equip_slot::WEAPON, 1, 0));
        let [WorldAction::Reply(op), WorldAction::Reply(own), WorldAction::Field(FieldRequest::Broadcast(seen)), WorldAction::LookChanged(_)] =
            actions.as_slice()
        else {
            panic!("unexpected actions {actions:?}");
        };
        assert_eq!(op.opcode(), SendOpcode::InventoryOperation as i16);
        // exclusive, 3 changes: remove the scroll, remove and add the sword,
        // then the look refresh byte.
        assert_eq!(&op.bytes[2..4], &[1, 3]);
        assert_eq!(op.bytes.last(), Some(&2));
        assert_eq!(own.bytes, seen.bytes);
        assert_eq!(&own.bytes[6..], &[1, 0, 0, 0]);
        let after = sword(&player).unwrap();
        assert_eq!(after.watk, before.watk + 1);
        assert_eq!(after.upgrade_slots, before.upgrade_slots - 1);
        assert_eq!(after.level, 1);
        assert!(player.inventory.get(InventoryType::Use, 1).is_none());
    }

    #[test]
    fn a_white_scroll_is_used_up_and_keeps_the_slot() {
        if items::data().is_err() {
            return;
        }
        let mut player = armed_player(&[(2043001, 2), (WHITE_SCROLL, 1)]);
        let before = sword(&player).unwrap();
        let used = use_scroll(&mut player, 1, WORN, true, &mut |_, _| 99).unwrap();
        assert_eq!((used.outcome, used.white), (ScrollOutcome::Fail, true));
        assert_eq!(
            used.changes,
            vec![
                InventoryChange::Quantity {
                    kind: InventoryType::Use,
                    slot: 1,
                    quantity: 1
                },
                InventoryChange::Remove {
                    kind: InventoryType::Use,
                    slot: 2
                },
                InventoryChange::Remove {
                    kind: InventoryType::Equipped,
                    slot: equip_slot::WEAPON
                },
                InventoryChange::Add {
                    kind: InventoryType::Equipped,
                    slot: equip_slot::WEAPON,
                    item: player
                        .inventory
                        .get(InventoryType::Equipped, equip_slot::WEAPON)
                        .unwrap()
                        .clone()
                },
            ]
        );
        assert_eq!(sword(&player).unwrap(), before);
        // Without a White Scroll left, the flag is ignored.
        let used = use_scroll(&mut player, 1, WORN, true, &mut |_, _| 99).unwrap();
        assert!(!used.white);
        assert_eq!(
            sword(&player).unwrap().upgrade_slots,
            before.upgrade_slots - 1
        );
    }

    #[test]
    fn a_cursed_failure_destroys_the_worn_equip() {
        if items::data().is_err() {
            return;
        }
        let mut player = armed_player(&[(2043004, 1)]);
        let mut rolls = [99, 0].into_iter();
        let used = use_scroll(&mut player, 1, WORN, false, &mut |_, _| {
            rolls.next().unwrap()
        })
        .unwrap();
        assert_eq!(used.outcome, ScrollOutcome::Cursed);
        assert_eq!(
            used.changes,
            vec![
                InventoryChange::Remove {
                    kind: InventoryType::Use,
                    slot: 1
                },
                InventoryChange::Remove {
                    kind: InventoryType::Equipped,
                    slot: equip_slot::WEAPON
                },
            ]
        );
        assert!(sword(&player).is_none());
    }

    #[test]
    fn scrolls_that_do_not_fit_only_unlock() {
        if items::data().is_err() {
            return;
        }
        // A helmet scroll, a sword with no slots left, a sword in the bag.
        let mut player = armed_player(&[(2040000, 1), (2043000, 1)]);
        let unlock_only = |actions: Vec<WorldAction>| {
            matches!(actions.as_slice(), [WorldAction::Reply(p)]
                if p.opcode() == SendOpcode::StatChange as i16)
        };
        assert!(unlock_only(handle(
            &mut player,
            request(1, equip_slot::WEAPON, 1, 0)
        )));
        assert!(unlock_only(handle(&mut player, request(2, 1, 1, 0))));
        let mut worn = player
            .inventory
            .get(InventoryType::Equipped, equip_slot::WEAPON)
            .unwrap()
            .clone();
        worn.equip.as_mut().unwrap().upgrade_slots = 0;
        player
            .inventory
            .insert(InventoryType::Equipped, equip_slot::WEAPON, worn);
        assert!(unlock_only(handle(
            &mut player,
            request(2, equip_slot::WEAPON, 1, 0)
        )));
        assert_eq!(player.inventory.count(2043000), 1);
        player.stats.hp = 0;
        assert!(use_scroll(&mut player, 2, WORN, false, &mut |_, _| 0).is_none());
    }

    /// Whether `actions` are Legendary Spirit's refusal: the unlock and a
    /// -1 for its dialog.
    fn refused_in_the_dialog(actions: &[WorldAction]) -> bool {
        matches!(actions, [WorldAction::Reply(unlock), WorldAction::Reply(shown)]
            if unlock.opcode() == SendOpcode::StatChange as i16
                && shown.opcode() == SendOpcode::ShowScrollEffect as i16
                && shown.bytes[6..] == [0xFF, 0, 1, 0])
    }

    fn learn(player: &mut Player, skill_id: i32) {
        player.skills.set(game::SkillRecord {
            skill_id,
            level: 1,
            master_level: 0,
            expiration: None,
        });
    }

    #[test]
    fn legendary_spirit_scrolls_a_sword_in_the_equip_tab() {
        if items::data().is_err() {
            return;
        }
        let mut player = armed_player(&[(2043000, 1)]);
        player
            .inventory
            .insert(InventoryType::Equip, 3, items::create(SWORD, 1).unwrap());
        let before = player
            .inventory
            .get(InventoryType::Equip, 3)
            .unwrap()
            .clone();
        // Without the skill, the dialog is told no and nothing changes.
        assert!(refused_in_the_dialog(&handle(
            &mut player,
            request(1, 3, 1, 1)
        )));
        assert_eq!(player.inventory.count(2043000), 1);

        learn(&mut player, 10001003);
        let actions = handle(&mut player, request(1, 3, 1, 1));
        let [WorldAction::Reply(op), WorldAction::Reply(own), WorldAction::Field(FieldRequest::Broadcast(seen))] =
            actions.as_slice()
        else {
            panic!("unexpected actions {actions:?}");
        };
        assert_eq!(op.opcode(), SendOpcode::InventoryOperation as i16);
        assert_eq!(&op.bytes[2..4], &[1, 3]);
        assert_eq!(own.bytes, seen.bytes);
        assert_eq!(&own.bytes[6..], &[1, 0, 1, 0]);
        let after = player.inventory.get(InventoryType::Equip, 3).unwrap();
        let (before, after) = (before.equip.unwrap(), after.equip.clone().unwrap());
        assert_eq!(after.watk, before.watk + 1);
        assert_eq!(after.upgrade_slots, before.upgrade_slots - 1);
        assert!(player.inventory.get(InventoryType::Use, 1).is_none());
    }

    #[test]
    fn legendary_spirit_takes_only_the_equip_tab() {
        if items::data().is_err() {
            return;
        }
        let mut player = armed_player(&[(2043000, 2)]);
        learn(&mut player, 1003);
        // The worn sword from the dialog, an empty Equip-tab slot, and the
        // Equip tab without the dialog's byte.
        assert!(refused_in_the_dialog(&handle(
            &mut player,
            request(1, equip_slot::WEAPON, 1, 1)
        )));
        assert!(refused_in_the_dialog(&handle(
            &mut player,
            request(1, 3, 1, 1)
        )));
        player
            .inventory
            .insert(InventoryType::Equip, 3, items::create(SWORD, 1).unwrap());
        let actions = handle(&mut player, request(1, 3, 1, 0));
        assert!(
            matches!(actions.as_slice(), [WorldAction::Reply(p)] if p.opcode() == SendOpcode::StatChange as i16)
        );
        assert_eq!(player.inventory.count(2043000), 2);
    }

    #[test]
    fn a_request_without_the_legendary_byte_still_works() {
        if items::data().is_err() {
            return;
        }
        let mut player = armed_player(&[(2043000, 1)]);
        let mut packet = request(1, equip_slot::WEAPON, 1, 0);
        packet.bytes.pop();
        let actions = handle(&mut player, packet);
        assert_eq!(actions.len(), 4);
    }
}
