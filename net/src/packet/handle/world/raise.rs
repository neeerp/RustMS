//! The raise window (CUIRaise, report b233): an Etc item with `info/questId`
//! and `uiData`, as the Timer's Egg (4220046) of "A Cute Baby Bird" (3250),
//! opens it on a double-click. It grows as the items its `consumeItem` lists
//! are dragged onto it, each adding its amount to the record `questId` names,
//! up to `exp` for each of its `grade` stages; the quest's completion reads
//! the record.
//!
//! On opening, the window sends 0xED (the item, then 1; 0 when it closes),
//! which the server needs nothing from, and asks for its record (0xEC). The
//! record's value comes back as a quest record message, which the client
//! hands to the window as well as its quest log.

use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::quest::build_quest_record;
use crate::packet::build::world::stat::build_stat_update;
use game::quest::QuestState;
use game::{InventoryType, Player};
use packet::io::read::PktRead;
use packet::Packet;
use std::io::Cursor;

/// Whether a quest that reads `record` (`RECORD_QUESTS`) is under way, as
/// only then the item grows.
fn raising(player: &Player, record: i16) -> bool {
    game_data::quests::RECORD_QUESTS
        .iter()
        .any(|&(quest, of)| of == record && player.quests.state(quest) == QuestState::Started)
}

/// The record's value, 0 before anything is fed.
fn growth(player: &Player, record: i16) -> i32 {
    player
        .quests
        .info(record)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

/// 0xEC: the window's record, as a short. The answer is its value, while a
/// quest that reads it is under way.
pub struct RaiseRecordHandler;

impl WorldHandler for RaiseRecordHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let record = reader.read_short()?;
        if !raising(ctx.player, record) {
            return Ok(HandlerResult::empty());
        }
        let value = growth(ctx.player, record).to_string();
        Ok(HandlerResult::reply(build_quest_record(record, &value)?))
    }
}

/// 0xEE: an item dragged onto the window: its bag as a byte, its slot as a
/// short, then the raise item's id. The client asks only while the item is
/// not yet grown, and waits for the answer to unlock.
pub struct RaiseFeedHandler;

impl WorldHandler for RaiseFeedHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let kind = reader.read_byte()?;
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        let Some((record, value, changes)) = feed(ctx.player, kind, slot, item_id) else {
            return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
        };
        ctx.player.mark_dirty();
        Ok(
            HandlerResult::reply(build_inventory_operation(true, &changes)?)
                .with_reply(build_quest_record(record, &value)?),
        )
    }
}

/// Feed `player`'s raise item `item_id` the item in bag `kind`, `slot`: take
/// one and grow the record by what it adds, short of the item's last stage.
/// The record, its new value and the inventory changes, or `None` if it
/// can't be fed that.
fn feed(
    player: &mut Player,
    kind: u8,
    slot: i16,
    item_id: i32,
) -> Option<(i16, String, Vec<game::InventoryChange>)> {
    let raise = items::info(item_id)?.raise?;
    if player.inventory.count(item_id) == 0 || !raising(player, raise.record) {
        return None;
    }
    let kind =
        InventoryType::from_i16(i16::from(kind)).filter(|kind| *kind != InventoryType::Equipped)?;
    let fed = player.inventory.get(kind, slot)?.item_id;
    let &(_, adds) = raise.feed.iter().find(|&&(id, _)| id == fed)?;
    let now = growth(player, raise.record);
    if now >= raise.max() {
        return None;
    }
    let changes = player.inventory_mut().take(kind, slot, 1).ok()?;
    let value = now.saturating_add(adds).min(raise.max()).to_string();
    player.quests.set_record(raise.record, &value);
    Some((raise.record, value, changes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::quest::{QuestRecord, QuestStatus};
    use game::Item;

    const TIMERS_EGG: i32 = 4_220_046;
    const BIRD_FEED: i32 = 4_031_992;
    const A_CUTE_BABY_BIRD: i16 = 3250;
    const GROWTH: i16 = 7067;

    fn raiser() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(TIMERS_EGG, 1));
        player
            .inventory
            .insert(InventoryType::Etc, 2, Item::stack(BIRD_FEED, 40));
        player.quests.upsert(QuestRecord {
            quest_id: A_CUTE_BABY_BIRD,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        player
    }

    #[test]
    fn the_timers_egg_grows_on_bird_feed_up_to_its_last_stage() {
        let mut player = raiser();
        let (record, value, _) = feed(&mut player, 4, 2, TIMERS_EGG).unwrap();
        assert_eq!((record, value.as_str()), (GROWTH, "100"));
        assert_eq!(player.inventory.count(BIRD_FEED), 39);

        // Not the feed, nor the egg's own slot, nor another bag.
        assert!(feed(&mut player, 4, 1, TIMERS_EGG).is_none());
        assert!(feed(&mut player, 2, 2, TIMERS_EGG).is_none());
        assert!(feed(&mut player, 4, 2, BIRD_FEED).is_none());

        player.quests.set_record(GROWTH, "2950");
        let (_, value, _) = feed(&mut player, 4, 2, TIMERS_EGG).unwrap();
        assert_eq!(value, "3000");
        // Grown, it takes nothing more.
        assert!(feed(&mut player, 4, 2, TIMERS_EGG).is_none());
        assert_eq!(player.inventory.count(BIRD_FEED), 38);
    }

    #[test]
    fn the_egg_grows_only_while_its_quest_is_under_way() {
        let mut player = raiser();
        player.quests.remove(A_CUTE_BABY_BIRD);
        assert!(feed(&mut player, 4, 2, TIMERS_EGG).is_none());
        assert_eq!(player.inventory.count(BIRD_FEED), 40);

        // Nor without the egg.
        let mut player = raiser();
        player
            .inventory_mut()
            .take(InventoryType::Etc, 1, 1)
            .unwrap();
        assert!(feed(&mut player, 4, 2, TIMERS_EGG).is_none());
    }
}
