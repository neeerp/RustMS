//! Magic Guard takes its share of a mob's touch from MP instead of HP, and
//! Power Guard sends some of it back at the mob.

use integration_harness::database::{grant_skill, set_dex, set_hp, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{stat, step};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MAGICIAN: i16 = 200;
const MAGIC_GUARD: i32 = 2001002;
const FIGHTER: i16 = 110;
const POWER_GUARD: i32 = 1101007;
const STUMPY: i32 = 3_220_000;

/// TAKE_DAMAGE for `mob_id`'s touch, with no guard or stance after it.
fn touched(damage: i32, mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// `touched`, with the Power Guard block sending `percent` back.
fn touched_back(damage: i32, mob_id: i32, percent: u8) -> Packet {
    let mut p = touched(damage, mob_id);
    p.write_byte(percent).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(1).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(6).unwrap();
    for _ in 0..4 {
        p.write_short(0).unwrap();
    }
    p.write_byte(0).unwrap();
    p
}

#[tokio::test]
async fn magic_guard_pays_most_of_a_touch_in_mp() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, MAGICIAN, 30, 500)
        .await
        .expect("make a magician");
    set_hp(id, 500, 500).await.expect("full HP");
    grant_skill(id, MAGIC_GUARD, 20)
        .await
        .expect("teach Magic Guard");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // Without the guard a touch is all HP.
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [480]);
    assert!(t.stat_values(stat::MP).is_empty());

    let t = step(c, &m, use_skill(MAGIC_GUARD, 20)).await;
    let mp = *t.stat_values(stat::MP).last().expect("the cast costs MP");

    // At level 20 it takes 80%: 16 of 20 from MP, 4 from HP.
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [476]);
    assert_eq!(t.stat_values(stat::MP), [mp - 16]);
}

#[tokio::test]
async fn power_guard_sends_some_of_a_touch_back() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, FIGHTER, 30, 500).await.expect("make a fighter");
    set_hp(id, 500, 500).await.expect("full HP");
    // Enough accuracy that Stumpy, five levels up, cannot dodge it.
    set_dex(id, 100).await.expect("raise DEX");
    grant_skill(id, POWER_GUARD, 30)
        .await
        .expect("teach Power Guard");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    step(c, &m, use_skill(POWER_GUARD, 30)).await;

    // 40% of 40 is 16, halved for a boss: 8 of the 40 never lands.
    let t = step(c, &m, touched_back(40, STUMPY, 40)).await;
    assert_eq!(t.stat_values(stat::HP), [468]);

    // Without the block the client saw no Power Guard, so none applies.
    let t = step(c, &m, touched(40, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [428]);
}
