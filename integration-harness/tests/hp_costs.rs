//! Skills that cost HP: a Dragon Knight's Slash Blast takes its `hpCon`,
//! Dragon Roar a share of max HP but only from a player with half of it,
//! then stuns them for a moment, and Dragon Blood drinks HP every four
//! seconds until it would take the last.

use integration_harness::database::{grant_skill, replace_item, set_hp, set_job};
use integration_harness::hunting::{skill_attack, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::{Duration, Instant};

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const SLASH_BLAST: i32 = 1001005;
const DRAGON_ROAR: i32 = 1311006;
const DRAGON_BLOOD: i32 = 1311008;
const SPEAR: i32 = 1432000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
/// STAT_CHANGED's HP bit.
const HP: i32 = 0x400;
/// CANCEL_BUFF's Stun bit.
const STUN_MASK: i64 = 0x2_0000_0000_0000;

#[tokio::test]
async fn skills_spend_hp() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 131, 70, 1000).await.expect("dragon knight");
    set_hp(id, 1000, 1000).await.expect("full HP");
    for (skill, level) in [(SLASH_BLAST, 1), (DRAGON_ROAR, 30), (DRAGON_BLOOD, 2)] {
        grant_skill(id, skill, level).await.expect("teach a skill");
    }
    replace_item(id, EQUIPPED, WEAPON, SPEAR)
        .await
        .expect("give a spear");

    // Dragon Roar 30 takes 30% of max HP.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];
    let mut t = step(c, "roar", skill_attack(DRAGON_ROAR, snail, 1)).await;
    assert_eq!(t.stat(HP), Some(700));
    // And stuns its user for 2 seconds, under its own id.
    assert_eq!(t.buffs, [(DRAGON_ROAR, 1, 2000)]);
    until(c, &mut t, |t| t.cancelled_buffs.contains(&STUN_MASK)).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(id, 100, 1000).await.expect("low HP");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];

    // Under half of max HP, Dragon Roar is refused.
    let t = step(c, "roar", skill_attack(DRAGON_ROAR, snail, 1)).await;
    assert_eq!(t.stat(HP), None);
    // Slash Blast 1 costs 8.
    let t = step(c, "slash", skill_attack(SLASH_BLAST, snail, 1)).await;
    assert_eq!(t.stat(HP), Some(92));

    // Dragon Blood 2 drinks 46 every 4 seconds, and lasts 16, but ends
    // rather than drink the last 46.
    let mut t = step(c, "blood", use_skill(DRAGON_BLOOD, 2)).await;
    let used = Instant::now();
    until(c, &mut t, |t| !t.cancelled_buffs.is_empty()).await;
    assert!(used.elapsed() < Duration::from_secs(12), "it ended early");
    assert_eq!(t.stat_values(HP), [46]);
}
