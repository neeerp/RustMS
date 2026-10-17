//! Player diseases: a mob's disease skill reaches the players in its
//! area. A Slow cast by a Korean Folk Town mob (126 level 1: x 80, ten
//! seconds, sure to land) slows the controller standing on it, and not
//! one standing far away; the server ends it when it runs out.

use integration_harness::connection::MapleTestConnection;
use integration_harness::hunting::walk_to;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::{sleep, Duration};

const FOLK_TOWN_MAP: i32 = 222_010_201;
const SLOWER: i32 = 5_100_005;
const SLOW: u8 = 126;
/// Slow is flag 32, the low bit of the mask's second long, whose ints go
/// high half first.
const SLOW_STAT: i64 = 1;

/// MOVE_LIFE for `object_id` at (x, y): wanting to attack, or playing
/// `action` with `skill_data` (skill | level << 8 | delay << 16).
fn move_mob(
    object_id: i32,
    (x, y): (i16, i16),
    want_attack: bool,
    action: u8,
    skill_data: i32,
) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MoveLife as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_short(1).unwrap();
    p.write_byte(u8::from(want_attack)).unwrap();
    p.write_byte(action).unwrap();
    p.write_int(skill_data).unwrap();
    p.write_bytes(&[0; 13]).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_byte(1).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_bytes(&[0; 6]).unwrap();
    p.write_byte(4).unwrap();
    p.write_short(100).unwrap();
    p
}

/// Grant the mob its one skill, then cast it from its first skill
/// animation.
async fn slow(c: &mut MapleTestConnection, t: &mut Trace, mob: i32, at: (i16, i16)) {
    let granted = t.field.grants.len();
    send(c, move_mob(mob, at, true, 0, 0)).await;
    until(c, t, |t| t.field.grants.len() > granted).await;
    assert_eq!(t.field.grants.last(), Some(&(mob, SLOW, 1)));
    send(
        c,
        move_mob(mob, at, false, 21 << 1, i32::from(SLOW) | 1 << 8),
    )
    .await;
}

#[tokio::test]
async fn a_mobs_slow_reaches_only_the_players_in_its_area() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let c = &mut player.connection;

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

    // Out of its 300 px reach, the cast misses.
    send(c, walk_to(at.0 + 1_000, at.1)).await;
    slow(c, &mut t, mob, at).await;
    read_for(c, &mut t, 500).await;
    assert!(t.buffs.is_empty());

    // On top of the mob, after its five second cooldown, it lands: 80
    // speed for ten seconds, from mob skill 126 level 1.
    sleep(Duration::from_secs(5)).await;
    send(c, walk_to(at.0, at.1)).await;
    slow(c, &mut t, mob, at).await;
    until(c, &mut t, |t| !t.buffs.is_empty()).await;
    assert_eq!(t.buffs, [(i32::from(SLOW) | 1 << 16, 80, 10_000)]);

    // The server ends it once its time is up.
    until(c, &mut t, |t| !t.cancelled_buffs.is_empty()).await;
    assert_eq!(t.cancelled_buffs, [SLOW_STAT]);
}
