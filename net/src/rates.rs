//! The world's EXP, drop and meso rates, the 2x EXP (521xxxx) and drop
//! (536xxxx) cards that raise them for whoever holds one, and Hunter's Luck
//! (2450000), which raises kill EXP while its buff lasts.
//!
//! v83's client never reads a card's `rate` or `time`, and no packet starts
//! or ends one: the server multiplies kill EXP and drops while a card in the
//! Cash inventory is unexpired and the clock is inside one of its windows.
//! Nor does it read Hunter's Luck's `expBuff`; it only shows the buff's icon.

use game::{BuffStat, InventoryType, Player};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

const EXP_CARDS: i32 = 521;
const DROP_CARDS: i32 = 536;

/// The world's rates: `RUSTMS_EXP_RATE`, `RUSTMS_DROP_RATE` and
/// `RUSTMS_MESO_RATE`, each 1 when unset.
struct World {
    exp: f64,
    drop: f64,
    meso: f64,
}

fn world() -> &'static World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD.get_or_init(|| {
        let rate = |key: &str| {
            std::env::var(key)
                .ok()
                .and_then(|rate| rate.parse::<f64>().ok())
                .filter(|rate| *rate > 0.0)
                .unwrap_or(1.0)
        };
        World {
            exp: rate("RUSTMS_EXP_RATE"),
            drop: rate("RUSTMS_DROP_RATE"),
            meso: rate("RUSTMS_MESO_RATE"),
        }
    })
}

/// How much more EXP `player` earns from a kill at `now`.
pub fn exp(player: &Player, now: SystemTime) -> f64 {
    let buff = player
        .buffs
        .value(BuffStat::ExpBuff)
        .map_or(1.0, |percent| f64::from(percent) / 100.0);
    world().exp * card(player, EXP_CARDS, now) * buff
}

/// How much likelier each of `player`'s drops is at `now`.
pub fn drop(player: &Player, now: SystemTime) -> f64 {
    world().drop * card(player, DROP_CARDS, now)
}

/// How many more mesos each of `player`'s meso drops holds at `now`. A drop
/// card doubles these too (5360000's description).
pub fn meso(player: &Player, now: SystemTime) -> f64 {
    world().meso * card(player, DROP_CARDS, now)
}

/// The best rate of the `kind` cards `player` holds that work at `now`; cards
/// don't stack.
fn card(player: &Player, kind: i32, now: SystemTime) -> f64 {
    let (day, hour) = day_and_hour(now);
    player
        .inventory
        .items(InventoryType::Cash)
        .filter(|(_, item)| item.item_id / 10_000 == kind)
        .filter(|(_, item)| item.expiration.map_or(true, |at| at > now))
        .filter_map(|(_, item)| crate::items::info(item.item_id)?.rate_card)
        .filter(|card| card.active(day, hour))
        .map(|card| card.rate)
        .fold(1.0, f64::max)
}

/// The weekday (Monday is 0) and hour cards are checked against: UTC, moved
/// by `RUSTMS_RATE_CARD_UTC_OFFSET` hours.
fn day_and_hour(now: SystemTime) -> (u8, u8) {
    static OFFSET: OnceLock<i64> = OnceLock::new();
    let offset = *OFFSET.get_or_init(|| {
        std::env::var("RUSTMS_RATE_CARD_UTC_OFFSET")
            .ok()
            .and_then(|hours| hours.parse::<i64>().ok())
            .filter(|hours| (-12..=14).contains(hours))
            .unwrap_or(0)
    });
    weekday_hour(now, offset)
}

fn weekday_hour(now: SystemTime, offset: i64) -> (u8, u8) {
    let seconds = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;
    let hours = seconds.div_euclid(3600) + offset;
    // 1 January 1970 was a Thursday.
    let day = (hours.div_euclid(24) + 3).rem_euclid(7);
    (day as u8, hours.rem_euclid(24) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::Item;

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    #[test]
    fn weekdays_start_on_monday_and_follow_the_offset() {
        // 2026-10-17 20:30 UTC was a Saturday.
        let saturday = at(1_792_269_000);
        assert_eq!(weekday_hour(saturday, 0), (5, 20));
        assert_eq!(weekday_hour(saturday, -7), (5, 13));
        assert_eq!(weekday_hour(saturday, 4), (6, 0));
        assert_eq!(weekday_hour(at(0), 0), (3, 0));
    }

    #[test]
    fn a_card_counts_only_inside_its_hours_and_before_it_expires() {
        if crate::items::info(5211013).is_none() {
            return;
        }
        let saturday_evening = at(1_792_269_000);
        let saturday_morning = at(1_792_269_000 - 10 * 3600);
        let mut player = crate::codec::stats::sample_player();
        assert_eq!(card(&player, EXP_CARDS, saturday_evening), 1.0);

        // Weekend Type 5: Saturdays and Sundays, 7 to 11 pm.
        let mut weekend = Item::stack(5211013, 1);
        weekend.expiration = Some(saturday_evening + Duration::from_secs(60));
        player.inventory.add_alone(weekend).unwrap();
        assert_eq!(card(&player, EXP_CARDS, saturday_evening), 2.0);
        assert_eq!(card(&player, EXP_CARDS, saturday_morning), 1.0);
        assert_eq!(card(&player, DROP_CARDS, saturday_evening), 1.0);
        let expired = saturday_evening + Duration::from_secs(120);
        assert_eq!(card(&player, EXP_CARDS, expired), 1.0);

        // The best card counts, not their product.
        player.inventory.add_alone(Item::stack(5211060, 1)).unwrap();
        assert_eq!(card(&player, EXP_CARDS, saturday_evening), 3.0);
        player.inventory.add_alone(Item::stack(5360000, 1)).unwrap();
        assert_eq!(card(&player, DROP_CARDS, saturday_morning), 2.0);
    }

    #[test]
    fn hunters_luck_multiplies_with_the_cards() {
        let now = at(1_792_269_000);
        let mut player = crate::codec::stats::sample_player();
        let lasts = Duration::from_secs(1800);
        let buff = game::Buff::new(
            -2450000,
            BuffStat::ExpBuff,
            200,
            std::time::Instant::now(),
            lasts,
        );
        player.buffs.give(buff);
        assert_eq!(exp(&player, now), 2.0 * world().exp);
        if crate::items::info(5211060).is_some() {
            player.inventory.add_alone(Item::stack(5211060, 1)).unwrap();
            assert_eq!(exp(&player, now), 6.0 * world().exp);
        }
    }
}
