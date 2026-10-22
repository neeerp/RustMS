//! Cliff's Special Potion (2211000) morphs its user into one of the eight
//! templates of its `spec/morphRandom` pool (23-30), picked by weight, for
//! an hour. The Maplemas Party Potion (2212000) morphs someone else, so
//! USE_ITEM refuses it and keeps it (report c041).

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::hunting::{cancel_item_effect, use_item};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::send;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::collections::BTreeSet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const CLIFFS: i32 = 2_211_000;
const PARTY_POTION: i32 = 2_212_000;
const USES: i16 = 12;
/// Morph's flag (bit 33), in the second half of a buff mask.
const MORPH: i64 = 2;

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("random morph"))
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

#[tokio::test]
async fn cliffs_potion_morphs_at_random_and_the_party_potion_is_refused() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, 2, 1, CLIFFS, USES).await.expect("Cliff's");
    grant_stack(id, 2, 2, PARTY_POTION, 1)
        .await
        .expect("party potion");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // The weights are 10 or 15 each, so twelve uses all landing on the same
    // template happen less than once in 10^9.
    let mut seen = BTreeSet::new();
    for _ in 0..USES {
        send(c, use_item(1, CLIFFS)).await;
        let (morph, reason, time) = expect(c, |p| {
            let mut r = body(p, SendOpcode::GiveBuff)?;
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
        assert_eq!((reason, time), (-CLIFFS, 3_600_000));
        seen.insert(morph);
        send(c, cancel_item_effect(-CLIFFS)).await;
        expect(c, |p| {
            let mut r = body(p, SendOpcode::CancelBuff)?;
            (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then_some(())
        })
        .await;
    }
    assert!(seen.len() > 1, "always {seen:?}");

    // The party potion is only unlocked, and kept.
    send(c, use_item(2, PARTY_POTION)).await;
    expect(c, |p| {
        assert_ne!(p.opcode(), SendOpcode::GiveBuff as i16, "morphed");
        assert_ne!(
            p.opcode(),
            SendOpcode::InventoryOperation as i16,
            "the party potion was taken"
        );
        let r = body(p, SendOpcode::StatChange)?;
        (r.get_ref()[2..] == [1, 0, 0, 0, 0]).then_some(())
    })
    .await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&CLIFFS), None);
    assert_eq!(session.items.get(&PARTY_POTION), Some(&1));
}
