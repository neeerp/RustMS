//! A morph item (report 225): 2210000 turns its user into Morph.nx
//! template 1 for its `time`. The map sees the morph
//! (GIVE_FOREIGN_BUFF, and SPAWN_PLAYER for someone coming in), and
//! right-clicking the icon ends it for everyone. Another morph item waits
//! until it ends (report c012).

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::hunting::{cancel_item_effect, use_item};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const LITH_HARBOR: i32 = 104_000_000;
const HENESYS: i32 = 100_000_000;
const MORPH_ITEM: i32 = 2_210_000;
/// Morph's flag (bit 33), in the second half of a buff mask.
const MORPH: i64 = 2;

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("morph"))
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

/// A foreign buff packet's (character, morph mask) when it is Morph's.
fn foreign(p: &Packet, op: SendOpcode) -> Option<i32> {
    let mut r = body(p, op)?;
    let who = r.read_int().unwrap();
    (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then_some(who)
}

#[tokio::test]
async fn a_morph_item_shows_to_the_map_until_cancelled() {
    let config = load_multi_harness_config_or_fail().await;
    let morpher_config = config.player("sender").unwrap();
    let watcher_config = config.player("recipient").unwrap();

    let morpher_id = login_to_world_session(&morpher_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&morpher_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(morpher_id, 2, 1, MORPH_ITEM, 2)
        .await
        .expect("grant the morph item");

    let mut morpher = relogin(&morpher_config).await;
    let mut m = Trace::default();
    assert_eq!(
        warp(&mut morpher.connection, LITH_HARBOR, "", &mut m).await,
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

    // The morph comes as a buff of the item, with template 1.
    send(&mut morpher.connection, use_item(1, MORPH_ITEM)).await;
    let given = expect(&mut morpher.connection, |p| {
        let mut r = body(p, SendOpcode::GiveBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH)
            .then(|| (r.read_short().unwrap(), r.read_int().unwrap()))
    })
    .await;
    assert_eq!(given, (1, -MORPH_ITEM));

    // The map sees it: template 1 comes first after the mask.
    let seen = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::GiveForeignBuff)?;
        let who = r.read_int().unwrap();
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH)
            .then(|| (who, r.read_short().unwrap()))
    })
    .await;
    assert_eq!(seen, (morpher_id, 1));

    // A second one is refused while morphed, and kept.
    send(&mut morpher.connection, use_item(1, MORPH_ITEM)).await;
    expect(&mut morpher.connection, |p| {
        assert_ne!(p.opcode(), SendOpcode::GiveBuff as i16, "morphed again");
        assert_ne!(
            p.opcode(),
            SendOpcode::InventoryOperation as i16,
            "the second morph item was taken"
        );
        let r = body(p, SendOpcode::StatChange)?;
        (r.get_ref()[2..] == [1, 0, 0, 0, 0]).then_some(())
    })
    .await;

    // So does someone coming onto the map.
    assert_eq!(
        warp(&mut watcher.connection, HENESYS, "", &mut w).await,
        HENESYS
    );
    assert_eq!(
        warp(&mut watcher.connection, LITH_HARBOR, "", &mut w).await,
        LITH_HARBOR
    );
    let spawned = expect(&mut watcher.connection, |p| {
        let r = body(p, SendOpcode::SpawnPlayer)?;
        let bytes = &r.get_ref()[2..];
        (bytes[..4] == morpher_id.to_le_bytes()).then(|| {
            // The mask's first half carries the two-state stats here.
            let morph = [&MORPH.to_le_bytes()[..], &1i16.to_le_bytes()].concat();
            bytes.windows(morph.len()).any(|w| w == morph)
        })
    })
    .await;
    assert!(spawned, "the spawn carries the morph");

    // Right-clicking the icon ends it, for the map too.
    send(&mut morpher.connection, cancel_item_effect(-MORPH_ITEM)).await;
    expect(&mut morpher.connection, |p| {
        let mut r = body(p, SendOpcode::CancelBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then_some(())
    })
    .await;
    let off = expect(&mut watcher.connection, |p| {
        foreign(p, SendOpcode::CancelForeignBuff)
    })
    .await;
    assert_eq!(off, morpher_id);
}
