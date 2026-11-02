//! A new Noblesse starts on Ereve's tutorial path (130030000) bearing the
//! mark its first quest asks for (20022): Mimo comes out at the first
//! portal, and each brother's door waits on their quest.

use integration_harness::connection::MapleTestConnection;
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const CYGNUS: i32 = 0;
const FIRST_MAP: i32 = 130_030_000;
const KIMUS_MAP: i32 = 130_030_001;
const KIZANS_MAP: i32 = 130_030_002;
const KIMU: i32 = 1102004;
const KIZAN: i32 = 1102005;
const WELCOME_TO_EREVE: i16 = 20010;
const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Walk to NPC `npc` on `map` and send a quest action to it.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

/// Click through `n` say pages.
async fn next(c: &mut MapleTestConnection, m: &str, n: usize) -> Trace {
    let mut t = Trace::default();
    for _ in 0..n {
        t = step(c, m, answer(0, 1, None)).await;
    }
    t
}

fn pink(trace: &Trace) -> bool {
    matches!(trace.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty())
}

#[tokio::test]
async fn a_new_noblesse_starts_on_ereves_tutorial_path() {
    let config = load_harness_config_or_fail().await.with_class(CYGNUS);
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    assert_eq!(session.map_id, FIRST_MAP);
    let c = &mut session.connection;

    // Mimo comes out once.
    let t = step(c, &m, enter("scr00")).await;
    assert_eq!(t.guides, [true]);
    assert!(matches!(t.guide_talk.as_slice(), [said] if !said.is_empty()));
    let t = step(c, &m, enter("scr00")).await;
    assert!(t.guides.is_empty() && t.guide_talk.is_empty());

    // Kimu's door waits for the quest Kimu gives.
    let mut trace = Trace::default();
    assert_eq!(warp(c, KIMUS_MAP, "", &mut trace).await, KIMUS_MAP);
    let t = step(c, &m, enter("west00")).await;
    assert!(t.warps.is_empty() && pink(&t), "{t:?}");

    // The mark lets Kimu offer Welcome to Ereve.
    let t = quest(
        c,
        &m,
        KIMUS_MAP,
        KIMU,
        build_quest_action(SCRIPT_START, WELCOME_TO_EREVE, KIMU),
    )
    .await;
    assert_eq!(t.prompts, [(KIMU, 0)]);
    let t = next(c, &m, 3).await;
    assert_eq!(t.prompts, [(KIMU, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&WELCOME_TO_EREVE), Some(&1));
    assert_eq!(t.guide_talk, ["#2"]);

    let t = step(c, &m, enter("west00")).await;
    assert!(
        matches!(t.arrivals.as_slice(), [(KIZANS_MAP, 0, _)]),
        "{:?}",
        t.arrivals
    );

    // Kizan gives what Kimu promised, and the next door waits on Kizan's.
    let t = quest(
        c,
        &m,
        KIZANS_MAP,
        KIZAN,
        build_quest_action(SCRIPT_END, WELCOME_TO_EREVE, KIZAN),
    )
    .await;
    assert_eq!(t.prompts, [(KIZAN, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.states.get(&WELCOME_TO_EREVE), Some(&2));
    assert_eq!(t.items, [(2000020, 5), (2000021, 5)]);
    assert_eq!(t.gains, [15]);
    assert_eq!(t.guide_talk, ["#3"]);
    let t = step(c, &m, enter("west00")).await;
    assert!(t.warps.is_empty() && pink(&t), "{t:?}");
}
