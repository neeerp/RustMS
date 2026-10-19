//! AP Reset: one AP moves from one stat to another and the item is used up;
//! a move the stats don't allow keeps it.

use integration_harness::database::{grant_stack, set_dex, set_hp, set_job, set_str};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const CASH: i16 = 5;
const AP_RESET: i32 = 5_050_000;
const STR: i32 = 0x40;
const DEX: i32 = 0x80;
const INT: i32 = 0x100;

fn reset(to: i32, from: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(AP_RESET).unwrap();
    p.write_int(to).unwrap();
    p.write_int(from).unwrap();
    p.write_int(0).unwrap();
    p
}

#[tokio::test]
async fn an_ap_reset_moves_an_ap() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 30, 200).await.expect("a fighter");
    set_hp(id, 1500, 1500).await.expect("hp");
    set_str(id, 120).await.expect("str");
    set_dex(id, 30).await.expect("dex");
    grant_stack(id, CASH, 1, AP_RESET, 2)
        .await
        .expect("grant AP Resets");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // INT is at its least: refused, and only the client is unlocked.
    let t = step(c, &m, reset(STR, INT)).await;
    assert_eq!(t.stat_updates, [Default::default()]);
    assert!(t.quantities.is_empty() && t.removed.is_empty());

    let t = step(c, &m, reset(STR, DEX)).await;
    assert_eq!((t.stat(STR), t.stat(DEX)), (Some(121), Some(29)));
    assert_eq!(t.quantities, [(CASH as u8, 1, 1)]);

    let t = step(c, &m, reset(DEX, STR)).await;
    assert_eq!((t.stat(STR), t.stat(DEX)), (Some(120), Some(30)));
    assert_eq!(t.removed, [(CASH as u8, 1)]);

    // None left.
    let t = step(c, &m, reset(STR, DEX)).await;
    assert_eq!(t.stat_updates, [Default::default()]);
}
