//! The Maplemas Party Potion (2212000), sent through v83's Transform dialog
//! (USE_TARGET_ITEM), morphs the character it names into one of its pool
//! (23-30) for an hour. The user must be in a town; a name not on the
//! channel keeps the potion (report c041, c60 defaults).

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::hunting::use_target_item;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const PARTY_POTION: i32 = 2_212_000;
const HENESYS: i32 = 100_000_000;
/// Henesys Hunting Ground I, not a town.
const HUNTING_GROUND: i32 = 100_010_000;
/// Morph's flag (bit 33), in the second half of a buff mask.
const MORPH: i64 = 2;

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("party potion"))
            .await
            .expect("timed out")
            .expect("read packet")
            .packet;
        if let Some(picked) = pick(&p) {
            return picked;
        }
    }
}

/// The body of RANDOM_MORPH_RES, the user's answer; `p` is no INVENTORY_OPERATION.
fn answer(p: &Packet) -> Option<Vec<u8>> {
    assert_ne!(
        p.opcode(),
        SendOpcode::InventoryOperation as i16,
        "the potion was taken"
    );
    (p.opcode() == SendOpcode::RandomMorphRes as i16).then(|| p.bytes[2..].to_vec())
}

#[tokio::test]
async fn a_party_potion_morphs_the_character_it_names_from_a_town() {
    let user_config = load_harness_config_or_fail().await;
    let user_id = login_to_world_session(&user_config)
        .await
        .expect("first login failed")
        .character_id;
    let target_config = load_harness_config_or_fail().await;
    drop(
        login_to_world_session(&target_config)
            .await
            .expect("second login failed"),
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(user_id, 2, 1, PARTY_POTION, 2)
        .await
        .expect("party potions");
    let name = target_config.character_name.clone();

    let mut target = relogin(&target_config).await;
    let mut user = relogin(&user_config).await;
    let u = &mut user.connection;
    let mut trace = Trace::default();

    // Out of town.
    assert_eq!(
        warp(u, HUNTING_GROUND, "", &mut trace).await,
        HUNTING_GROUND
    );
    send(u, use_target_item(1, PARTY_POTION, &name)).await;
    assert_eq!(expect(u, answer).await, [1, 3]);

    // In town, a name no one on the channel has.
    assert_eq!(warp(u, HENESYS, "", &mut trace).await, HENESYS);
    send(u, use_target_item(1, PARTY_POTION, "nobody")).await;
    let mut not_found = vec![1, 2, 6, 0];
    not_found.extend(b"nobody");
    assert_eq!(expect(u, answer).await, not_found);

    // The target is morphed; the user's potion is taken.
    send(u, use_target_item(1, PARTY_POTION, &name)).await;
    let quantity = expect(u, |p| {
        (p.opcode() == SendOpcode::InventoryOperation as i16).then(|| {
            // Unlocked, one change: Quantity (1), USE, slot 1, then the count.
            assert_eq!(p.bytes[2..8], [1, 1, 1, 2, 1, 0][..]);
            i16::from_le_bytes([p.bytes[8], p.bytes[9]])
        })
    })
    .await;
    assert_eq!(quantity, 1);
    assert_eq!(
        expect(u, |p| (p.opcode() == SendOpcode::RandomMorphRes as i16)
            .then(|| p.bytes[2..].to_vec()))
        .await,
        [0]
    );
    let (morph, reason, time) = expect(&mut target.connection, |p| {
        if p.opcode() != SendOpcode::GiveBuff as i16 {
            return None;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then(|| {
            (
                r.read_short().unwrap(),
                r.read_int().unwrap(),
                r.read_int().unwrap(),
            )
        })
    })
    .await;
    assert!((23..=30).contains(&morph), "template {morph}");
    assert_eq!((reason, time), (-PARTY_POTION, 3_600_000));
}
