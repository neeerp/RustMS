//! The beginner skills on Snail Hunting Ground I (40000): Three Snails attacks
//! through CLOSE_RANGE_ATTACK, and Recovery and Nimble Feet are buffs from
//! USE_SKILL. Each spends its MP, and the map sees it used.

use integration_harness::hunting::{attack, skill_attack, spend_sp, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const FOREST: i32 = 40_000;
const SNAIL: i32 = 100100;
const THREE_SNAILS: i32 = 1000;
const RECOVERY: i32 = 1001;
const NIMBLE_FEET: i32 = 1002;
/// The client's CANCEL_BUFF bits (second mask).
const SPEED_MASK: i64 = 0x80_0000_0000;
const RECOVERY_MASK: i64 = 0x4;

fn touched(damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(SNAIL).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// Kill these five snails (15 EXP) to reach level 2, and spend its SP.
async fn learn(s: &mut WorldSession, t: &mut Trace, snails: &[i32], skill_id: i32) {
    for &snail in snails {
        send(&mut s.connection, attack(snail, 8)).await;
    }
    until(&mut s.connection, t, |t| t.stat(stat::LEVEL) == Some(2)).await;
    send(&mut s.connection, spend_sp(skill_id)).await;
    until(&mut s.connection, t, |t| t.skills.contains(&(skill_id, 1))).await;
}

#[tokio::test]
async fn beginner_skills_spend_mp_and_take_effect() {
    let mut sessions = Vec::new();
    let mut traces = Vec::new();
    for _ in 0..3 {
        let config = load_harness_config_or_fail().await;
        let mut session = login_to_world_session(&config).await.unwrap();
        let mut t = Trace::default();
        assert_eq!(
            warp(&mut session.connection, FOREST, "", &mut t).await,
            FOREST
        );
        sessions.push(session);
        traces.push(t);
    }
    let [a, b, c] = &mut sessions[..] else {
        unreachable!()
    };
    let [ta, tb, tc] = &mut traces[..] else {
        unreachable!()
    };
    // Earlier tests may have hunted here; wait for enough to respawn.
    until(&mut a.connection, ta, |t| t.field.alive(SNAIL).len() >= 15).await;
    let snails = ta.field.alive(SNAIL);
    learn(a, ta, &snails[0..5], THREE_SNAILS).await;
    learn(b, tb, &snails[5..10], NIMBLE_FEET).await;
    learn(c, tc, &snails[10..15], RECOVERY).await;

    barrier(&mut a.connection, "all snails hunted", ta).await;

    // Three Snails at level 1 does exactly 10, for 3 of a new beginner's 5 MP.
    until(&mut a.connection, ta, |t| !t.field.alive(SNAIL).is_empty()).await;
    let target = ta.field.alive(SNAIL)[0];
    send(&mut a.connection, skill_attack(THREE_SNAILS, target, 11)).await;
    send(&mut a.connection, skill_attack(RECOVERY, target, 1)).await;
    barrier(&mut a.connection, "refused", ta).await;
    // Still the 5 MP from the level-up update.
    assert_eq!(ta.stat(stat::MP), Some(5));
    send(&mut a.connection, skill_attack(THREE_SNAILS, target, 10)).await;
    until(&mut a.connection, ta, |t| t.field.killed.contains(&target)).await;
    assert_eq!(ta.stat(stat::MP), Some(2));
    until(&mut b.connection, tb, |t| {
        t.field
            .attack_skills
            .contains(&(a.character_id, THREE_SNAILS, 1))
    })
    .await;

    // Nimble Feet: 4 MP, speed +10 for 4 seconds, then a minute's cooldown.
    send(&mut b.connection, use_skill(NIMBLE_FEET, 2)).await;
    barrier(&mut b.connection, "wrong level", tb).await;
    assert!(tb.buffs.is_empty());
    send(&mut b.connection, use_skill(NIMBLE_FEET, 1)).await;
    until(&mut b.connection, tb, |t| !t.cooldowns.is_empty()).await;
    assert_eq!(tb.stat(stat::MP), Some(1));
    assert_eq!(tb.buffs, [(NIMBLE_FEET, 10, 4000)]);
    assert_eq!(tb.cooldowns, [(NIMBLE_FEET, 60)]);
    until(&mut a.connection, ta, |t| {
        t.field.effects.contains(&(b.character_id, 1))
    })
    .await;
    send(&mut b.connection, use_skill(NIMBLE_FEET, 1)).await;
    until(&mut b.connection, tb, |t| t.cancelled_buffs == [SPEED_MASK]).await;
    assert_eq!(tb.buffs.len(), 1);

    // Recovery: 5 MP, then 4 HP every 5 seconds while hurt.
    let hurt = tc.stat(stat::HP).unwrap() - 12;
    send(&mut c.connection, touched(12)).await;
    until(&mut c.connection, tc, |t| t.stat(stat::HP) == Some(hurt)).await;
    send(&mut c.connection, use_skill(RECOVERY, 1)).await;
    until(&mut c.connection, tc, |t| !t.buffs.is_empty()).await;
    assert_eq!(tc.buffs, [(RECOVERY, 4, 30_000)]);
    assert_eq!(tc.stat(stat::MP), Some(0));
    until(&mut c.connection, tc, |t| {
        t.stat(stat::HP) == Some(hurt + 4)
    })
    .await;
    // Dying ends it.
    for _ in 0..4 {
        send(&mut c.connection, touched(12)).await;
    }
    until(&mut c.connection, tc, |t| t.stat(stat::HP) == Some(0)).await;
    until(&mut c.connection, tc, |t| {
        t.cancelled_buffs == [RECOVERY_MASK]
    })
    .await;
    read_for(&mut c.connection, tc, 300).await;
}
