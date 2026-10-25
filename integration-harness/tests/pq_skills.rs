//! The secret skills every job has at level 1 in their own maps
//! (report 229): Invincibility (1010) works only in the Dojo, where it
//! keeps every mob's touch off the player, and nowhere else. There each
//! takes a full energy bar, built by attacking and being hit, and empties
//! it.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_hp;
use integration_harness::hunting::{attack, cancel_buff, skill_attack, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const BAMBOO_RAIN: i32 = 1009;
const INVINCIBILITY: i32 = 1010;
const POWER_EXPLOSION: i32 = 1011;
const PINK: u8 = 5;
const DOJO: i32 = 925_020_100;
const ENTRANCE: i32 = 925_020_000;
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

fn energy(value: &str) -> (String, String) {
    ("energy".to_string(), value.to_string())
}

/// A hundred attacks that each reach a monster fill the energy bar.
async fn charge(c: &mut MapleTestConnection, marker: &str) -> Trace {
    for _ in 0..99 {
        send(c, attack(1, 1)).await;
    }
    step(c, marker, attack(1, 1)).await
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

    charge(c, &m).await;
    let t = step(c, &m, use_skill(INVINCIBILITY, 1)).await;
    assert_eq!(t.buffs, [DIVINE_BODY]);
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert!(
        t.stat_values(stat::HP).iter().all(|&hp| hp == 480),
        "a touch went through"
    );

    // Without it the next touch hurts again, from the HP the blocked one
    // left alone.
    step(c, &m, cancel_buff(INVINCIBILITY)).await;
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [460]);
}

#[tokio::test]
async fn the_dojos_secret_skills_take_a_full_energy_bar_and_empty_it() {
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

    // Out of the Dojo neither attacks nor hits build energy.
    assert_eq!(warp(c, TOWN, "", &mut trace).await, TOWN);
    let t = step(c, &m, attack(1, 1)).await;
    assert!(t.session_values.is_empty(), "{t:?}");

    // In it, a hit taken adds 20 and an attack 100, up to 10000; the
    // client hears each.
    assert_eq!(warp(c, DOJO, "", &mut trace).await, DOJO);
    read_for(c, &mut trace, 300).await;
    let t = step(c, &m, use_skill(INVINCIBILITY, 1)).await;
    assert!(t.buffs.is_empty(), "cast without energy");
    let t = step(c, &m, skill_attack(BAMBOO_RAIN, 1, 1)).await;
    assert!(t.session_values.is_empty(), "Bamboo Rain without energy");
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.session_values, [energy("20")]);
    let t = step(c, &m, attack(1, 1)).await;
    assert_eq!(t.session_values, [energy("120")]);
    let t = charge(c, &m).await;
    assert_eq!(t.session_values.last(), Some(&energy("10000")));

    // Full, Bamboo Rain empties it, with a notice.
    let reset = (PINK, net::content::text().get(keys::DOJO_ENERGY_RESET, &[]));
    let t = step(c, &m, skill_attack(BAMBOO_RAIN, 1, 1)).await;
    assert_eq!(t.session_values, [energy("0")]);
    assert!(t.broadcasts.contains(&reset), "{t:?}");

    // And so does Invincibility.
    charge(c, &m).await;
    let t = step(c, &m, use_skill(INVINCIBILITY, 1)).await;
    assert_eq!(t.buffs, [DIVINE_BODY]);
    assert_eq!(t.session_values, [energy("0")]);
    assert!(t.broadcasts.contains(&reset), "{t:?}");

    // And so does Power Explosion, whose buff is its `x`, 200 percent.
    charge(c, &m).await;
    let t = step(c, &m, use_skill(POWER_EXPLOSION, 1)).await;
    assert_eq!(t.buffs, [(POWER_EXPLOSION, 200, 30_000)]);
    assert_eq!(t.session_values, [energy("0")]);

    // Back at the entrance, a full bar empties too, and the client hears it.
    charge(c, &m).await;
    let mut t = Trace::default();
    assert_eq!(warp(c, ENTRANCE, "", &mut t).await, ENTRANCE);
    read_for(c, &mut t, 300).await;
    assert_eq!(t.session_values, [energy("0")]);
}
