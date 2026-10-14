//! NPC shops (OPEN_NPC_SHOP, NPC_SHOP_ACTION): the Henesys grocer's list
//! comes from the content pack, and buying and selling move mesos and items
//! that stay moved.

use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const GROCERY: i32 = 100_000_102;
const GROCER: i32 = 1011100;
const ARROW: i32 = 2060000;
const RED_POTION: i32 = 2000000;
const SUBI: i32 = 2070000;
const WEAPON: i16 = -11;
/// HeavenClient sends the EQUIPPED type for worn items.
const EQUIPPED: i8 = -1;

const BOUGHT: u8 = 0;
const NOT_ENOUGH_MESOS: u8 = 2;
const SOLD: u8 = 8;

fn talk(object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(object_id).unwrap();
    p
}

fn item_move(kind: i8, from: i16, to: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(kind as u8).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(1).unwrap();
    p
}

fn shop_action(mode: u8, slot: i16, item_id: i32, quantity: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcShopAction as i16).unwrap();
    p.write_byte(mode).unwrap();
    if mode < 3 {
        p.write_short(slot).unwrap();
    }
    if mode < 2 {
        p.write_int(item_id).unwrap();
        p.write_short(quantity).unwrap();
    }
    p
}

#[tokio::test]
async fn selling_a_sword_buys_an_arrow_at_the_henesys_grocer() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let weapon = session.equipped[&WEAPON.abs()];
    assert_eq!(session.meso, 0);
    let mut seen = Trace::default();
    assert_eq!(warp(c, GROCERY, "", &mut seen).await, GROCERY);
    until(c, &mut seen, |t| t.field.npc(GROCER).is_some()).await;

    // The list is the pack's, less what is only recharged there.
    send(c, talk(seen.field.npc(GROCER).unwrap())).await;
    until(c, &mut seen, |t| !t.shops.is_empty()).await;
    let (npc, rows) = seen.shops[0].clone();
    assert_eq!(npc, GROCER);
    assert!(rows.contains(&(RED_POTION, 50)), "{rows:?}");
    assert!(rows.contains(&(SUBI, 500)), "{rows:?}");
    assert!(rows
        .iter()
        .all(|&(item, price)| price > 0 || matches!(item / 10_000, 207 | 233)));
    let row = |item_id| rows.iter().position(|&(item, _)| item == item_id).unwrap() as i16;

    // Nothing is free.
    let t = step(c, "shop-broke", shop_action(0, row(ARROW), ARROW, 1)).await;
    assert_eq!(t.shop_results, [NOT_ENOUGH_MESOS]);
    assert!(t.added.is_empty());

    // The sword sells for its price of 1 meso, which buys one arrow.
    step(c, "shop-unequip", item_move(EQUIPPED, WEAPON, 1)).await;
    let t = step(c, "shop-sell", shop_action(1, 1, weapon, 1)).await;
    assert_eq!(t.shop_results, [SOLD]);
    assert_eq!(t.stat(stat::MESO), Some(1));
    let t = step(c, "shop-buy", shop_action(0, row(ARROW), ARROW, 1)).await;
    assert_eq!(t.shop_results, [BOUGHT]);
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.added.len(), 1);
    assert_eq!(t.added[0].2, ARROW);

    // A row the item isn't in, or one more sword, is refused outright.
    let t = step(c, "shop-wrong-row", shop_action(0, row(SUBI), ARROW, 1)).await;
    assert!(t.shop_results.is_empty());
    let t = step(c, "shop-sell-again", shop_action(1, 1, weapon, 1)).await;
    assert!(t.shop_results.is_empty());

    // Once the shop is closed nothing more is sold.
    send(c, shop_action(3, 0, 0, 0)).await;
    let t = step(c, "shop-closed", shop_action(0, row(ARROW), ARROW, 1)).await;
    assert!(t.shop_results.is_empty());
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.meso, 0);
    assert!(!session.equipped.contains_key(&WEAPON.abs()));
    assert!(!session.items.contains_key(&weapon));
    assert_eq!(session.items.get(&ARROW), Some(&1));
}
