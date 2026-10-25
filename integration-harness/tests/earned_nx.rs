//! NX earned by playing (`net::rates::nx`): the harness world gives 1 NX
//! Credit for each kill a player loots (`RUSTMS_NX_PER_KILL`), and the Cash
//! Shop shows it on entry.

use integration_harness::hunting::attack;
use integration_harness::login_to_world_session;
use integration_harness::packets::build_enter_cash_shop;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;

const FOREST: i32 = 40_000;
const SNAIL: i32 = 100100;

#[tokio::test]
async fn each_kill_earns_nx_credit() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, FOREST, "", &mut t).await, FOREST);
    // Earlier tests may have hunted here; wait for enough to respawn.
    until(c, &mut t, |t| t.field.alive(SNAIL).len() >= 3).await;
    let snails: Vec<i32> = t.field.alive(SNAIL)[..3].to_vec();
    for &snail in &snails {
        // A snail has 8 HP.
        send(c, attack(snail, 8)).await;
    }
    until(c, &mut t, |t| {
        snails.iter().all(|s| t.field.killed.contains(s))
    })
    .await;

    send(c, build_enter_cash_shop()).await;
    let credit = loop {
        let p = read(c, &mut t).await;
        if p.opcode() == SendOpcode::QueryCashResult as i16 {
            break Cursor::new(&p.bytes[2..]).read_int().unwrap();
        }
    };
    assert_eq!(credit, 3);
}
