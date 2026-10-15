use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::stat::StatUpdate;
use crate::packet::build::world::{inventory, stat};
use crate::skills;
use game::{InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `USE_ITEM`: tick, slot in the USE bag, item id. Applies the item's `spec`
/// (HP/MP, flat and percent) and consumes one.
pub struct UseItemHandler;

impl WorldHandler for UseItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        match use_item(ctx.player, slot, item_id) {
            Some(changes) => {
                let stats = &ctx.player.stats;
                Ok(
                    HandlerResult::reply(inventory::build_inventory_operation(true, &changes)?)
                        .with_reply(stat::build_stat_update(
                            true,
                            &[StatUpdate::Hp(stats.hp), StatUpdate::Mp(stats.mp)],
                        )?),
                )
            }
            None => {
                debug!(slot, item_id, "Item use refused");
                Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
            }
        }
    }
}

fn use_item(player: &mut Player, slot: i16, item_id: i32) -> Option<Vec<game::InventoryChange>> {
    if player.stats.hp <= 0 {
        return None;
    }
    if player.inventory.get(InventoryType::Use, slot)?.item_id != item_id {
        return None;
    }
    let spec = items::info(item_id)?.spec?;
    let changes = player
        .inventory_mut()
        .take(InventoryType::Use, slot, 1)
        .ok()?;
    let max_hp = i32::from(player.stats.max_hp);
    let max_mp = i32::from(player.stats.max_mp);
    player.add_hp(spec.hp + max_hp * spec.hp_r / 100);
    player.add_mp(spec.mp + max_mp * spec.mp_r / 100);
    Some(changes)
}

/// `HEAL_OVER_TIME`: the client's periodic natural recovery. The runtime
/// checks the timing; this only parses and bounds the amounts.
pub struct HealOverTimeHandler;

/// v83 heals 1.5 times as much on a chair or seat.
const SEATED_RECOVERY_RATE: f64 = 1.5;

/// Most the player may recover per tick: their natural amount at the map's
/// `info/recovery` rate, seated, or a portable chair's
/// `recoveryHP`/`recoveryMP` while they sit on one they still own.
fn max_heal(player: &Player) -> (i16, i16) {
    let natural = skills::natural_recovery(player);
    let rate = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id))
        .map_or(1.0, |field| field.recovery)
        * SEATED_RECOVERY_RATE;
    let chair = (player.chair != 0 && player.inventory.count(player.chair) > 0)
        .then(|| items::info(player.chair))
        .flatten();
    let (chair_hp, chair_mp) = chair.map_or((0, 0), |chair| (chair.recovery_hp, chair.recovery_mp));
    let bound = |natural: i32, chair: i32| {
        let natural = (f64::from(natural) * rate) as i32;
        i16::try_from(natural.max(chair)).unwrap_or(i16::MAX)
    };
    (bound(natural.hp, chair_hp), bound(natural.mp, chair_mp))
}

impl WorldHandler for HealOverTimeHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?;
        reader.read_int()?;
        let hp = reader.read_short()?;
        let mp = reader.read_short()?;
        let (max_hp, max_mp) = max_heal(ctx.player);
        if !(0..=max_hp).contains(&hp) || !(0..=max_mp).contains(&mp) {
            debug!(hp, mp, "Natural recovery out of range");
            return Ok(HandlerResult::empty());
        }
        Ok(HandlerResult::empty().with(WorldAction::NaturalHeal { hp, mp }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::Item;
    use packet::io::write::PktWrite;

    fn request(slot: i16, item_id: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseItem as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        p
    }

    fn handle(player: &mut Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseItemHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn eating_an_apple_heals_and_consumes_it() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.max_hp = 50;
        player.stats.hp = 25;
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(2010007, 1));
        let actions = handle(&mut player, request(3, 2010007));
        assert!(
            matches!(actions.as_slice(), [WorldAction::Reply(a), WorldAction::Reply(b)]
            if a.opcode() == SendOpcode::InventoryOperation as i16 && b.opcode() == SendOpcode::StatChange as i16)
        );
        assert_eq!(player.stats.hp, 50);
        assert!(player.inventory.get(InventoryType::Use, 3).is_none());
    }

    #[test]
    fn mismatched_or_missing_items_are_refused() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2000000, 5));
        for packet in [request(1, 2000001), request(2, 2000000)] {
            let actions = handle(&mut player, packet);
            assert!(matches!(actions.as_slice(), [WorldAction::Reply(p)]
                if p.opcode() == SendOpcode::StatChange as i16));
        }
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Use, 1)
                .unwrap()
                .quantity,
            5
        );
    }

    fn heal_over_time(player: &mut Player, hp: i16, mp: i16) -> bool {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::HealOverTime as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(0).unwrap();
        packet.write_short(hp).unwrap();
        packet.write_short(mp).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        let actions = HealOverTimeHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        matches!(actions.as_slice(), [WorldAction::NaturalHeal { hp: h, mp: m }] if (*h, *m) == (hp, mp))
    }

    #[test]
    fn a_portable_chair_raises_the_heal_bound() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        assert!(heal_over_time(&mut player, 10, 3));
        assert!(!heal_over_time(&mut player, 50, 0));
        // The Relaxer heals 50 HP.
        player
            .inventory
            .insert(InventoryType::Setup, 1, Item::stack(3010000, 1));
        player.chair = 3010000;
        assert!(heal_over_time(&mut player, 50, 0));
        assert!(heal_over_time(&mut player, 0, 4));
        assert!(!heal_over_time(&mut player, 51, 0));
        // Not once the chair has left the inventory.
        player.inventory.take(InventoryType::Setup, 1, 1).unwrap();
        assert!(!heal_over_time(&mut player, 50, 0));
    }

    #[test]
    fn recovery_skills_raise_the_heal_bound() {
        if skills::data().is_err() {
            return;
        }
        let learn = |player: &mut Player, skill_id, level| {
            player.skills.set(game::SkillRecord {
                skill_id,
                level,
                master_level: 0,
                expiration: None,
            })
        };
        // Improved HP Recovery 16 adds 50 HP, seated 1.5 times (10 + 50).
        let mut player = crate::codec::stats::sample_player();
        learn(&mut player, skills::IMPROVED_HP_RECOVERY, 16);
        assert!(heal_over_time(&mut player, 90, 4));
        assert!(!heal_over_time(&mut player, 91, 0));
        // A level 50 magician with Improved MP Recovery 16 adds 80 MP.
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 200;
        player.stats.level = 50;
        learn(&mut player, skills::IMPROVED_MP_RECOVERY, 16);
        assert!(heal_over_time(&mut player, 15, 124));
        assert!(!heal_over_time(&mut player, 0, 125));
    }

    #[test]
    fn a_recovery_map_raises_the_heal_bound() {
        if crate::get_game_data().is_err() {
            return;
        }
        // The Sleepywood sauna heals twice as fast.
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = 105040401;
        assert!(heal_over_time(&mut player, 30, 9));
        assert!(!heal_over_time(&mut player, 31, 0));
    }
}
