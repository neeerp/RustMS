//! A ghost item (report c039): 2360000 makes its user a ghost (Ghost,
//! bit 49) for its `time`. The map sees it, another ghost or morph item
//! waits until it ends, and right-clicking the icon ends it for everyone.
//! Who comes to the map later sees the ghost, a map change keeps it, and
//! dying ends it.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_hp};
use integration_harness::hunting::{cancel_item_effect, use_item};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const LITH_HARBOR: i32 = 104_000_000;
const HENESYS: i32 = 100_000_000;
const SNAIL: i32 = 100_100;
const GHOST_ITEM: i32 = 2_360_000;
/// Ghost's flag (bit 49), in the second half of a buff mask.
const GHOST: i64 = 1 << 17;

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("ghost"))
            .await
            .expect("timed out")
            .expect("read packet")
            .packet;
        if let Some(picked) = pick(&p) {
            return picked;
        }
    }
}

/// The body of `p` if it is an `op`.
fn body(p: &Packet, op: SendOpcode) -> Option<Cursor<&Vec<u8>>> {
    if p.opcode() != op as i16 {
        return None;
    }
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    Some(r)
}

/// TAKE_DAMAGE for a snail's touch.
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

#[tokio::test]
async fn a_ghost_item_shows_to_the_map_until_cancelled() {
    let config = load_multi_harness_config_or_fail().await;
    let ghost_config = config.player("sender").unwrap();
    let watcher_config = config.player("recipient").unwrap();

    let ghost_id = login_to_world_session(&ghost_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&ghost_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(ghost_id, 2, 1, GHOST_ITEM, 2)
        .await
        .expect("grant the ghost item");
    set_hp(ghost_id, 1, 50).await.expect("1 HP");

    let mut ghost = relogin(&ghost_config).await;
    let mut g = Trace::default();
    assert_eq!(
        warp(&mut ghost.connection, LITH_HARBOR, "", &mut g).await,
        LITH_HARBOR
    );
    let mut watcher = login_to_world_session(&watcher_config)
        .await
        .expect("watcher login failed");
    let mut w = Trace::default();
    assert_eq!(
        warp(&mut watcher.connection, LITH_HARBOR, "", &mut w).await,
        LITH_HARBOR
    );

    // The ghost comes as a buff of the item, with its value 1.
    send(&mut ghost.connection, use_item(1, GHOST_ITEM)).await;
    let given = expect(&mut ghost.connection, |p| {
        let mut r = body(p, SendOpcode::GiveBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST)
            .then(|| (r.read_short().unwrap(), r.read_int().unwrap()))
    })
    .await;
    assert_eq!(given, (1, -GHOST_ITEM));

    let seen = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::GiveForeignBuff)?;
        let who = r.read_int().unwrap();
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST)
            .then(|| (who, r.read_short().unwrap()))
    })
    .await;
    assert_eq!(seen, (ghost_id, 1));

    // A second one is refused while a ghost, and kept.
    send(&mut ghost.connection, use_item(1, GHOST_ITEM)).await;
    expect(&mut ghost.connection, |p| {
        assert_ne!(p.opcode(), SendOpcode::GiveBuff as i16, "a ghost again");
        assert_ne!(
            p.opcode(),
            SendOpcode::InventoryOperation as i16,
            "the second ghost item was taken"
        );
        let r = body(p, SendOpcode::StatChange)?;
        (r.get_ref()[2..] == [1, 0, 0, 0, 0]).then_some(())
    })
    .await;

    // Right-clicking the icon ends it, for the map too.
    send(&mut ghost.connection, cancel_item_effect(-GHOST_ITEM)).await;
    expect(&mut ghost.connection, |p| {
        let mut r = body(p, SendOpcode::CancelBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST).then_some(())
    })
    .await;
    let off = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::CancelForeignBuff)?;
        let who = r.read_int().unwrap();
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST).then_some(who)
    })
    .await;
    assert_eq!(off, ghost_id);

    // Someone coming to the map sees a ghost in its spawn: the Ghost flag,
    // and its value, the only shown buff's data.
    send(&mut ghost.connection, use_item(1, GHOST_ITEM)).await;
    expect(&mut ghost.connection, |p| {
        let mut r = body(p, SendOpcode::GiveBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST).then_some(())
    })
    .await;
    assert_eq!(
        warp(&mut watcher.connection, HENESYS, "", &mut w).await,
        HENESYS
    );
    assert_eq!(
        warp(&mut watcher.connection, LITH_HARBOR, "", &mut w).await,
        LITH_HARBOR
    );
    let spawned = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::SpawnPlayer)?;
        (r.read_int().unwrap() == ghost_id).then_some(())?;
        r.read_byte().unwrap(); // level
        r.read_str_with_length().unwrap();
        r.read_str_with_length().unwrap(); // guild
        r.set_position(r.position() + 6); // emblem
        r.read_long().unwrap(); // the two-state flags
        let shown = r.read_long().unwrap();
        Some((shown & GHOST, r.read_short().unwrap()))
    })
    .await;
    assert_eq!(spawned, (GHOST, 1));

    // The ghost's own client makes them anew in each map, so after a map
    // change it is told again what is left of the ghost (report c044).
    for map in [HENESYS, LITH_HARBOR] {
        assert_eq!(warp(&mut ghost.connection, map, "", &mut g).await, map);
        let again = expect(&mut ghost.connection, |p| {
            let mut r = body(p, SendOpcode::GiveBuff)?;
            (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST).then(|| {
                let value = r.read_short().unwrap();
                (value, r.read_int().unwrap(), r.read_int().unwrap())
            })
        })
        .await;
        assert_eq!((again.0, again.1), (1, -GHOST_ITEM));
        assert!(
            (3_500_000..3_600_000).contains(&again.2),
            "{} ms of an hour's ghost left",
            again.2
        );
    }

    // Dying ends it, for the map too.
    send(&mut ghost.connection, touched(1)).await;
    expect(&mut ghost.connection, |p| {
        let mut r = body(p, SendOpcode::CancelBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST).then_some(())
    })
    .await;
    let off = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::CancelForeignBuff)?;
        let who = r.read_int().unwrap();
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == GHOST).then_some(who)
    })
    .await;
    assert_eq!(off, ghost_id);

    // Back to life in town.
    warp(&mut ghost.connection, LITH_HARBOR, "", &mut g).await;
}
