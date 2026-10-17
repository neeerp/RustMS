//! Raising and dropping fame (GIVE_FAME, FAME_RESPONSE): from level 15, on
//! the same map, once a day, and to the same character once a month.
//! Fame that reaches nobody doesn't count.

use integration_harness::database::{backdate_fame, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MAP: i32 = 40_000;
const FAME: i32 = 0x20000;

fn give_fame(target: i32, up: bool) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::GiveFame as i16).unwrap();
    p.write_int(target).unwrap();
    p.write_byte(u8::from(up)).unwrap();
    p
}

fn refusal(mode: u8) -> Vec<(u8, String, bool, i32)> {
    vec![(mode, String::new(), false, 0)]
}

#[tokio::test]
async fn fame_goes_once_a_day_and_to_each_character_once_a_month() {
    let giver_config = load_harness_config_or_fail().await;
    let target_config = load_harness_config_or_fail().await;
    let novice_config = load_harness_config_or_fail().await;

    let giver_id = login_to_world_session(&giver_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&giver_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(giver_id, 100, 20, 50)
        .await
        .expect("make a warrior");

    let mut giver = relogin(&giver_config).await;
    let mut target = login_to_world_session(&target_config).await.unwrap();
    let mut novice = login_to_world_session(&novice_config).await.unwrap();
    assert_ne!(novice.map_id, MAP, "the novice starts off the map");
    let (mut g, mut t) = (Trace::default(), Trace::default());
    assert_eq!(warp(&mut giver.connection, MAP, "", &mut g).await, MAP);
    assert_eq!(warp(&mut target.connection, MAP, "", &mut t).await, MAP);
    let target_id = target.character_id;

    // Fame for someone not on the map reaches nobody and doesn't count.
    for (marker, missing) in [("nobody", i32::MAX), ("elsewhere", novice.character_id)] {
        let mut g = step(&mut giver.connection, marker, give_fame(missing, true)).await;
        until(&mut giver.connection, &mut g, |g| !g.fame.is_empty()).await;
        assert_eq!(g.fame, refusal(1), "{marker}");
    }
    assert_eq!(
        warp(&mut novice.connection, MAP, "", &mut Trace::default()).await,
        MAP
    );

    let n = step(&mut novice.connection, "novice", give_fame(target_id, true)).await;
    assert_eq!(n.fame, refusal(2));

    let s = step(&mut giver.connection, "self", give_fame(giver_id, true)).await;
    assert!(s.fame.is_empty());

    let mut t = Trace::default();
    let mut g = step(&mut giver.connection, "up", give_fame(target_id, true)).await;
    until(&mut target.connection, &mut t, |t| !t.fame.is_empty()).await;
    assert_eq!(t.fame, [(5, giver.character_name.clone(), true, 0)]);
    assert_eq!(t.stat(FAME), Some(1));
    until(&mut giver.connection, &mut g, |g| !g.fame.is_empty()).await;
    assert_eq!(g.fame, [(0, target.character_name.clone(), true, 1)]);

    let g = step(&mut giver.connection, "today", give_fame(target_id, false)).await;
    assert_eq!(g.fame, refusal(3));

    // Two days on, the same target is still refused for the month.
    backdate_fame(giver_id, 2).await.expect("backdate the fame");
    let g = step(&mut giver.connection, "month", give_fame(target_id, false)).await;
    assert_eq!(g.fame, refusal(4));

    // ...but anyone else may be.
    let novice_id = novice.character_id;
    let mut g = step(&mut giver.connection, "down", give_fame(novice_id, false)).await;
    until(&mut giver.connection, &mut g, |g| !g.fame.is_empty()).await;
    assert_eq!(g.fame, [(0, novice.character_name.clone(), false, -1)]);
}
