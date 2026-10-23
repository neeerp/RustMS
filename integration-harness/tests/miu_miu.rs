//! Miu Miu the Traveling Merchant (5450000): each use from the Cash tab
//! opens Mu Mu's shop where the player stands and costs one of the stack,
//! except in the maps that keep him away.

use integration_harness::database::{grant_stack, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const CASH: i16 = 5;
const MIU_MIU: i32 = 5_450_000;
const MU_MU: i32 = 9_090_000;
const APPLE: i32 = 2_010_000;
/// The first copy of the Pig Farm's mini dungeon: its fieldLimit has bit
/// 0x40000.
const PIG_FARM: i32 = 100_020_100;
const HENESYS: i32 = 100_000_000;

const BOUGHT: u8 = 0;

fn call() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(MIU_MIU).unwrap();
    p.write_int(0).unwrap();
    p
}

fn buy(row: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcShopAction as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(row).unwrap();
    p.write_int(item_id).unwrap();
    p.write_short(1).unwrap();
    p
}

#[tokio::test]
async fn miu_miu_opens_mu_mus_shop() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 100).await.expect("mesos");
    grant_stack(id, CASH, 1, MIU_MIU, 2).await.expect("Miu Miu");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, HENESYS, "", &mut seen).await, HENESYS);

    // He comes, for one of the two, and his shop sells an apple.
    send(c, call()).await;
    until(c, &mut seen, |t| !t.shops.is_empty()).await;
    assert_eq!(seen.quantities, [(CASH as u8, 1, 1)]);
    let (npc, rows) = seen.shops[0].clone();
    assert_eq!(npc, MU_MU);
    let row = rows.iter().position(|&(item, _)| item == APPLE).unwrap();
    assert_eq!(rows[row], (APPLE, 30));
    let t = step(c, "miu-miu-buy", buy(row as i16, APPLE)).await;
    assert_eq!(t.shop_results, [BOUGHT]);
    assert_eq!(t.stat(stat::MESO), Some(70));

    // Not while his shop (or any other window) is open.
    let t = step(c, "miu-miu-open", call()).await;
    assert_eq!(t.stat_updates, [Default::default()], "only the unlock");
    assert!(t.shops.is_empty() && t.quantities.is_empty());

    // Not in the Pig Farm: only the unlock, and the last one stays.
    let mut seen = Trace::default();
    assert_eq!(warp(c, PIG_FARM, "", &mut seen).await, PIG_FARM);
    barrier(c, "pig-farm", &mut seen).await;
    let t = step(c, &m, call()).await;
    assert_eq!(t.stat_updates, [Default::default()], "only the unlock");
    assert!(t.shops.is_empty() && t.quantities.is_empty() && t.removed.is_empty());
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.meso, 70);
    assert_eq!(session.items.get(&MIU_MIU), Some(&1));
    assert_eq!(session.items.get(&APPLE), Some(&1));
}
