//! Writs of Solomon: one becomes gacha EXP, and no other may be used while
//! some is left. Taking it in finishes the level, and the next if it covers
//! it; what is left is kept across logins.

use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const USE: i16 = 2;
/// A Writ of Solomon worth 100 EXP, for level 50 and under.
const WRIT: i32 = 2_370_012;

/// USE_SOLOMON_ITEM of the writ in USE slot `slot`.
fn use_writ(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSolomonItem as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(WRIT).unwrap();
    p
}

/// USE_GACHA_EXP, sent on double-clicking the EXP gauge.
fn take() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseGachaExp as i16).unwrap();
    p.write_int(0).unwrap();
    p
}

#[tokio::test]
async fn a_writ_of_solomon_becomes_exp_taken_a_level_or_two_at_a_time() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, WRIT, 2).await.expect("grant writs");

    let mut player = relogin(&config).await;
    let (c, mut t) = (&mut player.connection, Trace::default());
    send(c, use_writ(1)).await;
    until(c, &mut t, |t| t.stat(stat::GACHA_EXP).is_some()).await;
    assert_eq!(t.stat(stat::GACHA_EXP), Some(100));
    assert_eq!(t.quantities.last(), Some(&(USE as u8, 1, 1)));

    // Not another while some is left: the client is only unlocked.
    let updates = t.stat_updates.len();
    send(c, use_writ(1)).await;
    until(c, &mut t, |t| t.stat_updates.len() > updates).await;
    assert!(t.stat_updates[updates].is_empty());
    assert_eq!(t.quantities.len(), 1, "the second writ is kept");

    // At level 1 it finishes level 1 (15) and level 2 (34), keeping 51.
    send(c, take()).await;
    until(c, &mut t, |t| t.stat(stat::GACHA_EXP) == Some(51)).await;
    assert_eq!(t.stat(stat::LEVEL), Some(3));
    assert_eq!(t.stat(stat::EXP), Some(0));
    drop(player);

    // The 51 left is kept, and short of level 3's 57, taken whole.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut player = relogin(&config).await;
    let (c, mut t) = (&mut player.connection, Trace::default());
    send(c, take()).await;
    until(c, &mut t, |t| t.stat(stat::GACHA_EXP) == Some(0)).await;
    assert_eq!(t.stat(stat::EXP), Some(51));
    assert_eq!(t.stat(stat::LEVEL), None);
}
