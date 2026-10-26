//! A GM's `!jail`: the player goes to the jail map and stays there, however
//! they leave it and across a relog, until `!unjail` or their time is up,
//! when they come out at the cellar's door. Offline players are jailed too,
//! and the time adds up.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{date_jail, set_gm_level};
use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::build::world::messaging::Broadcast;
use packet::Packet;
use tokio::time::Duration;

const JAIL: i32 = 300000012;
const CELLAR: i32 = 300000010;

fn chat(line: &str) -> Packet {
    integration_harness::packets::build_all_chat(line, 0).unwrap()
}

fn pink(line: impl Into<String>) -> (u8, String) {
    (Broadcast::Pink as u8, line.into())
}

/// Read until the player has changed maps `count` more times.
async fn warped(c: &mut MapleTestConnection, trace: &mut Trace, count: usize) {
    let until = trace.warps.len() + count;
    while trace.warps.len() < until {
        read(c, trace).await;
    }
}

#[tokio::test]
async fn a_jailed_player_stays_in_jail_until_let_out() {
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
    let (me, them) = (&config.character_name, &other_config.character_name);

    let mut gm = relogin(&config).await;
    let mut player = relogin(&other_config).await;
    let them_id = player.character_id;
    let g = &mut gm.connection;
    let p = &mut player.connection;

    // In they go.
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    send(g, chat(&format!("!jail {} 10", them.to_lowercase()))).await;
    warped(p, &mut pt, 1).await;
    barrier(g, "gm done", &mut gt).await;
    barrier(p, "player done", &mut pt).await;
    assert_eq!(
        gt.broadcasts,
        [pink(format!("Jailed {them} for 10 minutes (10 in all)."))]
    );
    assert_eq!(pt.warps, [JAIL]);
    assert_eq!(
        pt.broadcasts,
        [pink(
            "A GM put you in jail. You will be let out in 10 minutes."
        )]
    );

    // Out by a warp, and straight back.
    let mut pt = Trace::default();
    send(p, build_change_map(100000000, "").unwrap()).await;
    warped(p, &mut pt, 2).await;
    assert_eq!(pt.warps, [100000000, JAIL]);

    // More time adds to what is left; GMs of the GM's rank can't be jailed.
    let mut gt = Trace::default();
    send(g, chat(&format!("!jail {them} 5"))).await;
    send(g, chat(&format!("!jail {me}"))).await;
    send(g, chat("!jail nobody_here")).await;
    send(g, chat(&format!("!jail {them} 0"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [
            pink(format!("Jailed {them} for 5 minutes (15 in all).")),
            pink(format!("{me} is your rank or higher.")),
            pink("There is no character named nobody_here."),
            pink("Usage: !jail <name> [minutes]"),
        ]
    );

    // A relog starts in jail.
    drop(player);
    let mut player = relogin(&other_config).await;
    assert_eq!(player.map_id, JAIL);
    let p = &mut player.connection;

    // Let out, at the cellar's door.
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    send(g, chat(&format!("!unjail {them}"))).await;
    warped(p, &mut pt, 1).await;
    send(g, chat(&format!("!unjail {them}"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [
            pink(format!("Let {them} out of jail.")),
            pink(format!("{them} is not in jail.")),
        ]
    );
    assert_eq!(pt.warps, [CELLAR]);
    assert_eq!(pt.broadcasts, [pink("You are free to go.")]);
    assert_eq!(pt.arrivals.last().map(|a| a.1), Some(3), "at in01");

    // Jailed while away, they log in to jail and come out when time is up.
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut gt = Trace::default();
    send(g, chat(&format!("!jail {them}"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [pink(format!("Jailed {them} for 5 minutes (5 in all)."))]
    );
    date_jail(them_id, 2).await.expect("end the jailing soon");
    let mut player = relogin(&other_config).await;
    assert_eq!(player.map_id, JAIL);
    let mut pt = Trace::default();
    warped(&mut player.connection, &mut pt, 1).await;
    assert_eq!(pt.warps, [CELLAR]);
    assert_eq!(pt.broadcasts, [pink("You are free to go.")]);
}
