//! Cash mists (528xxxx): Passed Gas leaves its cloud around the player for
//! everyone on the map; a second from the same item, or any while standing
//! in one, leaves none, though the item is spent.

use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// A map no other test leaves a cloud on or looks for one.
const LUDIBRIUM: i32 = 220_000_000;
const CASH: i16 = 5;
const PASSED_GAS: i32 = 5_281_000;
const FLORAL_SCENT: i32 = 5_281_001;
/// A cash item's mist kind.
const ITEM: i32 = 3;

fn use_mist(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_int(0).unwrap();
    p
}

#[tokio::test]
async fn passed_gas_leaves_one_cloud() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, PASSED_GAS, 2).await.expect("gas");
    grant_stack(id, CASH, 2, FLORAL_SCENT, 1)
        .await
        .expect("scent");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, LUDIBRIUM, "", &mut t).await, LUDIBRIUM);

    send(c, use_mist(1, PASSED_GAS)).await;
    // The face comes last.
    until(c, &mut t, |t| !t.item_emotions.is_empty()).await;
    let mist = t.field.mists[0];
    assert_eq!(
        (
            mist.kind,
            mist.owner_id,
            mist.skill_id,
            mist.level,
            mist.delay
        ),
        (ITEM, id, PASSED_GAS, 1, 0)
    );
    // The item's box, (-110, -82) to (110, 83) around the player.
    let (left, top, right, bottom) = mist.area;
    assert_eq!((right - left, bottom - top), (220, 165));
    assert_eq!(t.quantities, [(CASH as u8, 1, 1)]);
    assert_eq!(t.mist_puffs, [PASSED_GAS]);
    assert_eq!(t.item_emotions, [PASSED_GAS]);

    // Another from the same item, or any while standing in the cloud: the
    // item goes, but no second cloud.
    send(c, use_mist(1, PASSED_GAS)).await;
    send(c, use_mist(2, FLORAL_SCENT)).await;
    read_for(c, &mut t, 1000).await;
    assert_eq!(t.field.mists.len(), 1, "one cloud");
    assert_eq!(t.mist_puffs, [PASSED_GAS], "and one puff");
    assert_eq!(t.removed.len(), 2, "both spent");
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&PASSED_GAS), None);
    assert_eq!(session.items.get(&FLORAL_SCENT), None);
}
