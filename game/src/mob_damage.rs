//! Bounds on what a mob's touch or attack can take from a player. The v83
//! client works the damage out itself (FUN_0079309f physical,
//! FUN_0079345e magic) and only reports it, so the server checks that the
//! report is possible.

/// The W.DEF a player of job category `category` (the hundreds digit of
/// the job) and `level` is expected to have (Base.wz StandardPDD.img, read
/// by FUN_00792d21). Each level takes the last listed value at or below it,
/// or 0 before the first.
pub fn standard_pdd(category: i32, level: i32) -> i32 {
    const BEGINNER: &[(i32, i32)] = &[(1, 7), (5, 17), (8, 19)];
    const WARRIOR: &[(i32, i32)] = &[
        (10, 54),
        (12, 57),
        (15, 83),
        (20, 106),
        (22, 109),
        (25, 129),
        (30, 154),
        (35, 179),
        (40, 203),
        (47, 208),
        (50, 261),
        (55, 267),
        (60, 305),
        (65, 308),
        (70, 359),
        (75, 356),
        (80, 382),
        (85, 388),
        (90, 440),
        (95, 446),
        (100, 494),
    ];
    const MAGICIAN: &[(i32, i32)] = &[
        (8, 25),
        (10, 31),
        (13, 40),
        (15, 49),
        (18, 54),
        (20, 56),
        (25, 60),
        (28, 64),
        (30, 75),
        (33, 91),
        (35, 98),
        (40, 99),
        (48, 107),
        (50, 131),
        (55, 134),
        (58, 142),
        (60, 159),
        (65, 162),
        (68, 170),
        (70, 184),
        (75, 190),
        (78, 198),
        (80, 212),
        (85, 218),
        (88, 226),
        (90, 240),
        (95, 246),
        (98, 254),
        (100, 266),
    ];
    const BOWMAN: &[(i32, i32)] = &[
        (10, 32),
        (15, 49),
        (20, 65),
        (25, 80),
        (30, 95),
        (35, 110),
        (40, 125),
        (50, 145),
        (55, 148),
        (60, 177),
        (65, 180),
        (70, 206),
        (75, 212),
        (80, 238),
        (85, 244),
        (90, 270),
        (95, 276),
        (100, 298),
    ];
    const THIEF: &[(i32, i32)] = &[
        (10, 42),
        (15, 60),
        (20, 76),
        (22, 85),
        (25, 100),
        (30, 115),
        (32, 116),
        (35, 131),
        (37, 132),
        (40, 147),
        (50, 184),
        (55, 187),
        (60, 220),
        (65, 223),
        (70, 257),
        (75, 263),
        (80, 291),
        (85, 297),
        (90, 325),
        (95, 331),
        (100, 331),
    ];
    const PIRATE: &[(i32, i32)] = &[
        (10, 24),
        (15, 43),
        (20, 56),
        (25, 69),
        (30, 82),
        (35, 95),
        (40, 108),
        (50, 146),
        (55, 149),
        (60, 178),
        (65, 181),
        (70, 207),
        (75, 213),
        (80, 239),
        (85, 245),
        (90, 271),
        (100, 309),
    ];
    let table = match category {
        0 => BEGINNER,
        1 => WARRIOR,
        2 => MAGICIAN,
        3 => BOWMAN,
        4 => THIEF,
        5 => PIRATE,
        _ => return 0,
    };
    table
        .iter()
        .take_while(|(from, _)| *from <= level)
        .last()
        .map_or(0, |(_, pdd)| *pdd)
}

/// The player's side of a hit, from what the server knows: base stats,
/// without equipment.
pub struct Defender {
    pub level: i32,
    pub job: i32,
    pub str: i32,
    pub dex: i32,
    pub int: i32,
    pub luk: i32,
}

/// The most a physical hit with `attack` (the mob's or its attack's
/// PADamage, clamped to 0..=1999) can take: the highest roll, 0.85 *
/// attack^2 / 100, with no W.DEF at all. Below the job's expected W.DEF a
/// hit gains up to 1.3 times the shortfall, weighted by level and a stat
/// factor; the stats are doubled to allow for equipment.
pub fn most_physical(player: &Defender, attack: i32) -> i32 {
    let attack = f64::from(attack.clamp(0, 1999));
    let raw = attack * attack * 0.0085;
    let category = player.job % 1000 / 100;
    let factor = f64::from(2 * (player.str + player.dex + player.int + player.luk).max(0));
    let shortfall = f64::from(standard_pdd(category, player.level))
        * (f64::from(player.level) * 0.0018181818 + factor * 0.00125 + 0.28)
        * 1.3;
    (raw + shortfall).ceil() as i32
}

/// The most a magic hit with the mob's `attack` can take: the highest roll,
/// 0.8 * attack^2 / 100; the player's stats only take from it.
pub fn most_magic(attack: i32) -> i32 {
    let attack = f64::from(attack.clamp(0, 1999));
    (attack * attack * 0.008).ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_takes_the_last_listed_value() {
        assert_eq!(standard_pdd(0, 1), 7);
        assert_eq!(standard_pdd(0, 30), 19);
        assert_eq!(standard_pdd(1, 9), 0);
        assert_eq!(standard_pdd(4, 100), 331);
        assert_eq!(standard_pdd(5, 200), 309);
    }

    #[test]
    fn a_shortfall_raises_the_bound() {
        // A level 30 magician took 137 from Stumpy's 125 in the client's
        // formula, over 125^2 * 0.0085 = 133.
        let magician = Defender {
            level: 30,
            job: 210,
            str: 4,
            dex: 25,
            int: 150,
            luk: 4,
        };
        assert!(most_physical(&magician, 125) >= 137);
        assert_eq!(most_magic(200), 320);
    }
}
