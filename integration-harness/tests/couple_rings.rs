//! Crush and friendship rings (report b061): CASHSHOP_OPERATION 0x1D buys a
//! crush ring pair, one for the buyer's Cash Inventory and one sent to the
//! partner as a gift (0x23 is the same for friendship rings), and the
//! server answers with mode 0x87. The pair is recorded by the rings' cash
//! serials: each character's data lists the rings it carries, and a worn
//! ring shows its crush block, (serial, partner's serial, item), to the
//! map.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_nx_credit;
use integration_harness::packets::{
    build_cash_shop_buy, build_cash_shop_buy_ring, build_cash_shop_gift, build_cash_shop_take_out,
    build_enter_cash_shop, CoupleRingRecord,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;

const HENESYS: i32 = 100_000_000;
/// Commodities from Etc.nx: a Crush Ring (1112001, 6,000 NX) and a
/// Friendship Ring (1112800, 4,500 NX), both on sale.
const CRUSH_RING_SN: i32 = 20900028;
const CRUSH_RING: i32 = 1112001;
const FRIENDSHIP_RING_SN: i32 = 20900056;
const EQUIP: u8 = 1;
/// The first ring slot, less 100 for a cash item.
const CASH_RING_SLOT: i16 = -112;

async fn next(c: &mut MapleTestConnection, t: &mut Trace, op: SendOpcode) -> Vec<u8> {
    let op = op as i16;
    loop {
        let p = read(c, t).await;
        if p.opcode() == op {
            return p.bytes[2..].to_vec();
        }
    }
}

/// Enter the Cash Shop and return its Cash Inventory's cash serials.
async fn enter_cash_shop(c: &mut MapleTestConnection, t: &mut Trace) -> Vec<i64> {
    send(c, build_enter_cash_shop()).await;
    let serials = loop {
        let list = next(c, t, SendOpcode::CashShopOperation).await;
        if list[0] == 0x4B {
            let mut r = Cursor::new(&list[1..]);
            let count = r.read_short().unwrap();
            break (0..count)
                .map(|_| {
                    let serial = r.read_long().unwrap();
                    r.read_bytes(55 - 8).unwrap();
                    serial
                })
                .collect();
        }
    };
    next(c, t, SendOpcode::QueryCashResult).await;
    serials
}

/// Take the Cash Inventory item `cash_id` out into the Equip bag and
/// return the slot it went to.
async fn take_out(c: &mut MapleTestConnection, t: &mut Trace, cash_id: i64) -> i16 {
    send(c, build_cash_shop_take_out(cash_id, EQUIP, 1).unwrap()).await;
    let taken = next(c, t, SendOpcode::CashShopOperation).await;
    assert_eq!(taken[0], 0x68, "taken out");
    Cursor::new(&taken[1..]).read_short().unwrap()
}

fn equip(from: i16, to: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(EQUIP).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(1).unwrap();
    p
}

#[tokio::test]
async fn a_crush_ring_pair_is_bought_for_two_and_shows_when_worn() {
    let a_config = load_harness_config_or_fail().await;
    let b_config = load_harness_config_or_fail().await;
    let mut a = login_to_world_session(&a_config)
        .await
        .expect("buyer login");
    let b = login_to_world_session(&b_config)
        .await
        .expect("partner login");
    let (a_id, b_id) = (a.character_id, b.character_id);
    let b_name = b.character_name.clone();
    drop(b);
    set_nx_credit(&a_config.username, 20_000)
        .await
        .expect("grant NX");
    let (c, mut t) = (&mut a.connection, Trace::default());
    enter_cash_shop(c, &mut t).await;

    let buy = |friendship, birthday, sn, to: &str| {
        build_cash_shop_buy_ring(friendship, birthday, 1, sn, to, "be mine").unwrap()
    };
    let cases = [
        (
            build_cash_shop_buy(1, CRUSH_RING_SN).unwrap(),
            [0x58, 0x00],
            "a crush ring bought alone",
        ),
        (
            build_cash_shop_gift(19830113, CRUSH_RING_SN, &b_name, "be mine").unwrap(),
            [0x5C, 0x00],
            "a crush ring as a gift",
        ),
        (
            build_cash_shop_buy_ring(false, 19830113, 2, CRUSH_RING_SN, &b_name, "be mine")
                .unwrap(),
            [0x5C, 0x00],
            "Maple Points",
        ),
        (
            buy(true, 19830113, CRUSH_RING_SN, &b_name),
            [0x5C, 0x00],
            "a crush ring as a friendship ring",
        ),
        (
            buy(false, 19830113, FRIENDSHIP_RING_SN, &b_name),
            [0x5C, 0x00],
            "a friendship ring as a crush ring",
        ),
        (
            buy(false, 0, CRUSH_RING_SN, &b_name),
            [0x5C, 0xC4],
            "no birthday code",
        ),
        (
            buy(false, 19830113, CRUSH_RING_SN, "NoSuchName"),
            [0x5C, 0xA9],
            "an unknown partner",
        ),
        (
            buy(false, 19830113, CRUSH_RING_SN, &a_config.character_name),
            [0x5C, 0xA8],
            "the buyer's account",
        ),
    ];
    for (packet, reply, phase) in cases {
        send(c, packet).await;
        assert_eq!(
            next(c, &mut t, SendOpcode::CashShopOperation).await,
            reply,
            "{phase}"
        );
    }

    send(c, buy(false, 19830113, CRUSH_RING_SN, &b_name)).await;
    let bought = next(c, &mut t, SendOpcode::CashShopOperation).await;
    let mut r = Cursor::new(&bought[..]);
    assert_eq!(r.read_byte().unwrap(), 0x87, "a ring pair bought");
    let a_ring = r.read_long().unwrap();
    r.read_bytes(55 - 8).unwrap();
    assert_eq!(r.read_str_with_length().unwrap(), b_name);
    assert_eq!(r.read_int().unwrap(), CRUSH_RING);
    assert_eq!(r.read_short().unwrap(), 1);
    let balance = next(c, &mut t, SendOpcode::QueryCashResult).await;
    assert_eq!(Cursor::new(&balance[..]).read_int().unwrap(), 14_000);
    let a_slot = take_out(c, &mut t, a_ring).await;
    drop(a);

    // The partner's ring waits in its Cash Inventory.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut b = relogin(&b_config).await;
    let (c, mut t) = (&mut b.connection, Trace::default());
    let gifts = enter_cash_shop(c, &mut t).await;
    let [b_ring] = gifts[..] else {
        panic!("expected the ring alone: {gifts:?}");
    };
    let b_slot = take_out(c, &mut t, b_ring).await;
    drop(b);

    // Each one's character data lists its ring of the pair.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;
    let record = |partner_id, partner_name: &str, ring_id, partner_ring_id| CoupleRingRecord {
        partner_id,
        partner_name: partner_name.into(),
        ring_id,
        partner_ring_id,
        item_id: None,
    };
    assert_eq!(a.couple_rings, [record(b_id, &b_name, a_ring, b_ring)]);
    assert_eq!(
        b.couple_rings,
        [record(a_id, &a_config.character_name, b_ring, a_ring)]
    );

    // Worn, each one's ring shows to the other with both serials.
    let (mut at, mut bt) = (Trace::default(), Trace::default());
    assert_eq!(warp(&mut a.connection, HENESYS, "", &mut at).await, HENESYS);
    assert_eq!(warp(&mut b.connection, HENESYS, "", &mut bt).await, HENESYS);
    until(&mut a.connection, &mut at, |t| {
        t.players.iter().any(|p| p.character_id == b_id)
    })
    .await;
    send(&mut a.connection, equip(a_slot, CASH_RING_SLOT)).await;
    until(&mut b.connection, &mut bt, |t| !t.looks.is_empty()).await;
    assert_eq!(bt.looks[0].0, a_id);
    assert_eq!(bt.looks[0].1.crush, Some((a_ring, b_ring, CRUSH_RING)));
    send(&mut b.connection, equip(b_slot, CASH_RING_SLOT)).await;
    until(&mut a.connection, &mut at, |t| !t.looks.is_empty()).await;
    assert_eq!(at.looks[0].0, b_id);
    assert_eq!(at.looks[0].1.crush, Some((b_ring, a_ring, CRUSH_RING)));
    assert_eq!(at.looks[0].1.friendship, None);
}
