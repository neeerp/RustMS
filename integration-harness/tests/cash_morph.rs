//! Cash morphs (report c012): the Fungus Scroll (5300000) turns its user
//! into Morph.nx template 1, shown to the map. Oinker Delight (5300001),
//! used while still a mushroom, makes them a pig instead, and
//! right-clicking the icon ends it for everyone.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::hunting::cancel_item_effect;
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const LITH_HARBOR: i32 = 104_000_000;
const CASH: i16 = 5;
const FUNGUS_SCROLL: i32 = 5_300_000;
const OINKER_DELIGHT: i32 = 5_300_001;
/// Morph's flag (bit 33), in the second half of a buff mask.
const MORPH: i64 = 2;

fn use_cash_item(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_int(0).unwrap();
    p
}

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("cash morph"))
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

/// The (template, buff id) of a GIVE_BUFF that is Morph's.
fn given(p: &Packet) -> Option<(i16, i32)> {
    let mut r = body(p, SendOpcode::GiveBuff)?;
    (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH)
        .then(|| (r.read_short().unwrap(), r.read_int().unwrap()))
}

/// The (character, template) of a GIVE_FOREIGN_BUFF that is Morph's.
fn seen(p: &Packet) -> Option<(i32, i16)> {
    let mut r = body(p, SendOpcode::GiveForeignBuff)?;
    let who = r.read_int().unwrap();
    (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH)
        .then(|| (who, r.read_short().unwrap()))
}

#[tokio::test]
async fn a_cash_morph_shows_and_gives_way_to_another() {
    let config = load_multi_harness_config_or_fail().await;
    let morpher_config = config.player("sender").unwrap();
    let watcher_config = config.player("recipient").unwrap();

    let morpher_id = login_to_world_session(&morpher_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&morpher_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(morpher_id, CASH, 1, FUNGUS_SCROLL, 1)
        .await
        .expect("grant the Fungus Scroll");
    grant_stack(morpher_id, CASH, 2, OINKER_DELIGHT, 1)
        .await
        .expect("grant Oinker Delight");

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

    // The scroll goes, and the morph comes as its buff, with template 1.
    send(&mut morpher.connection, use_cash_item(1, FUNGUS_SCROLL)).await;
    let gone = expect(&mut morpher.connection, |p| {
        let r = body(p, SendOpcode::InventoryOperation)?;
        Some(r.get_ref()[2..5].to_vec())
    })
    .await;
    assert_eq!(gone, [1, 1, 3], "one change, which removes the scroll");
    let mushroom = expect(&mut morpher.connection, given).await;
    assert_eq!(mushroom, (1, -FUNGUS_SCROLL));
    let shown = expect(&mut watcher.connection, seen).await;
    assert_eq!(shown, (morpher_id, 1));

    // Still a mushroom, the user may turn into a pig.
    send(&mut morpher.connection, use_cash_item(2, OINKER_DELIGHT)).await;
    let pig = expect(&mut morpher.connection, given).await;
    assert_eq!(pig, (2, -OINKER_DELIGHT));
    let shown = expect(&mut watcher.connection, seen).await;
    assert_eq!(shown, (morpher_id, 2));

    // Right-clicking the icon ends it, for the map too.
    send(&mut morpher.connection, cancel_item_effect(-OINKER_DELIGHT)).await;
    expect(&mut morpher.connection, |p| {
        let mut r = body(p, SendOpcode::CancelBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then_some(())
    })
    .await;
    let off = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::CancelForeignBuff)?;
        let who = r.read_int().unwrap();
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then_some(who)
    })
    .await;
    assert_eq!(off, morpher_id);
}
