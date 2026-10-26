//! Event mounts (report b183): a GM teaches the Spaceship with `!skill`,
//! rides it with no mount worn, gaining its defense, and the map sees the
//! vehicle of its level. Getting off ends the defense too.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{set_gm_level, set_job};
use integration_harness::hunting::{cancel_buff, use_skill};
use integration_harness::packets::build_all_chat;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const HENESYS: i32 = 100_000_000;
const SPACESHIP: i32 = 1013;
/// The Spaceship's vehicle at level 2.
const SPACESHIP_2: i32 = 1_932_002;
/// Riding's flag, in the first half of a buff mask.
const RIDING: i64 = 1 << 53;
/// Weapon and magic defense, in the second half.
const DEFENSE: i64 = (1 << 33) | (1 << 35);

/// Read until `pick` takes a packet about `who`'s buffs, failing after 5
/// seconds.
async fn expect<T>(
    c: &mut MapleTestConnection,
    op: SendOpcode,
    who: i32,
    pick: impl Fn(Cursor<&Vec<u8>>) -> T,
) -> T {
    let op = op as i16;
    loop {
        let p: Packet = timeout(Duration::from_secs(5), c.read_packet("event mounts"))
            .await
            .expect("timed out")
            .expect("read packet")
            .packet;
        if p.opcode() != op {
            continue;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        if r.read_int().unwrap() == who {
            return pick(r);
        }
    }
}

#[tokio::test]
async fn a_gm_taught_spaceship_rides_its_own_vehicle() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    let watcher_config = load_harness_config_or_fail().await;
    login_to_world_session(&watcher_config)
        .await
        .expect("watcher's first login failed");
    drop(relogin(&watcher_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 2).await.expect("make a GM");
    set_job(id, 0, 10, 100).await.expect("give MP");

    let mut gm = relogin(&config).await;
    let mut watcher = relogin(&watcher_config).await;
    let (g, w) = (&mut gm.connection, &mut watcher.connection);
    let m = config.character_name.clone();
    let (mut gt, mut wt) = (Trace::default(), Trace::default());
    assert_eq!(warp(g, HENESYS, "", &mut gt).await, HENESYS);
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);

    // `!skill` teaches it at its highest level, 2.
    let t = step(g, &m, build_all_chat("!skill 1013", 0).unwrap()).await;
    assert_eq!(t.skills, [(SPACESHIP, 2)]);

    // Riding it costs its MP and gives level 2's defense for its time.
    let t = step(g, &m, use_skill(SPACESHIP, 2)).await;
    assert_eq!(t.stat_values(stat::MP), [100 - 10]);
    assert_eq!(t.buffs, [(SPACESHIP, 20, 2_100_000_000)]);
    let seen = expect(w, SendOpcode::GiveForeignBuff, id, |mut r| {
        assert_eq!(r.read_long().unwrap(), RIDING);
        r.read_long().unwrap();
        r.read_short().unwrap();
        (r.read_int().unwrap(), r.read_int().unwrap())
    })
    .await;
    assert_eq!(seen, (SPACESHIP_2, SPACESHIP));

    // Getting off ends the defense too, and the map sees it.
    let t = step(g, &m, cancel_buff(SPACESHIP)).await;
    assert_eq!(t.cancelled_buffs, [DEFENSE]);
    let off = expect(w, SendOpcode::CancelForeignBuff, id, |mut r| {
        r.read_long().unwrap()
    })
    .await;
    assert_eq!(off, RIDING);
}
