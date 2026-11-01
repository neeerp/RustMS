//! The chandelier in Ergoth's room (990000900): a hit on the pulley locks
//! it for everyone and drops the chandelier, step by step, to the bottom;
//! one coming in while it is down sees it there, the pulley locked.

use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ROOM: i32 = 990_000_900;
const CORRIDOR: i32 = 990_000_800;
const BOTTOM: i16 = -350;

fn pull() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::GuildBossPulley as i16).unwrap();
    p
}

fn heights(trace: &Trace) -> Vec<i16> {
    let op = SendOpcode::GuildBossHealerMove as i16;
    (trace.guild_boss.iter())
        .filter(|(o, _)| *o == op)
        .map(|(_, b)| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}

fn pulley(trace: &Trace) -> Vec<u8> {
    let op = SendOpcode::GuildBossPulleyState as i16;
    (trace.guild_boss.iter())
        .filter(|(o, _)| *o == op)
        .map(|(_, b)| b[0])
        .collect()
}

#[tokio::test]
async fn the_pulley_drops_the_chandelier_in_ergoths_room() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config)
        .await
        .expect("first login failed");
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, ROOM, "", &mut t).await, ROOM);
    assert!(t.guild_boss.is_empty(), "it hangs at the top: {t:?}");

    send(c, pull()).await;
    until(c, &mut t, |t| heights(t).contains(&BOTTOM)).await;
    assert_eq!(pulley(&t), [1]);
    let fell = heights(&t);
    assert!(fell.windows(2).all(|w| w[0] < w[1]), "{fell:?}");
    assert_eq!(fell[0], -554);

    // Back in while it is down.
    let mut t = Trace::default();
    assert_eq!(warp(c, CORRIDOR, "", &mut t).await, CORRIDOR);
    let mut t = Trace::default();
    assert_eq!(warp(c, ROOM, "", &mut t).await, ROOM);
    until(c, &mut t, |t| !pulley(t).is_empty()).await;
    assert_eq!(pulley(&t), [1]);
    let seen = heights(&t);
    assert!(seen[0] > -564 && seen[0] <= BOTTOM, "{seen:?}");
}
