//! Summoning sacks: one brings out its mobs on the ground below the player,
//! under the player's control, and is used up. Where the map forbids them
//! (fieldLimit 0x4) it stays shut and is kept.

use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const USE: i16 = 2;
/// Brings out a Jr. Balrog every time.
const SACK: i32 = 2_100_000;
const JR_BALROG: i32 = 8_130_100;
/// Maple Island's Mushroom Town, which forbids summoning sacks.
const MUSHROOM_TOWN: i32 = 10_000;
/// Sleepywood, where no other test goes, so the Jr. Balrog troubles no one.
const FIELD: i32 = 105_040_300;

/// USE_SUMMON_BAG of the sack in USE slot `slot`.
fn open_sack(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSummonBag as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

#[tokio::test]
async fn a_summoning_sack_brings_out_its_mobs_where_the_map_allows() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, SACK, 2)
        .await
        .expect("grant summoning sacks");
    let mut player = relogin(&config).await;
    let (c, mut t) = (&mut player.connection, Trace::default());

    // Maple Island forbids them: the client is unlocked and keeps its sack.
    assert_eq!(warp(c, MUSHROOM_TOWN, "", &mut t).await, MUSHROOM_TOWN);
    send(c, open_sack(1, SACK)).await;
    let op = SendOpcode::StatChange as i16;
    let unlock = loop {
        let p = read(c, &mut t).await;
        if p.opcode() == op {
            break p;
        }
    };
    assert_eq!(
        unlock.bytes[2], 1,
        "an empty stat change unlocks the client"
    );
    assert!(t.quantities.is_empty(), "the sack is kept");
    assert!(t.field.alive(JR_BALROG).is_empty());

    // Elsewhere a Jr. Balrog fades in under the player's control.
    assert_eq!(warp(c, FIELD, "", &mut t).await, FIELD);
    send(c, open_sack(1, SACK)).await;
    until(c, &mut t, |t| {
        t.field
            .alive(JR_BALROG)
            .iter()
            .any(|oid| t.field.controlled.contains(oid))
    })
    .await;
    let balrog = t.field.alive(JR_BALROG)[0];
    assert_eq!(t.field.appearances.get(&balrog), Some(&-2));
    assert_eq!(t.quantities.last(), Some(&(USE as u8, 1, 1)));
}
