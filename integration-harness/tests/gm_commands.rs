//! Chat commands (`net::gm_commands`): a GM's `!` line runs and nobody
//! else sees it; a player's is ordinary chat and gives nothing. `!givenx`
//! adds to an account's NX, which the Cash Shop shows, and `!coupon` makes
//! codes it redeems.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::packets::{build_all_chat, build_coupon_code, build_enter_cash_shop};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::Duration;

const FIELD: i32 = 101_020_001;
const HENESYS: i32 = 100_000_000;
const RED_POTION: i32 = 2_000_000;

/// The chat lines `c` receives up to and including `marker`.
async fn chat_until(c: &mut MapleTestConnection, marker: &str) -> Vec<String> {
    let mut trace = Trace::default();
    let mut lines = Vec::new();
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() != SendOpcode::ChatText as i16 {
            continue;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        r.read_int().unwrap();
        r.read_byte().unwrap();
        let line = r.read_str_with_length().unwrap();
        let done = line == marker;
        lines.push(line);
        if done {
            return lines;
        }
    }
}

#[tokio::test]
async fn a_gms_bang_lines_run_and_a_players_are_chat() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config)
        .await
        .expect("first login failed");
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    login_to_world_session(&other_config)
        .await
        .expect("second login failed");
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 2).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let mut player = relogin(&other_config).await;
    let (g, p) = (&mut gm.connection, &mut player.connection);
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    assert_eq!(warp(g, FIELD, "", &mut gt).await, FIELD);
    assert_eq!(warp(p, FIELD, "", &mut pt).await, FIELD);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The player's line is chat that both see, and gives nothing.
    let mut pt = Trace::default();
    send(p, build_all_chat("!item 2000000 5", 0).unwrap()).await;
    barrier(p, "player done", &mut pt).await;
    assert!(pt.added.is_empty(), "{:?}", pt.added);
    let seen = chat_until(g, "player done").await;
    assert!(seen.contains(&"!item 2000000 5".to_string()), "{seen:?}");

    // The GM's lines run, and the player sees none of them.
    let mut gt = Trace::default();
    send(g, build_all_chat("!item 2000000 5", 0).unwrap()).await;
    send(g, build_all_chat("!meso 1000", 0).unwrap()).await;
    send(g, build_all_chat("!nothing", 0).unwrap()).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(gt.items, [(RED_POTION, 5)]);
    assert_eq!(gt.meso_gains, [1000]);
    assert_eq!(
        gt.broadcasts,
        [(5, "Unknown command. Type !help for a list.".to_string())]
    );
    assert_eq!(chat_until(p, "gm done").await, ["gm done"]);

    // And a warp.
    send(g, build_all_chat(&format!("!warp {HENESYS}"), 0).unwrap()).await;
    let before = gt.warps.len();
    while gt.warps.len() == before {
        read(g, &mut gt).await;
    }
    assert_eq!(gt.warps.last(), Some(&HENESYS));
}

/// The balances (NX Credit, Maple Points, NX Prepaid) the Cash Shop opens
/// with.
async fn cash_shop_balances(c: &mut MapleTestConnection) -> (i32, i32, i32) {
    send(c, build_enter_cash_shop()).await;
    let mut trace = Trace::default();
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() == SendOpcode::QueryCashResult as i16 {
            let mut r = Cursor::new(&p.bytes[2..]);
            return (
                r.read_int().unwrap(),
                r.read_int().unwrap(),
                r.read_int().unwrap(),
            );
        }
    }
}

#[tokio::test]
async fn givenx_adds_to_an_accounts_nx() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config)
        .await
        .expect("first login failed");
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    login_to_world_session(&other_config)
        .await
        .expect("second login failed");
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 2).await.expect("make a GM");

    // The other character's account gets Maple Points while they are away.
    let mut gm = relogin(&config).await;
    let g = &mut gm.connection;
    let other = other_config.character_name.to_lowercase();
    let mut gt = Trace::default();
    send(g, build_all_chat("!givenx 5000", 0).unwrap()).await;
    send(g, build_all_chat("!givenx nx -1000", 0).unwrap()).await;
    send(
        g,
        build_all_chat(&format!("!givenx mp {other} 20"), 0).unwrap(),
    )
    .await;
    send(g, build_all_chat("!givenx nobody_here 1", 0).unwrap()).await;
    barrier(g, "gm done", &mut gt).await;
    let (me, them) = (&config.character_name, &other_config.character_name);
    assert_eq!(
        gt.broadcasts,
        [
            (5, format!("{me}'s account has 5000 NX Credit (+5000).")),
            (5, format!("{me}'s account has 4000 NX Credit (-1000).")),
            (5, format!("{them}'s account has 20 Maple Points (+20).")),
            (5, "There is no character named nobody_here.".to_string()),
        ]
    );
    assert_eq!(cash_shop_balances(g).await, (4000, 0, 0));

    let mut player = relogin(&other_config).await;
    assert_eq!(cash_shop_balances(&mut player.connection).await, (0, 20, 0));
}

/// Redeem `code` in the Cash Shop: the CASHSHOP_OPERATION mode it gets,
/// and the balances after.
async fn redeem(c: &mut MapleTestConnection, code: &str) -> (u8, (i32, i32, i32)) {
    send(c, build_coupon_code(code).unwrap()).await;
    let mut trace = Trace::default();
    let mut mode = None;
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() == SendOpcode::CashShopOperation as i16 {
            mode = Some(p.bytes[2]);
        } else if p.opcode() == SendOpcode::QueryCashResult as i16 {
            let mut r = Cursor::new(&p.bytes[2..]);
            let balances = (
                r.read_int().unwrap(),
                r.read_int().unwrap(),
                r.read_int().unwrap(),
            );
            return (mode.expect("no reply before the balances"), balances);
        }
    }
}

#[tokio::test]
async fn coupons_a_gm_makes_are_redeemed() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("login failed");
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 2).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let g = &mut gm.connection;
    let code = format!("GM{}", config.character_name.to_uppercase());
    let mut gt = Trace::default();
    send(
        g,
        build_all_chat(&format!("!coupon nx=700 code={code}"), 0).unwrap(),
    )
    .await;
    send(
        g,
        build_all_chat(&format!("!coupon nx=1 code={code}"), 0).unwrap(),
    )
    .await;
    send(g, build_all_chat("!coupon mp=5 days=3", 0).unwrap()).await;
    barrier(g, "gm done", &mut gt).await;
    let lines: Vec<&str> = gt
        .broadcasts
        .iter()
        .map(|(_, line)| line.as_str())
        .collect();
    let [made, taken, random] = lines.as_slice() else {
        panic!("{lines:?}");
    };
    assert_eq!(*made, format!("Made coupon {code}."));
    assert_eq!(*taken, "That code is taken.");
    let random = random
        .strip_prefix("Made coupon ")
        .and_then(|rest| rest.strip_suffix(", good for 3 days."))
        .unwrap_or_else(|| panic!("{random}"));
    assert_eq!(random.len(), 16);

    assert_eq!(cash_shop_balances(g).await, (0, 0, 0));
    // 0x59: what the coupon gave.
    assert_eq!(redeem(g, &code.to_lowercase()).await, (0x59, (700, 0, 0)));
    assert_eq!(redeem(g, random).await, (0x59, (700, 5, 0)));
}
