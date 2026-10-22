//! Hero's Will (1121011 level 1: 30 MP, a 600 s cooldown) ends the
//! diseases on its caster. A Korean Folk Town mob's Slow (126 level 1, ten
//! seconds) is cured at once, and the skill can't be cast again during its
//! cooldown.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::{move_mob, use_skill, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::{sleep, Duration};

const FOLK_TOWN_MAP: i32 = 222_010_201;
const SLOWER: i32 = 5_100_005;
const SLOW: u8 = 126;
/// Slow is flag 32, the low bit of the mask's second long.
const SLOW_STAT: i64 = 1;
const HERO: i16 = 112;
const HEROS_WILL: i32 = 1_121_011;

/// Grant the mob its one skill, then cast it from its first skill
/// animation.
async fn slow(c: &mut MapleTestConnection, t: &mut Trace, mob: i32, at: (i16, i16)) {
    let granted = t.field.grants.len();
    send(c, move_mob(mob, at, true, 0, 0)).await;
    until(c, t, |t| t.field.grants.len() > granted).await;
    send(
        c,
        move_mob(mob, at, false, 21 << 1, i32::from(SLOW) | 1 << 8),
    )
    .await;
}

#[tokio::test]
async fn heros_will_cures_a_slow_once_per_cooldown() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    sleep(Duration::from_millis(500)).await;
    set_job(id, HERO, 120, 1000).await.expect("Hero");
    grant_skill(id, HEROS_WILL, 1).await.expect("Hero's Will");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, FOLK_TOWN_MAP, "", &mut t).await, FOLK_TOWN_MAP);
    let controls = |t: &Trace| {
        t.field
            .controlled
            .iter()
            .copied()
            .find(|oid| t.field.mobs.contains(&(*oid, SLOWER)))
    };
    until(c, &mut t, |t| controls(t).is_some()).await;
    let mob = controls(&t).unwrap();
    let at = t.field.mob_positions[&mob];
    send(c, walk_to(at.0, at.1)).await;
    slow(c, &mut t, mob, at).await;
    until(c, &mut t, |t| !t.buffs.is_empty()).await;
    assert_eq!(t.buffs, [(i32::from(SLOW) | 1 << 16, 80, 10_000)]);

    // The cast ends the Slow long before its ten seconds, costs 30 MP, and
    // starts the cooldown.
    send(c, use_skill(HEROS_WILL, 1)).await;
    until(c, &mut t, |t| {
        !t.cancelled_buffs.is_empty() && !t.cooldowns.is_empty()
    })
    .await;
    assert_eq!(t.cancelled_buffs, [SLOW_STAT]);
    assert_eq!(t.cooldowns, [(HEROS_WILL, 600)]);
    until(c, &mut t, |t| t.stat(stat::MP).is_some()).await;
    assert_eq!(t.stat(stat::MP), Some(970));

    // Slowed again, a second cast is refused: the Slow stays, no MP goes.
    sleep(Duration::from_secs(5)).await;
    slow(c, &mut t, mob, at).await;
    until(c, &mut t, |t| t.buffs.len() == 2).await;
    send(c, use_skill(HEROS_WILL, 1)).await;
    read_for(c, &mut t, 1_000).await;
    assert_eq!(t.cancelled_buffs, [SLOW_STAT]);
    assert_eq!(t.cooldowns, [(HEROS_WILL, 600)]);
    assert_eq!(t.stat_values(stat::MP), [970]);
}
