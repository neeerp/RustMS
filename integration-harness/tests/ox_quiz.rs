//! The OX Quiz (report b171): a GM's `/ox` opens it, Paul takes players to
//! its field, and the GM's `/start` there asks ten questions. Each is
//! answered by standing in the O room or the X room when its time is up:
//! a player who stays on the platform goes out to 109050001 after the
//! first, and one who answers all ten goes on to 109050000. The GM stays.

use game_data::ox_quiz::OxQuiz;
use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::hunting::walk_to;
use integration_harness::packets::{build_all_chat, build_change_map};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use tokio::time::Duration;

const SOUTHPERRY: i32 = 60000;
const PAUL: i32 = 9000000;
const FIELD: i32 = 109_020_001;
const EXIT: i32 = 109_050_001;
const WINNERS: i32 = 109_050_000;
const PINK: u8 = 5;
/// Standing on the floor of the O room and of the X room.
const O_ROOM: (i16, i16) = (-600, 334);
const X_ROOM: (i16, i16) = (200, 334);

fn assets(file: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/game-data")
        .join(file)
}

fn talk_to_paul() -> Packet {
    let maps = GameData::load_from_nx_map(assets("Map.nx")).unwrap();
    let index = maps
        .field(SOUTHPERRY)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == PAUL)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// The OX_QUIZ packets seen: (question, set, number).
fn quizzes(t: &Trace) -> Vec<(bool, u8, u16)> {
    t.ox.iter()
        .filter(|(op, _)| *op == SendOpcode::OxQuiz as i16)
        .map(|(_, b)| (b[0] == 1, b[1], u16::from_le_bytes([b[2], b[3]])))
        .collect()
}

fn questions(t: &Trace) -> Vec<(u8, u16)> {
    quizzes(t)
        .into_iter()
        .filter(|&(question, _, _)| question)
        .map(|(_, set, number)| (set, number))
        .collect()
}

/// Paul's "go".
async fn go(c: &mut MapleTestConnection, m: &str, paul: Packet) -> Trace {
    let t = step(c, m, paul).await;
    assert_eq!(t.prompts, [(PAUL, 4)], "his menu");
    step(c, m, answer(4, 1, Some(1))).await
}

#[tokio::test]
async fn the_wrong_go_out_and_who_answers_all_ten_wins() {
    let quiz = OxQuiz::load(assets("Etc.nx")).unwrap();
    let right = |set: u8, number: u16| {
        quiz.sets()
            .flatten()
            .find(|q| q.set == set && q.number == number)
            .map(|q| q.o)
            .unwrap()
    };
    let paul = talk_to_paul();
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("GM login");
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    for config in &configs {
        login_to_world_session(config).await.expect("player login");
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    send(g, build_all_chat("/ox", 0).unwrap()).await;
    barrier(g, "opened", &mut gt).await;

    // Paul takes both to the field, where the map's help shows.
    let mut players = Vec::new();
    for config in &configs {
        let mut session = relogin(config).await;
        let (c, m) = (&mut session.connection, config.character_name.as_str());
        let mut t = Trace::default();
        assert_eq!(warp(c, SOUTHPERRY, "", &mut t).await, SOUTHPERRY);
        barrier(c, m, &mut t).await;
        let t = go(c, m, paul.clone()).await;
        assert_eq!(t.warps, [FIELD], "in");
        assert_eq!(
            t.ox,
            [(SendOpcode::GmEventInstructions as i16, vec![0])],
            "the help"
        );
        players.push((session, config.character_name.clone()));
    }
    // The way to the winners' map by the spawn points (`join00`) is shut.
    let (session, m) = &mut players[0];
    send(
        &mut session.connection,
        build_change_map(-1, "join00").unwrap(),
    )
    .await;
    let mut t = Trace::default();
    barrier(&mut session.connection, m, &mut t).await;
    assert_eq!(t.blocked, [(SendOpcode::BlockedMap as i16, 1)], "shut");
    assert!(t.warps.is_empty());
    for (session, m) in &mut players {
        barrier(&mut session.connection, m, &mut Trace::default()).await;
    }
    assert_eq!(warp(g, FIELD, "", &mut gt).await, FIELD);
    barrier(g, "on the field", &mut gt).await;
    for (session, m) in &mut players {
        barrier(&mut session.connection, m, &mut Trace::default()).await;
    }

    send(g, build_all_chat("/start", 0).unwrap()).await;
    let [(stayer, _), (winner, _)] = &mut players[..] else {
        unreachable!()
    };

    // One answers every question by going to its room in time.
    let mut wt = Trace::default();
    let mut asked = 0;
    loop {
        until(&mut winner.connection, &mut wt, |t| {
            questions(t).len() > asked || !t.warps.is_empty()
        })
        .await;
        if !wt.warps.is_empty() {
            break;
        }
        let (set, number) = questions(&wt)[asked];
        let (x, y) = if right(set, number) { O_ROOM } else { X_ROOM };
        send(&mut winner.connection, walk_to(x, y)).await;
        asked += 1;
    }
    assert_eq!(asked, 10, "ten questions");
    assert_eq!(wt.warps, [WINNERS], "won");
    let set = questions(&wt)[0].0;
    assert!(questions(&wt).iter().all(|&(s, _)| s == set), "one set");
    assert_eq!(quizzes(&wt).last(), Some(&(false, set, 0)), "cleared");
    assert!(wt.broadcasts.iter().any(|(kind, _)| *kind == PINK));

    // The other never left the platform, and went out after the first.
    let mut st = Trace::default();
    until(&mut stayer.connection, &mut st, |t| !t.warps.is_empty()).await;
    assert_eq!(st.warps, [EXIT], "out");
    let first = questions(&wt)[0];
    assert_eq!(
        quizzes(&st),
        [
            (true, first.0, first.1),
            (false, first.0, first.1),
            (false, set, 0)
        ]
    );
    assert!(st.broadcasts.iter().any(|(kind, _)| *kind == PINK));

    // The GM saw it all and stays.
    barrier(g, "over", &mut gt).await;
    assert_eq!(gt.warps, [FIELD]);
    assert_eq!(questions(&gt).len(), 10);
    assert_eq!(quizzes(&gt).last(), Some(&(false, set, 0)));
    send(g, build_all_chat("/endevent", 0).unwrap()).await;
    barrier(g, "closed", &mut gt).await;
}
