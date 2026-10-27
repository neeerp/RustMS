//! Portals that open an NPC's conversation without a click: a young
//! beginner walking into a job instructor's room (`tutorialNPC`) hears the
//! instructor out, and Leafre's `TD_neo` (`TD_chat_enter`) opens the Time
//! Gate. Neither opens over a conversation already open.

use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const ROOMS: [(i32, i32); 5] = [
    (101000003, 1032001),
    (100000201, 1012100),
    (102000003, 1022000),
    (103000003, 1052001),
    (120000101, 1090000),
];
const TIME_GATE_FOREST: i32 = 240070000;
const TIME_GATE: i32 = 2083006;
const EVIL_EYE_CAVE: i32 = 105070300;
const CAVE_WALL: i32 = 1063011;

fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

async fn touch(player: &mut WorldSession, portal: &str) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, enter(portal)).await
}

async fn go(player: &mut WorldSession, map: i32) {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
}

/// Close the conversation open on a prompt of `kind`.
async fn close(player: &mut WorldSession, kind: u8) {
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, answer(kind, -1, None)).await;
    assert!(trace.prompts.is_empty(), "{:?}", trace.prompts);
}

#[tokio::test]
async fn a_young_beginner_hears_each_instructor_out_and_the_time_gate_opens() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();

    for (map, npc) in ROOMS {
        go(&mut player, map).await;
        let trace = touch(&mut player, "tutorial").await;
        let [(speaker, kind)] = trace.prompts[..] else {
            panic!("{map}: {:?}", trace.prompts);
        };
        assert_eq!(speaker, npc, "{map}");
        assert!(trace.warps.is_empty(), "{:?}", trace.warps);
        // Touched again while it is open, it doesn't start over.
        let again = touch(&mut player, "tutorial").await;
        assert!(again.prompts.is_empty(), "{map}: {:?}", again.prompts);
        close(&mut player, kind).await;
    }

    go(&mut player, TIME_GATE_FOREST).await;
    let trace = touch(&mut player, "TD_neo").await;
    assert_eq!(trace.prompts, [(TIME_GATE, 0)]);
    close(&mut player, 0).await;

    // The Puppeteer's cave wall asks for its password.
    go(&mut player, EVIL_EYE_CAVE).await;
    let trace = touch(&mut player, "in00").await;
    assert_eq!(trace.prompts, [(CAVE_WALL, 2)]);
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    close(&mut player, 2).await;
}
