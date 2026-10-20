//! Battleship (report 228): a Corsair boards the ship whatever mount they
//! wear, gaining its defense, and the map sees them on it. Hits wear the
//! ship down, it stays damaged between rides, and once it sinks the
//! Corsair is put off it and has to wait out its cooldown.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, set_hp, set_job, wear_equip};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const HENESYS: i32 = 100_000_000;
const CORSAIR: i16 = 522;
const BATTLESHIP: i32 = 5221006;
const SHIP: i32 = 1932000;
const SHIP_HP: i32 = 5221999;
const HOG: i32 = 1902000;
const SADDLE: i32 = 1912000;
const STUMPY: i32 = 3_220_000;
/// Riding's flag, in the first half of a buff mask.
const RIDING: i64 = 1 << 53;
/// Weapon and magic defense, in the second half.
const DEFENSE: i64 = (1 << 33) | (1 << 35);

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("battleship"))
            .await
            .expect("timed out")
            .expect("read packet")
            .packet;
        if let Some(picked) = pick(&p) {
            return picked;
        }
    }
}

/// The body of `p` after the opcode and character id if it is an `op`
/// about `who`.
fn about(p: &Packet, op: SendOpcode, who: i32) -> Option<Cursor<&Vec<u8>>> {
    if p.opcode() != op as i16 {
        return None;
    }
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    (r.read_int().unwrap() == who).then_some(r)
}

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

fn cancel_buff(skill_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CancelBuff as i16).unwrap();
    p.write_int(skill_id).unwrap();
    p
}

#[tokio::test]
async fn the_battleship_wears_down_and_sinks() {
    let config = load_multi_harness_config_or_fail().await;
    let corsair_config = config.player("sender").unwrap();
    let watcher_config = config.player("recipient").unwrap();
    let m = corsair_config.character_name.clone();

    // A level 120 Corsair with Battleship 1, whose ship holds 400 HP, and
    // a Hog worn that it must not ride.
    let id = login_to_world_session(&corsair_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&corsair_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, CORSAIR, 120, 1000)
        .await
        .expect("make a corsair");
    set_hp(id, 5000, 5000).await.expect("full HP");
    grant_skill(id, BATTLESHIP, 1)
        .await
        .expect("teach Battleship");
    wear_equip(id, -18, HOG).await.expect("wear the Hog");
    wear_equip(id, -19, SADDLE).await.expect("saddle it");

    let mut corsair = relogin(&corsair_config).await;
    let c = &mut corsair.connection;
    let mut r = Trace::default();
    assert_eq!(warp(c, HENESYS, "", &mut r).await, HENESYS);
    let mut watcher = login_to_world_session(&watcher_config)
        .await
        .expect("watcher login failed");
    let w = &mut watcher.connection;
    let mut wt = Trace::default();
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);

    // Boarding gives the defense with the ride, then the ship's HP; the
    // cooldown waits for the ship to sink.
    let t = step(c, &m, use_skill(BATTLESHIP, 1)).await;
    assert_eq!(t.stat_values(stat::MP), [1000 - 22]);
    assert_eq!(t.buffs, [(BATTLESHIP, 10, 2_100_000_000)]);
    assert_eq!(t.cooldowns, [(SHIP_HP, 400)]);
    let seen = expect(w, |p| {
        let mut r = about(p, SendOpcode::GiveForeignBuff, id)?;
        assert_eq!(r.read_long().unwrap(), RIDING);
        r.read_long().unwrap();
        r.read_short().unwrap();
        Some((r.read_int().unwrap(), r.read_int().unwrap()))
    })
    .await;
    assert_eq!(seen, (SHIP, BATTLESHIP));

    // A hit takes the player's HP and the ship's.
    let t = step(c, &m, touched(100, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [4900]);
    assert_eq!(t.cooldowns, [(SHIP_HP, 300)]);

    // Getting off ends the defense too; the ship comes back as damaged.
    let t = step(c, &m, cancel_buff(BATTLESHIP)).await;
    assert_eq!(t.cancelled_buffs, [DEFENSE]);
    let off = expect(w, |p| {
        let mut r = about(p, SendOpcode::CancelForeignBuff, id)?;
        Some(r.read_long().unwrap())
    })
    .await;
    assert_eq!(off, RIDING);
    let t = step(c, &m, use_skill(BATTLESHIP, 1)).await;
    assert_eq!(t.cooldowns, [(SHIP_HP, 300)]);

    // It sinks at 0: its cooldown starts and the player is put off it.
    for left in [200, 100] {
        let t = step(c, &m, touched(100, STUMPY)).await;
        assert_eq!(t.cooldowns, [(SHIP_HP, left)]);
    }
    let t = step(c, &m, touched(100, STUMPY)).await;
    assert_eq!(t.cooldowns, [(BATTLESHIP, 90)]);
    assert_eq!(t.cancelled_buffs, [DEFENSE]);

    // Until it cools down, boarding is refused with riding's cancel.
    let t = step(c, &m, use_skill(BATTLESHIP, 1)).await;
    assert!(t.buffs.is_empty());
    assert_eq!(t.cancelled_buffs, [0]);
}
