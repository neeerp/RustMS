//! Magic Guard takes its share of a mob's touch from MP instead of HP.

use integration_harness::database::{grant_skill, set_hp, set_job};
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
