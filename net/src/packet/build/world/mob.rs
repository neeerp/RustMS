use super::buff::write_mask;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::MobStat;
use packet::{io::write::PktWrite, Packet};
use std::time::Duration;

/// A status on a mob, as the client is told of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobStatusView {
    pub stat: MobStat,
    pub value: i16,
    /// The skill that caused it.
    pub skill_id: i32,
    /// How long it has left.
    pub left: Duration,
    /// A counter's spread: it hurts the attacker its value, give or take
    /// half this. 0 for the rest.
    pub spread: i32,
}

/// A mob as other players see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobView {
    pub object_id: i32,
    pub mob_id: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    /// The foothold the mob stands on, 0 if airborne. v83 puts the mob on it.
    pub foothold: i16,
    /// The spawn point's foothold.
    pub home_foothold: i16,
    /// Fade in (a fresh spawn) rather than just appear.
    pub fade_in: bool,
    pub controlled: bool,
    pub statuses: Vec<MobStatusView>,
    /// A mob just summoned by another appears with this
    /// `Effect/Summon.img` entry rather than fading in.
    pub summon_effect: Option<u8>,
}

/// The statuses as the client reads them (FUN_0078b0b1): their flags, then
/// each one's value, skill, and time left in half seconds, in the client's
/// order; then each counter's spread, and with either the chance in
/// percent that they hurt (0 always).
fn write_statuses(p: &mut Packet, statuses: &[MobStatusView]) -> Result<(), NetworkError> {
    write_mask(p, statuses.iter().map(|status| status.stat.bit()))?;
    let find = |stat| statuses.iter().find(|status| status.stat == stat);
    for stat in MobStat::DECODE_ORDER {
        let Some(status) = find(stat) else {
            continue;
        };
        p.write_short(status.value)?;
        p.write_int(status.skill_id)?;
        let halves = status.left.as_millis().div_ceil(500);
        p.write_short(halves.min(i16::MAX as u128) as i16)?;
    }
    let counters = [find(MobStat::PCounter), find(MobStat::MCounter)];
    for counter in counters.iter().flatten() {
        p.write_int(counter.spread)?;
    }
    if counters.iter().any(Option::is_some) {
        p.write_int(0)?;
    }
    Ok(())
}

/// The shared mob body of SPAWN_MONSTER and SPAWN_MONSTER_CONTROL.
fn write_mob(p: &mut Packet, mob: &MobView) -> Result<(), NetworkError> {
    p.write_int(mob.object_id)?;
    p.write_byte(if mob.controlled { 1 } else { 5 })?;
    p.write_int(mob.mob_id)?;
    write_statuses(p, &mob.statuses)?;
    p.write_short(mob.x)?;
    p.write_short(mob.y)?;
    p.write_byte(mob.stance)?;
    p.write_short(mob.foothold)?;
    p.write_short(mob.home_foothold)?;
    // How it appears (FUN_00662884): a summon effect of its own, with an
    // int the client waits in ms before playing it, -2 fading in, or -1
    // just there.
    match mob.summon_effect {
        Some(effect) => {
            p.write_byte(effect)?;
            p.write_int(0)?;
        }
        None => {
            p.write_byte(if mob.fade_in { -2i8 as u8 } else { -1i8 as u8 })?;
        }
    }
    p.write_byte(-1i8 as u8)?; // team
    p.write_int(0)?; // item effect
    Ok(())
}

/// SPAWN_MONSTER: show a mob.
pub fn build_spawn_mob(mob: &MobView) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMonster as i16)?;
    write_mob(&mut p, mob)?;
    Ok(p)
}

/// SPAWN_MONSTER_CONTROL mode 1: this client now moves the mob. Mode 2
/// (`aggro`) also has it chase this client's player.
pub fn build_control_mob(mob: &MobView, aggro: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMonsterControl as i16)?;
    p.write_byte(if aggro { 2 } else { 1 })?;
    write_mob(
        &mut p,
        &MobView {
            controlled: true,
            ..mob.clone()
        },
    )?;
    Ok(p)
}

/// SPAWN_MONSTER_CONTROL mode 0: stop moving the mob.
pub fn build_stop_controlling_mob(object_id: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMonsterControl as i16)?;
    p.write_byte(0)?;
    p.write_int(object_id)?;
    Ok(p)
}

/// MOVE_MONSTER: relay the controller's movement to the others.
pub fn build_move_mob(moved: &MobMoveView, movement_bytes: &[u8]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MoveMonster as i16)?;
    p.write_int(moved.object_id)?;
    p.write_byte(0)?; // not forced to land
    p.write_byte(u8::from(moved.next_attack))?;
    p.write_byte(moved.action)?;
    p.write_int(moved.skill_data)?;
    p.write_short(moved.start.0)?;
    p.write_short(moved.start.1)?;
    p.write_bytes(movement_bytes)?;
    Ok(p)
}

/// A controller's mob move as the others see it (FUN_0066be61).
pub struct MobMoveView {
    pub object_id: i32,
    /// Whether its controller may attack with it next.
    pub next_attack: bool,
    /// What it did: action << 1 | facing left, or 0xFF for nothing.
    pub action: u8,
    /// The attack's target, or the skill's id and level.
    pub skill_data: i32,
    pub start: (i16, i16),
}

/// MOVE_MONSTER_RESPONSE: acknowledge the controller's movement, saying
/// whether it may attack next, the mob's MP, and the mob skill and level it
/// may cast next, or 0 (FUN_0066c23b). The client keeps a skill it was
/// given until it casts it, whatever later responses say.
pub fn build_move_mob_response(
    object_id: i32,
    move_id: i16,
    next_attack: bool,
    mp: i16,
    (skill, level): (u8, u8),
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MoveMonsterResponse as i16)?;
    p.write_int(object_id)?;
    p.write_short(move_id)?;
    p.write_byte(u8::from(next_attack))?;
    p.write_short(mp)?;
    p.write_byte(skill)?;
    p.write_byte(level)?;
    Ok(p)
}

/// DAMAGE_MONSTER: a number over a mob (0x66c6c2), here only the HP a mob
/// skill healed, sent negative, then its HP and max HP for its HP tag.
pub fn build_heal_mob(
    object_id: i32,
    healed: i32,
    hp: i32,
    max_hp: i32,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::DamageMonster as i16)?;
    p.write_int(object_id)?;
    p.write_byte(0)?;
    p.write_int(-healed)?;
    p.write_int(hp)?;
    p.write_int(max_hp)?;
    Ok(p)
}

/// KILL_MONSTER: remove a mob; `animation` 1 plays its death.
pub fn build_kill_mob(object_id: i32, animation: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::KillMonster as i16)?;
    p.write_int(object_id)?;
    p.write_byte(animation)?;
    p.write_byte(animation)?;
    Ok(p)
}

/// SHOW_MONSTER_HP: the HP bar shown to the player who hit it.
pub fn build_mob_hp(object_id: i32, percent: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowMonsterHp as i16)?;
    p.write_int(object_id)?;
    p.write_byte(percent)?;
    Ok(p)
}

/// SHOW_MAGNET: another player's Monster Magnet caught the mob, or failed
/// to; the client shows the catch over it (0x66d6b9).
pub fn build_show_magnet(object_id: i32, success: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowMagnet as i16)?;
    p.write_int(object_id)?;
    p.write_byte(u8::from(success))?;
    Ok(p)
}

/// CATCH_MONSTER_WITH_ITEM: a player threw the 227xxxx item `item_id` at
/// the mob; the client plays the item's ItemEff over it, or its `fail`
/// frames (0x66d997). It leaves the mob on the map: a catch also kills it.
pub fn build_catch_with_item(
    object_id: i32,
    item_id: i32,
    success: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CatchMonsterWithItem as i16)?;
    p.write_int(object_id)?;
    p.write_int(item_id)?;
    p.write_byte(u8::from(success))?;
    Ok(p)
}

/// Why a catch item didn't work, as BRIDLE_MOB_CATCH_FAIL says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatchFail {
    /// "Unable to capture the monster; the monster is too strong."
    TooStrong = 0,
    /// The item's `delayMsg`, or "Element Rock cannot be used right after
    /// usage." without one.
    TooSoon = 1,
}

/// BRIDLE_MOB_CATCH_FAIL: the 227xxxx item `item_id` couldn't catch the
/// mob, for `why` (0xa0800e). The client stays locked until a stat
/// update.
pub fn build_catch_fail(why: CatchFail, item_id: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::BridleMobCatchFail as i16)?;
    p.write_byte(why as u8)?;
    p.write_int(item_id)?;
    p.write_int(0)?;
    Ok(p)
}

/// MOB_STAT_SET: statuses put on a mob (0x66c301). The client has them
/// take hold at once, and for one that stops the mob reads a byte more.
pub fn build_mob_stat_set(
    object_id: i32,
    statuses: &[MobStatusView],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MobStatSet as i16)?;
    p.write_int(object_id)?;
    write_statuses(&mut p, statuses)?;
    p.write_short(0)?; // delay
    p.write_byte(0)?;
    if statuses.iter().any(|status| status.stat.stops()) {
        p.write_byte(0)?;
    }
    Ok(p)
}

/// MOB_STAT_RESET: statuses a mob lost (0x66c424). For one that stopped
/// it, a mob some templates mark reads a byte more; the rest ignore it.
pub fn build_mob_stat_reset(object_id: i32, stats: &[MobStat]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MobStatReset as i16)?;
    p.write_int(object_id)?;
    write_mask(&mut p, stats.iter().map(|stat| stat.bit()))?;
    p.write_byte(0)?;
    if stats.iter().any(|stat| stat.stops()) {
        p.write_byte(0)?;
    }
    Ok(p)
}

/// Touch damage, the `TAKE_DAMAGE` source (`TakeDamagePacket::TOUCH`).
pub const FROM_TOUCH: i8 = -1;
/// One of the map's traps (an obj marked `obstacle`), the source the v83
/// client sends from 0x9593f7 with no mob.
pub const FROM_OBSTACLE: i8 = -3;
/// A mob's mist, the source the v83 client sends from FUN_00431783.
pub const FROM_MIST: i8 = -4;

/// How a player was hurt, as the others see it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hurt {
    /// A mob's attack index, [`FROM_TOUCH`], or something with no mob,
    /// such as [`FROM_MIST`].
    pub from: i8,
    pub damage: i32,
    pub mob_id: i32,
    /// Whether the mob was to the player's left.
    pub left: u8,
    /// The Power Guard or Mana Reflection percent, and what the player's
    /// client sent for the others to show over the mob: whether it was
    /// Power Guard, the mob's object id and hit action, and its position.
    pub reflect: Option<(u8, [u8; 10])>,
    /// Whether Power Stance kept the player in place.
    pub stance: bool,
    /// The number shown over the player.
    pub shown: i32,
    /// The skill that dodged the hit (Guardian or Shadow Shifter), shown
    /// in place of a number.
    pub dodged_by: Option<i32>,
}

/// DAMAGE_PLAYER: show others that a player was hurt (FUN_009832e3: below
/// touch it reads no mob).
pub fn build_damage_player(character_id: i32, hurt: &Hurt) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::DamagePlayer as i16)?;
    p.write_int(character_id)?;
    p.write_byte(hurt.from as u8)?;
    p.write_int(hurt.damage)?;
    if hurt.from >= FROM_TOUCH {
        p.write_int(hurt.mob_id)?;
        p.write_byte(hurt.left)?;
        match &hurt.reflect {
            Some((percent, shown)) => {
                p.write_byte(*percent)?;
                p.write_bytes(shown)?;
            }
            None => {
                p.write_byte(0)?;
            }
        }
        p.write_byte(u8::from(hurt.stance))?;
    }
    match hurt.dodged_by {
        Some(skill_id) => {
            p.write_int(-1)?;
            p.write_int(skill_id)?;
        }
        None => p.write_int(hurt.shown)?,
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    fn snail() -> MobView {
        MobView {
            object_id: 2_000_000_001,
            mob_id: 100100,
            x: 147,
            y: -25,
            stance: 5,
            foothold: 45,
            home_foothold: 44,
            fade_in: true,
            controlled: false,
            statuses: Vec::new(),
            summon_effect: None,
        }
    }

    #[test]
    fn spawn_carries_the_mob_statuses() {
        let mut mob = snail();
        mob.statuses = vec![MobStatusView {
            stat: MobStat::Speed,
            value: -40,
            skill_id: 2101003,
            left: Duration::from_millis(39_800),
            spread: 0,
        }];
        let packet = build_spawn_mob(&mob).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonster as i16);
        expect.int(2_000_000_001).byte(5).int(100100);
        expect.long(0).long(1 << (32 + 6));
        expect.short(-40).int(2101003).short(80);
        expect.short(147);
    }

    #[test]
    fn stat_set_follows_the_client_order_and_stops_with_a_byte() {
        let status = |stat, value, skill_id| MobStatusView {
            stat,
            value,
            skill_id,
            left: Duration::from_secs(10),
            spread: 0,
        };
        let statuses = [
            status(MobStat::Doom, 1, 2311005),
            status(MobStat::PImmune, 1, 140 | 1 << 16),
            status(MobStat::Pad, -20, 1201006),
        ];
        let packet = build_mob_stat_set(5, &statuses).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MobStatSet as i16).int(5);
        expect
            .long(0)
            .long(1 << 32 | 1 << (32 + 16) | 1 << (32 + 18));
        expect.short(-20).int(1201006).short(20);
        expect.short(1).int(140 | 1 << 16).short(20);
        expect.short(1).int(2311005).short(20);
        expect.short(0).byte(0).byte(0);
        expect.end();
    }

    #[test]
    fn counters_follow_with_their_spreads_and_a_chance() {
        let counter = |stat, value, spread| MobStatusView {
            stat,
            value,
            skill_id: 145 | 3 << 16,
            left: Duration::from_secs(10),
            spread,
        };
        let statuses = [
            counter(MobStat::MCounter, 5000, 8000),
            counter(MobStat::PCounter, 5000, 5000),
        ];
        let packet = build_mob_stat_set(5, &statuses).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MobStatSet as i16).int(5);
        expect.long(0).long(1 << (32 + 29) | 1 << (32 + 30));
        expect.short(5000).int(145 | 3 << 16).short(20);
        expect.short(5000).int(145 | 3 << 16).short(20);
        expect.int(5000).int(8000).int(0);
        expect.short(0).byte(0);
        expect.end();
    }

    #[test]
    fn stat_reset_names_the_lost_statuses() {
        let packet = build_mob_stat_reset(5, &[MobStat::Seal]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MobStatReset as i16).int(5);
        expect.long(0).long(1 << (32 + 10));
        expect.byte(0);
        expect.end();
        let packet = build_mob_stat_reset(5, &[MobStat::Doom]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MobStatReset as i16).int(5);
        expect.long(0).long(1 << (32 + 16));
        expect.byte(0).byte(0);
        expect.end();
    }

    #[test]
    fn spawn_matches_the_v83_layout() {
        let packet = build_spawn_mob(&snail()).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonster as i16);
        expect.int(2_000_000_001).byte(5).int(100100);
        expect.bytes(&[0; 16]);
        expect.short(147).short(-25).byte(5).short(45).short(44);
        expect.byte(-2i8 as u8).byte(-1i8 as u8).int(0);
        expect.end();
    }

    #[test]
    fn a_summon_names_its_effect_and_a_delay() {
        let mut mob = snail();
        mob.summon_effect = Some(2);
        let packet = build_spawn_mob(&mob).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonster as i16);
        expect.int(2_000_000_001).byte(5).int(100100);
        expect.bytes(&[0; 16]);
        expect.short(147).short(-25).byte(5).short(45).short(44);
        expect.byte(2).int(0).byte(-1i8 as u8).int(0);
        expect.end();
    }

    #[test]
    fn a_mobs_hit_relays_its_reflect_stance_and_dodge() {
        let shown = [1, 9, 0, 0, 0, 4, 10, 0, 20, 0];
        let hurt = Hurt {
            from: FROM_TOUCH,
            damage: 120,
            mob_id: 100100,
            left: 1,
            reflect: Some((30, shown)),
            stance: true,
            shown: 102,
            dodged_by: None,
        };
        let packet = build_damage_player(7, &hurt).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::DamagePlayer as i16).int(7);
        expect.byte(FROM_TOUCH as u8).int(120).int(100100).byte(1);
        expect.byte(30).bytes(&shown).byte(1).int(102);
        expect.end();

        let dodged = Hurt {
            damage: -1,
            reflect: None,
            stance: false,
            dodged_by: Some(4120002),
            ..hurt
        };
        let packet = build_damage_player(7, &dodged).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::DamagePlayer as i16).int(7);
        expect.byte(FROM_TOUCH as u8).int(-1).int(100100).byte(1);
        expect.byte(0).byte(0).int(-1).int(4120002);
        expect.end();
    }

    #[test]
    fn mist_damage_names_no_mob() {
        let hurt = Hurt {
            from: FROM_MIST,
            damage: 30,
            shown: 30,
            ..Hurt::default()
        };
        let p = build_damage_player(7, &hurt).unwrap();
        let mut want = (SendOpcode::DamagePlayer as i16).to_le_bytes().to_vec();
        want.extend_from_slice(&7i32.to_le_bytes());
        want.push(FROM_MIST as u8);
        want.extend_from_slice(&30i32.to_le_bytes());
        want.extend_from_slice(&30i32.to_le_bytes());
        assert_eq!(p.bytes, want);
    }

    #[test]
    fn a_heal_is_negative_damage() {
        let packet = build_heal_mob(7, 700, 900, 1000).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::DamageMonster as i16)
            .int(7)
            .byte(0);
        expect.int(-700).int(900).int(1000);
        expect.end();
    }

    #[test]
    fn control_has_a_mode_and_marks_the_mob_controlled() {
        let packet = build_control_mob(&snail(), false).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonsterControl as i16).byte(1);
        expect.int(2_000_000_001).byte(1).int(100100);
        let aggro = build_control_mob(&snail(), true).unwrap();
        let mut expect = Expect::new(&aggro.bytes);
        expect.short(SendOpcode::SpawnMonsterControl as i16).byte(2);
        expect.int(2_000_000_001).byte(1).int(100100);
        let stop = build_stop_controlling_mob(2_000_000_001).unwrap();
        let mut expect = Expect::new(&stop.bytes);
        expect
            .short(SendOpcode::SpawnMonsterControl as i16)
            .byte(0)
            .int(2_000_000_001);
        expect.end();
    }

    #[test]
    fn movement_kill_and_hp() {
        let view = MobMoveView {
            object_id: 7,
            next_attack: true,
            action: 12 << 1 | 1,
            skill_data: 0x0050_0040,
            start: (10, 20),
        };
        let moved = build_move_mob(&view, &[1, 2]).unwrap();
        let mut expect = Expect::new(&moved.bytes);
        expect.short(SendOpcode::MoveMonster as i16).int(7);
        expect.byte(0).byte(1).byte(25).int(0x0050_0040);
        expect.short(10).short(20).byte(1).byte(2);
        expect.end();
        let response = build_move_mob_response(7, 3, true, 40, (114, 15)).unwrap();
        let mut expect = Expect::new(&response.bytes);
        expect
            .short(SendOpcode::MoveMonsterResponse as i16)
            .int(7)
            .short(3)
            .byte(1)
            .short(40)
            .byte(114)
            .byte(15);
        expect.end();
        let kill = build_kill_mob(7, 1).unwrap();
        let mut expect = Expect::new(&kill.bytes);
        expect
            .short(SendOpcode::KillMonster as i16)
            .int(7)
            .byte(1)
            .byte(1);
        expect.end();
        let hp = build_mob_hp(7, 40).unwrap();
        let mut expect = Expect::new(&hp.bytes);
        expect
            .short(SendOpcode::ShowMonsterHp as i16)
            .int(7)
            .byte(40);
        expect.end();
    }

    #[test]
    fn a_catch_names_its_item() {
        let caught = build_catch_with_item(7, 2270000, true).unwrap();
        let mut expect = Expect::new(&caught.bytes);
        expect
            .short(SendOpcode::CatchMonsterWithItem as i16)
            .int(7)
            .int(2270000)
            .byte(1);
        expect.end();
        let fail = build_catch_fail(CatchFail::TooStrong, 2270001).unwrap();
        let mut expect = Expect::new(&fail.bytes);
        expect
            .short(SendOpcode::BridleMobCatchFail as i16)
            .byte(0)
            .int(2270001)
            .int(0);
        expect.end();
    }
}
