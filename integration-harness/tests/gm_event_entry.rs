//! A GM's event on their channel (report b170): until a GM opens one an
//! Event Assistant has nothing to take players to; a GM's `/snowball`
//! opens one and tells the channel, and Paul then takes each player to
//! their team's lane on the Snowball field, Story's `st00` and Maple's
//! `st01`; past the GM's limit, or after `/endevent`, he takes no more.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::packets::build_all_chat;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const SOUTHPERRY: i32 = 60000;
const PAUL: i32 = 9000000;
const FIELD: i32 = 109_060_000;
/// `st00` and `st01` on the Snowball fields.
const LANES: [u8; 2] = [2, 3];
const NOTICE: u8 = 0;

/// The tests share the channel's one event, so they take turns.
static EVENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// Talk to Paul and pick "go"; what followed.
async fn ask_paul(c: &mut MapleTestConnection, m: &str) -> Trace {
    let t = step(c, m, talk(SOUTHPERRY, PAUL)).await;
    assert_eq!(t.prompts, [(PAUL, 4)], "his menu");
    step(c, m, answer(4, 1, Some(1))).await
}

#[tokio::test]
async fn paul_takes_players_to_their_lane_while_a_gms_event_is_open() {
    let _turn = EVENT.lock().await;
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("GM login");
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    for config in &configs {
        login_to_world_session(config).await.expect("player login");
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let mut players = Vec::new();
    for config in &configs {
        players.push((relogin(config).await, config.character_name.clone()));
    }
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    for (session, m) in &mut players {
        let c = &mut session.connection;
        let mut t = Trace::default();
        assert_eq!(warp(c, SOUTHPERRY, "", &mut t).await, SOUTHPERRY);
        barrier(c, m, &mut t).await;
    }

    // Nothing open yet: Paul says so.
    let (first, m) = &mut players[0];
    let t = ask_paul(&mut first.connection, m).await;
    assert!(t.warps.is_empty());
    assert_eq!(t.prompts, [(PAUL, 0)], "not open");

    // The GM opens Snowball to two, and the channel is told.
    send(g, build_all_chat("/snowball 2", 0).unwrap()).await;
    let mut pt = Trace::default();
    until(&mut first.connection, &mut pt, |t| {
        t.broadcasts.iter().any(|(kind, _)| *kind == NOTICE)
    })
    .await;
    barrier(g, "opened", &mut gt).await;

    // The first goes to Story's lane, the second to Maple's, and the
    // field puts each on the team whose lane they stand in.
    let mut ids = Vec::new();
    for (i, (session, m)) in players.iter_mut().take(2).enumerate() {
        let t = ask_paul(&mut session.connection, m).await;
        assert_eq!(t.arrivals, [(FIELD, LANES[i], None)], "player {i}");
        ids.push(session.character_id);
    }
    let (first, _) = &mut players[0];
    until(&mut first.connection, &mut pt, |t| {
        t.players.iter().any(|s| s.character_id == ids[1])
    })
    .await;
    let seen = pt.players.iter().find(|s| s.character_id == ids[1]);
    assert_eq!(seen.unwrap().team, 1, "Maple");

    // The third finds it full.
    let (third, m) = &mut players[2];
    let t = ask_paul(&mut third.connection, m).await;
    assert!(t.warps.is_empty());
    assert_eq!(t.prompts, [(PAUL, 0)], "full");

    // Opened again, then closed: still no way in.
    send(g, build_all_chat("/snowball", 0).unwrap()).await;
    send(g, build_all_chat("/endevent", 0).unwrap()).await;
    barrier(g, "closed", &mut gt).await;
    let t = ask_paul(&mut third.connection, m).await;
    assert!(t.warps.is_empty());
    assert_eq!(t.prompts, [(PAUL, 0)], "closed");
}

const COCONUT: i32 = 109_080_000;
const HARRY: i32 = 9000012;
const PIETRA: i32 = 9000010;
const LEAVING: i32 = 109_050_001;

/// Harry takes a player out of Coconut Season back to Southperry and
/// gives their place back, so another can take it; Pietra, where the
/// game's losers go, sends them back there too.
#[tokio::test]
async fn harry_and_pietra_send_players_back_where_they_came_from() {
    let _turn = EVENT.lock().await;
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("GM login");
    let player = load_harness_config_or_fail().await;
    login_to_world_session(&player).await.expect("player login");
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let mut session = relogin(&player).await;
    let (g, c, m) = (
        &mut gm.connection,
        &mut session.connection,
        player.character_name.as_str(),
    );
    let mut t = Trace::default();
    assert_eq!(warp(c, SOUTHPERRY, "", &mut t).await, SOUTHPERRY);
    barrier(c, m, &mut t).await;

    // Room for one: in, out by Harry, and in again on the place he gave
    // back.
    let mut gt = Trace::default();
    send(g, build_all_chat("/coconut 1", 0).unwrap()).await;
    barrier(g, "opened", &mut gt).await;
    for _ in 0..2 {
        let t = ask_paul(c, m).await;
        assert_eq!(t.warps, [COCONUT], "in");
        let t = step(c, m, talk(COCONUT, HARRY)).await;
        assert_eq!(t.prompts, [(HARRY, 1)], "leave?");
        let t = step(c, m, answer(1, 1, None)).await;
        assert_eq!(t.warps, [SOUTHPERRY], "out");
    }

    // After a game, Pietra sends them back from where it put them.
    let t = ask_paul(c, m).await;
    assert_eq!(t.warps, [COCONUT]);
    let mut t = Trace::default();
    assert_eq!(warp(c, LEAVING, "", &mut t).await, LEAVING);
    barrier(c, m, &mut t).await;
    let t = step(c, m, talk(LEAVING, PIETRA)).await;
    assert_eq!(t.prompts, [(PIETRA, 0)]);
    let t = step(c, m, answer(0, 1, None)).await;
    assert_eq!(t.warps, [SOUTHPERRY], "back");
    send(g, build_all_chat("/endevent", 0).unwrap()).await;
    barrier(g, "closed", &mut gt).await;
}
