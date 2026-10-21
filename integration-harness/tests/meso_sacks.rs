//! Meso sacks (520xxxx): opening one turns it into the mesos its data
//! holds; one that would pass the most mesos anyone holds stays shut.

use integration_harness::database::{grant_stack, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{stat, step};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const CASH: i16 = 5;
const SACK: i32 = 5_200_000;

fn open_sack() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(SACK).unwrap();
    p.write_int(0).unwrap();
    p
}

#[tokio::test]
async fn a_meso_sack_turns_into_its_mesos() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 100).await.expect("mesos");
    grant_stack(id, CASH, 1, SACK, 2).await.expect("sacks");

    let mut session = relogin(&config).await;
    let t = step(&mut session.connection, &m, open_sack()).await;
    assert_eq!(t.stat(stat::MESO), Some(1_000_100));
    assert_eq!(t.quantities, [(CASH as u8, 1, 1)]);
    assert_eq!(t.meso_gains, [1_000_000]);
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.meso, 1_000_100);
    assert_eq!(session.items.get(&SACK), Some(&1));
}

#[tokio::test]
async fn a_sack_past_the_meso_cap_stays_shut() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, i32::MAX - 10).await.expect("mesos");
    grant_stack(id, CASH, 1, SACK, 1).await.expect("a sack");

    let mut session = relogin(&config).await;
    let t = step(&mut session.connection, &m, open_sack()).await;
    assert_eq!(t.stat_updates, [Default::default()], "only the unlock");
    assert!(t.quantities.is_empty() && t.removed.is_empty());
    assert!(t.meso_gains.is_empty());
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.meso, i32::MAX - 10);
    assert_eq!(session.items.get(&SACK), Some(&1));
}
