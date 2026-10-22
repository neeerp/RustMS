//! A member in the Cash Shop is on no map, so their party can't take them
//! into a party quest: with them gone, Lakelis finds too few in Kerning
//! City and doesn't ask the leader.

use integration_harness::events::{in_lobby, party, talk, LAKELIS};
use integration_harness::packets::build_enter_cash_shop;
use integration_harness::quests::answer;
use integration_harness::trace::{read, send, step, Trace};
use integration_harness::WorldSession;
use net::events::KPQ;
use net::packet::op::SendOpcode;
use tokio::time::Duration;

/// What the leader is shown on talking to Lakelis.
async fn ask_lakelis(leader: &mut WorldSession) -> Vec<(i32, u8)> {
    let name = leader.character_name.clone();
    step(&mut leader.connection, &name, talk(KPQ.lobby, LAKELIS))
        .await
        .prompts
}

#[tokio::test]
async fn a_member_in_the_cash_shop_does_not_count_for_entry() {
    let mut team = vec![
        in_lobby(25, &[]).await,
        in_lobby(25, &[]).await,
        in_lobby(25, &[]).await,
    ];
    party(&mut team).await;

    // All three in Kerning City: Lakelis asks the leader, once the party
    // before them is out; the leader says no.
    let mut asked = false;
    for _ in 0..30 {
        if ask_lakelis(&mut team[0]).await == [(LAKELIS, 1)] {
            asked = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(asked, "Lakelis never asked the leader");
    let name = team[0].character_name.clone();
    step(&mut team[0].connection, &name, answer(1, 0, None)).await;

    // One goes into the Cash Shop, and two are too few.
    let shopper = &mut team[2].connection;
    send(shopper, build_enter_cash_shop()).await;
    let mut trace = Trace::default();
    while read(shopper, &mut trace).await.opcode() != SendOpcode::QueryCashResult as i16 {}
    assert_eq!(ask_lakelis(&mut team[0]).await, [(LAKELIS, 0)]);
}
