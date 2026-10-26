//! Gachapon machines (report c058): a machine trades a Gachapon Ticket for a
//! prize, rolled on the server since v83 kept the prize lists there. A
//! ticket does nothing when double-clicked; only the machine's NPC script
//! takes it. The prizes come from the content pack or our stand-ins
//! (`content::gachapon`).

use crate::content;
use game::InventoryType;
use game_data::server_content::{GachaponTable, GachaponTier};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// The Gachapon Ticket, from the Cash Shop.
pub const TICKET: i32 = 5220000;

/// The Remote Gachapon Ticket, from the Cash Shop: its window plays any
/// machine from afar (`remote_machine`).
pub const REMOTE_TICKET: i32 = 5451000;

/// The bags a prize may go to.
pub const BAGS: [InventoryType; 4] = [
    InventoryType::Equip,
    InventoryType::Use,
    InventoryType::Setup,
    InventoryType::Etc,
];

/// The machines `Map.nx` places, and the town each is named after in the
/// world notice of a win. Their order is the Remote Gachapon window's
/// buttons' (report b196).
const MACHINES: [(i32, &str); 10] = [
    (9100100, "Henesys"),
    (9100101, "Ellinia"),
    (9100102, "Perion"),
    (9100103, "Kerning City"),
    (9100104, "Sleepywood"),
    (9100105, "Mushroom Shrine"),
    (9100106, "Showa Spa (M)"),
    (9100107, "Showa Spa (F)"),
    (9100109, "New Leaf City"),
    (9100117, "Nautilus Harbor"),
];

/// The town of machine `npc_id`, if it is one.
pub fn town(npc_id: i32) -> Option<&'static str> {
    MACHINES
        .iter()
        .find(|(machine, _)| *machine == npc_id)
        .map(|(_, town)| *town)
}

/// The machine the Remote Gachapon window's button `index` plays (0 to 9,
/// report b196).
pub fn remote_machine(index: i32) -> Option<i32> {
    let index = usize::try_from(index).ok()?;
    MACHINES.get(index).map(|&(machine, _)| machine)
}

/// What a ticket won.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prize {
    pub item_id: i32,
    pub quantity: i16,
    pub tier: GachaponTier,
}

impl Prize {
    /// Whether the world hears of it: only prizes past the common ones
    /// (lanes/QUESTIONS.md).
    pub fn announced(&self) -> bool {
        self.tier != GachaponTier::Common
    }
}

/// A prize from machine `npc_id`, from the content pack's prizes or our
/// stand-ins. `None` if the machine has none.
pub fn roll(npc_id: i32, rng: &mut impl Rng) -> Option<Prize> {
    roll_from(content::gachapon(), npc_id, rng)
}

/// Out of 100: 90 common, 8 uncommon and 2 rare (Cosmic's split; v83's is
/// the server's own). A tier with no prizes gives way to the one below it.
fn roll_from(table: &GachaponTable, npc_id: i32, rng: &mut impl Rng) -> Option<Prize> {
    let tiers = [
        GachaponTier::Rare,
        GachaponTier::Uncommon,
        GachaponTier::Common,
    ];
    let first = match rng.gen_range(0u32, 100) {
        0..=1 => 0,
        2..=9 => 1,
        _ => 2,
    };
    tiers[first..].iter().find_map(|&tier| {
        let prizes = table.prizes(npc_id, tier);
        let prize = prizes.get(rng.gen_range(0, prizes.len().max(1)))?;
        Some(Prize {
            item_id: prize.item,
            // A pack's count of 0 still gives one.
            quantity: prize.count.max(1),
            tier,
        })
    })
}

/// The dice for a roll: `RUSTMS_GACHAPON_SEED`, for tests, makes every roll
/// the same.
pub fn rng() -> StdRng {
    match std::env::var("RUSTMS_GACHAPON_SEED")
        .ok()
        .and_then(|seed| seed.parse().ok())
    {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_entropy(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::mock::StepRng;

    /// A roll of `n` out of 100, then the first prize of the tier.
    fn rolling(n: u64) -> StepRng {
        // gen_range scales the value it draws: n/100 of the way up a u32,
        // and next to nothing of a u64, which picks a tier's first prize.
        StepRng::new((n << 32) / 100 + 1, 0)
    }

    #[test]
    fn the_roll_splits_90_8_2() {
        let table = GachaponTable::load(None).unwrap();
        let tier = |n| roll_from(&table, 9100100, &mut rolling(n)).unwrap().tier;
        assert_eq!(tier(0), GachaponTier::Rare);
        assert_eq!(tier(1), GachaponTier::Rare);
        assert_eq!(tier(2), GachaponTier::Uncommon);
        assert_eq!(tier(9), GachaponTier::Uncommon);
        assert_eq!(tier(10), GachaponTier::Common);
        assert_eq!(tier(99), GachaponTier::Common);
    }

    #[test]
    fn an_empty_tier_gives_way_to_the_one_below() {
        let dir = std::env::temp_dir().join(format!("rustms-gacha-net-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("gachapon.json"),
            r#"{"global": {"common": [{"item": 2000000, "count": 30}]}}"#,
        )
        .unwrap();
        let table = GachaponTable::load(Some(&dir)).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        let prize = roll_from(&table, 9100100, &mut rolling(0)).unwrap();
        assert_eq!(
            prize,
            Prize {
                item_id: 2000000,
                quantity: 30,
                tier: GachaponTier::Common
            }
        );
        assert!(!prize.announced());
    }

    #[test]
    fn a_seed_makes_every_roll_the_same() {
        let table = GachaponTable::load(None).unwrap();
        let roll = |seed| roll_from(&table, 9100100, &mut StdRng::seed_from_u64(seed)).unwrap();
        assert_eq!(roll(7), roll(7));
        // The harness's seed (docker-compose.test.yml) wins an uncommon
        // prize, which the world hears of.
        assert_eq!(roll(HARNESS_SEED).tier, GachaponTier::Uncommon);
    }

    #[test]
    fn every_stand_in_prize_is_an_item() {
        let table = GachaponTable::load(None).unwrap();
        for tier in [
            GachaponTier::Common,
            GachaponTier::Uncommon,
            GachaponTier::Rare,
        ] {
            for prize in table.prizes(9100100, tier) {
                assert!(
                    crate::items::item_name(prize.item).is_some(),
                    "{} has no name",
                    prize.item
                );
            }
        }
    }

    #[test]
    fn the_machines_are_where_map_nx_puts_them() {
        assert_eq!(town(9100100), Some("Henesys"));
        assert_eq!(town(9100117), Some("Nautilus Harbor"));
        assert_eq!(town(9100108), None);
    }

    #[test]
    fn the_remote_windows_buttons_play_the_ten_machines() {
        let machines: Vec<_> = (0..10).map(|i| remote_machine(i).unwrap()).collect();
        assert_eq!(
            machines[..8],
            [9100100, 9100101, 9100102, 9100103, 9100104, 9100105, 9100106, 9100107]
        );
        assert_eq!(machines[8..], [9100109, 9100117]);
        assert_eq!(remote_machine(-1), None);
        assert_eq!(remote_machine(10), None);
    }

    const HARNESS_SEED: u64 = 10;
}
