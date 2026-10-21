//! The secret skills every job has at level 1 in their own maps
//! (report 229): Invincibility (1010) works only in the Dojo, where it
//! keeps every mob's touch off the player, and nowhere else.

use integration_harness::database::set_hp;
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const INVINCIBILITY: i32 = 1010;
const DOJO: i32 = 925_020_100;
const PYRAMID: i32 = 926_010_100;
const TOWN: i32 = 100_000_000;
const STUMPY: i32 = 3_220_000;
/// Its `x` of 1 for its `time` of 30 seconds.
const DIVINE_BODY: (i32, i16, i32) = (INVINCIBILITY, 1, 30_000);

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
async fn invincibility_works_only_in_the_dojo_and_stops_every_touch() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(id, 500, 500).await.expect("full HP");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // No skill record, and not a map of its own: nothing happens.
    for map in [TOWN, PYRAMID] {
        assert_eq!(warp(c, map, "", &mut trace).await, map);
        let t = step(c, &m, use_skill(INVINCIBILITY, 1)).await;
        assert!(t.buffs.is_empty(), "cast on {map}");
    }

    assert_eq!(warp(c, DOJO, "", &mut trace).await, DOJO);
    read_for(c, &mut trace, 300).await;
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [480]);

    let t = step(c, &m, use_skill(INVINCIBILITY, 1)).await;
    assert_eq!(t.buffs, [DIVINE_BODY]);
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert!(
        t.stat_values(stat::HP).iter().all(|&hp| hp == 480),
        "a touch went through"
    );
}
