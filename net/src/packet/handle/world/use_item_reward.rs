//! Reward boxes (report c037): the Use items with a `reward` node, such as
//! the Golden Pig's eggs and the King Pepe boxes. The client sends them as
//! USE_ITEM_REWARD rather than USE_ITEM and leaves the roll to the server,
//! which trades the box for one of the entries, picked by weight.

use crate::handler::{
    BroadcastScope, FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler,
    WorldResult,
};
use crate::items;
use crate::packet::build::world::effect::build_reward_item_effect;
use crate::packet::build::world::inventory::{
    build_inventory_full, build_inventory_operation, build_item_gain_in_chat,
};
use crate::packet::build::world::messaging::build_item_line;
use crate::packet::build::world::stat::build_stat_update;
use game::{InventoryChange, InventoryType, Player};
use game_data::is_use_item;
use game_data::items::Reward;
use packet::{io::read::PktRead, Packet};
use rand::Rng;
use std::io::Cursor;
use std::time::{Duration, SystemTime};
use tracing::debug;

/// USE_ITEM_REWARD (0x70, FUN_00a1249f): the USE slot and the item id. The
/// client stays locked until the inventory changes or an empty stat update
/// comes, so every answer ends in one.
pub struct UseItemRewardHandler;

impl WorldHandler for UseItemRewardHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let slot = r.read_short()?;
        let item_id = r.read_int()?;
        let unlock = build_stat_update(true, &[])?;
        let Some(rewards) = may_open(ctx.player, slot, item_id) else {
            debug!(slot, item_id, "Reward box refused");
            return Ok(HandlerResult::reply(unlock));
        };
        let Some(reward) = pick(&rewards, &mut rand::thread_rng()) else {
            return Ok(HandlerResult::reply(unlock));
        };
        let Some(changes) = open(ctx.player, slot, &reward, SystemTime::now()) else {
            return Ok(HandlerResult::reply(build_inventory_full()?).with_reply(unlock));
        };
        // The prize's sound and the entry's animation, for the opener and
        // the others on the map.
        let effect = reward
            .effect
            .as_deref()
            .filter(|effect| !effect.is_empty())
            .map(effect_path);
        let effect = effect.as_deref();
        let mut result = HandlerResult::reply(build_inventory_operation(true, &changes)?)
            .with_reply(build_item_gain_in_chat(
                reward.item_id,
                i32::from(reward.count),
            )?)
            .with_reply(build_reward_item_effect(None, reward.item_id, effect)?)
            .with(WorldAction::Field(FieldRequest::Broadcast(
                build_reward_item_effect(Some(ctx.player.id()), reward.item_id, effect)?,
            )));
        if let Some(line) = world_line(ctx.player.name(), &reward) {
            result = result.with(WorldAction::Broadcast {
                scope: BroadcastScope::World,
                packet: build_item_line(&line, reward.item_id)?,
            });
        }
        Ok(result)
    }
}

/// What the box `item_id` in USE `slot` holds, if the player may open it
/// here: they're alive, the map allows the item, and it is there.
fn may_open(player: &Player, slot: i16, item_id: i32) -> Option<Vec<Reward>> {
    if player.stats.hp <= 0 || !is_use_item(item_id) {
        return None;
    }
    let field = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id));
    if !field.is_none_or(|field| field.may_use(item_id)) {
        return None;
    }
    if player.inventory.get(InventoryType::Use, slot)?.item_id != item_id {
        return None;
    }
    let rewards = items::info(item_id)?.reward;
    (!rewards.is_empty()).then_some(rewards)
}

/// One entry, each as likely as its `prob` against the box's total: a
/// single roll, not one per entry.
fn pick(rewards: &[Reward], rng: &mut impl Rng) -> Option<Reward> {
    let total: u32 = rewards.iter().map(|reward| reward.prob).sum();
    if total == 0 {
        return None;
    }
    at(rewards, rng.gen_range(0, total))
}

/// The entry `roll` lands on, counting each entry's `prob` in turn.
fn at(rewards: &[Reward], mut roll: u32) -> Option<Reward> {
    rewards.iter().cloned().find(|reward| {
        if roll < reward.prob {
            return true;
        }
        roll -= reward.prob;
        false
    })
}

/// The entry's `worldMsg` told to the world (report c038), its `/name` the
/// opener's and its `/item` the prize's name, which the client links; `None`
/// if it has none, or the prize has no name to give.
fn world_line(opener: &str, reward: &Reward) -> Option<String> {
    let message = reward.world_msg.as_deref()?;
    let item = items::item_name(reward.item_id)?;
    Some(message.replace("/name", opener).replace("/item", &item))
}

/// The entry's `Effect` as a path into the Effect file: Item.nx writes it
/// without the `.img` its image node carries (`Effect/BasicEff/Event1/Best`
/// is `BasicEff.img/Event1/Best`), so it is added after the image's name,
/// which every client resolves (report c038).
fn effect_path(effect: &str) -> String {
    let mut parts: Vec<&str> = effect.split('/').collect();
    let image = match parts.get(1) {
        Some(image) if parts.len() > 2 && !parts.iter().any(|p| p.ends_with(".img")) => {
            format!("{image}.img")
        }
        _ => return effect.to_owned(),
    };
    parts[1] = &image;
    parts.join("/")
}

/// Trade the box in USE `slot` for `reward`, which lasts its `period` in
/// minutes from `now` (lanes/QUESTIONS.md); `None`, with nothing changed,
/// if it doesn't fit.
fn open(
    player: &mut Player,
    slot: i16,
    reward: &Reward,
    now: SystemTime,
) -> Option<Vec<InventoryChange>> {
    let mut next = player.inventory.clone();
    let mut changes = next.take(InventoryType::Use, slot, 1).ok()?;
    if items::refuses_only(&next, reward.item_id, i32::from(reward.count)) {
        return None;
    }
    let until = u64::try_from(reward.period)
        .ok()
        .filter(|&minutes| minutes > 0)
        .map(|minutes| now + Duration::from_secs(minutes * 60));
    changes.extend(items::give_until(&mut next, reward.item_id, reward.count, until).ok()?);
    *player.inventory_mut() = next;
    Some(changes)
}

#[cfg(test)]
mod tests {
    use super::{at, effect_path, open, pick, world_line, UseItemRewardHandler};
    use crate::handler::{FieldRequest, WorldAction, WorldContext, WorldHandler};
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::{InventoryType, Item, Player};
    use game_data::items::Reward;
    use packet::{io::write::PktWrite, Packet};
    use std::time::{Duration, SystemTime};

    /// The Treasure Hunt Note: eight entries, weights summing to 100.
    const NOTE: i32 = 2_022_309;
    const SLOT: i16 = 3;

    fn holding(item_id: i32, count: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, SLOT, Item::stack(item_id, count));
        player
    }

    /// The opcodes of the answer to opening the item in `slot`.
    fn open_box(player: &mut Player, slot: i16, item_id: i32) -> Vec<i16> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseItemReward as i16).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseItemRewardHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(p)
                | WorldAction::Field(FieldRequest::Broadcast(p))
                | WorldAction::Broadcast { packet: p, .. } => {
                    i16::from_le_bytes([p.bytes[0], p.bytes[1]])
                }
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    fn reward(item_id: i32, prob: u32) -> Reward {
        Reward {
            item_id,
            count: 1,
            prob,
            ..Reward::default()
        }
    }

    #[test]
    fn one_roll_walks_the_weights() {
        let rewards = [reward(1, 10), reward(2, 0), reward(3, 30)];
        let picked = |roll| at(&rewards, roll).map(|r| r.item_id);
        assert_eq!(picked(0), Some(1));
        assert_eq!(picked(9), Some(1));
        assert_eq!(picked(10), Some(3));
        assert_eq!(picked(39), Some(3));
        let mut rng = rand::thread_rng();
        assert!(pick(&rewards, &mut rng).is_some_and(|r| r.item_id != 2));
        assert_eq!(pick(&[reward(1, 0)], &mut rng), None);
        assert_eq!(pick(&[], &mut rng), None);
    }

    #[test]
    fn a_box_gives_one_of_its_entries() {
        let Some(info) = crate::items::info(NOTE) else {
            return;
        };
        let mut player = holding(NOTE, 2);
        let held = |player: &Player| -> Vec<i32> {
            info.reward
                .iter()
                .map(|r| player.inventory.count(r.item_id))
                .collect()
        };
        let before = held(&player);
        // The gain line, the prize's effect for the opener and the map.
        assert_eq!(
            open_box(&mut player, SLOT, NOTE),
            [
                SendOpcode::InventoryOperation as i16,
                SendOpcode::ShowItemGainInChat as i16,
                SendOpcode::ShowItemGainInChat as i16,
                SendOpcode::ShowForeignEffect as i16
            ]
        );
        assert_eq!(player.inventory.count(NOTE), 1);
        // Exactly one entry came, whole.
        let gained: Vec<(&Reward, i32)> = info
            .reward
            .iter()
            .zip(held(&player).into_iter().zip(before).map(|(a, b)| a - b))
            .filter(|&(_, gained)| gained != 0)
            .collect();
        assert!(
            gained
                .iter()
                .any(|&(r, gained)| gained == i32::from(r.count)),
            "{gained:?}"
        );
    }

    #[test]
    fn the_world_hears_of_a_big_prize() {
        // The Golden Pig's Egg tells the world of its best prizes.
        let Some(info) = crate::items::info(2_022_503) else {
            return;
        };
        let scroll = info
            .reward
            .iter()
            .find(|r| r.item_id == 2_049_100)
            .expect("Chaos Scroll");
        let name = crate::items::item_name(2_049_100).expect("its name");
        assert_eq!(
            world_line("Lanec", scroll).as_deref(),
            Some(format!("Lanec has obtained a {name} from the Golden Pig's Egg.").as_str())
        );
        let quiet = Reward {
            item_id: 1_132_010,
            ..Reward::default()
        };
        assert_eq!(world_line("Lanec", &quiet), None);
    }

    #[test]
    fn an_effect_names_its_image() {
        assert_eq!(
            effect_path("Effect/BasicEff/Event1/Best"),
            "Effect/BasicEff.img/Event1/Best"
        );
        assert_eq!(
            effect_path("Effect/BasicEff.img/Event1/Best"),
            "Effect/BasicEff.img/Event1/Best"
        );
        assert_eq!(effect_path("Effect/BasicEff"), "Effect/BasicEff");
    }

    #[test]
    fn a_lasting_reward_expires() {
        // The Golden Pig's belt, 5 days.
        let belt = Reward {
            item_id: 1_132_010,
            count: 1,
            prob: 1,
            period: 7_200,
            ..Reward::default()
        };
        if crate::items::info(belt.item_id).is_none() {
            return;
        }
        let mut player = holding(NOTE, 1);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        assert!(open(&mut player, SLOT, &belt, now).is_some());
        let (_, item) = player
            .inventory
            .items(InventoryType::Equip)
            .find(|(_, item)| item.item_id == belt.item_id)
            .unwrap();
        assert_eq!(
            item.expiration,
            Some(now + Duration::from_secs(5 * 24 * 60 * 60))
        );
        assert_eq!(player.inventory.count(NOTE), 0);
    }

    #[test]
    fn a_full_tab_keeps_the_box() {
        let Some(info) = crate::items::info(NOTE) else {
            return;
        };
        // Two, so the one opened frees no slot.
        let mut player = holding(NOTE, 2);
        for kind in [
            InventoryType::Equip,
            InventoryType::Use,
            InventoryType::Setup,
            InventoryType::Etc,
            InventoryType::Cash,
        ] {
            player.inventory.set_slot_limit(kind, 4);
            for slot in (1..=4).filter(|&slot| (kind, slot) != (InventoryType::Use, SLOT)) {
                player
                    .inventory
                    .insert(kind, slot, Item::stack(4_010_000 + i32::from(slot), 1));
            }
        }
        // Nothing it gives could stack with the ore.
        assert!(info
            .reward
            .iter()
            .all(|r| player.inventory.count(r.item_id) == 0));
        assert_eq!(
            open_box(&mut player, SLOT, NOTE),
            [
                SendOpcode::ShowStatusInfo as i16,
                SendOpcode::StatChange as i16
            ]
        );
        assert_eq!(player.inventory.count(NOTE), 2);
    }

    #[test]
    fn only_a_box_that_is_there_opens() {
        let unlocked = [SendOpcode::StatChange as i16];
        // Elixir: no reward node.
        let mut player = holding(2_000_004, 1);
        assert_eq!(open_box(&mut player, SLOT, 2_000_004), unlocked);
        assert_eq!(player.inventory.count(2_000_004), 1);
        let mut player = holding(NOTE, 1);
        // Not the box's slot.
        assert_eq!(open_box(&mut player, SLOT + 1, NOTE), unlocked);
        // Dead.
        player.stats.hp = 0;
        assert_eq!(open_box(&mut player, SLOT, NOTE), unlocked);
        assert_eq!(player.inventory.count(NOTE), 1);
    }
}
