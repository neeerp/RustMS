//! Mini-dungeons: the Pig Farm's (100020000) `MD00` takes a player, or a
//! party's leader with the members beside them, into a free copy of the
//! dungeon, and the copy's `out00` leads back. A party member who doesn't
//! lead it is turned back, and the leader's leaving sends the rest out.

use integration_harness::events::party;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const PIG_FARM: i32 = 100020000;
const COPIES: std::ops::Range<i32> = 100020100..100020130;
/// The copies' `out00` and the Pig Farm's `MD00`.
const OUT00: u8 = 4;
const MD00: u8 = 8;
const PINK: u8 = 5;

fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

async fn at_the_farm() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut player.connection, PIG_FARM, "", &mut trace).await,
        PIG_FARM
    );
    player
}

/// Take `portal` and say where it led.
async fn take(player: &mut WorldSession, portal: &str) -> (i32, u8) {
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, enter(portal)).await;
    assert!(trace.broadcasts.is_empty(), "{:?}", trace.broadcasts);
    let [(map, portal, _)] = trace.arrivals[..] else {
        panic!("{trace:?}");
    };
    (map, portal)
}

/// Wait to be sent somewhere by someone else.
async fn sent(player: &mut WorldSession) -> (i32, u8) {
    let mut trace = Trace::default();
    until(&mut player.connection, &mut trace, |t| {
        !t.arrivals.is_empty()
    })
    .await;
    let (map, portal, _) = trace.arrivals[0];
    (map, portal)
}

#[tokio::test]
async fn each_comer_gets_a_copy_and_a_partys_leader_brings_the_party() {
    // Alone, each to a copy of their own, and back.
    let mut first = at_the_farm().await;
    let mut second = at_the_farm().await;
    let (copy, portal) = take(&mut first, "MD00").await;
    assert!(COPIES.contains(&copy), "{copy}");
    assert_eq!(portal, OUT00);
    let (other, _) = take(&mut second, "MD00").await;
    assert!(COPIES.contains(&other), "{other}");
    assert_ne!(other, copy);
    assert_eq!(take(&mut first, "out00").await, (PIG_FARM, MD00));
    assert_eq!(take(&mut second, "out00").await, (PIG_FARM, MD00));

    // In a party, only its leader opens the way.
    let mut team = [at_the_farm().await, at_the_farm().await];
    party(&mut team).await;
    let [leader, member] = &mut team;
    let name = member.character_name.clone();
    let trace = step(&mut member.connection, &name, enter("MD00")).await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(
        matches!(trace.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty()),
        "{:?}",
        trace.broadcasts
    );
    let (copy, _) = take(leader, "MD00").await;
    assert!(COPIES.contains(&copy), "{copy}");
    assert_eq!(sent(member).await, (copy, OUT00));
    // The leader's leaving sends the member back to the farm.
    assert_eq!(take(leader, "out00").await, (PIG_FARM, MD00));
    assert_eq!(sent(member).await.0, PIG_FARM);
}
