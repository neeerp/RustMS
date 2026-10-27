//! Exploring Mushroom Forest (1) (2314) and Over the Castle Walls (2)
//! (2322): the Secretary of Domestic Affairs sends the player to look at
//! the forest's barrier and at the castle wall. Touching `investigate1`
//! beside the barrier, or `investigate2` at the wall (which `gotocastle`
//! leads to while 2322 is under way), sets the quest's record to 1 and has
//! the player think it over under a balloon, once; the Secretary then
//! completes the quest.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{set_job, set_quest};
use integration_harness::events::said;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START,
};
use integration_harness::trace::{send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MUSHROOM_FOREST_FIELD: i32 = 106_020_000;
const SECRETARY: i32 = 1_300_003;
const SECRETARY_AT: (i16, i16) = (265, 35);
const EXPLORING_MUSHROOM_FOREST: i16 = 2314;
const OVER_THE_CASTLE_WALLS: i16 = 2322;
/// The barrier's map and its `investigate1` portal.
const DEEP_INSIDE: i32 = 106_020_300;
const BARRIER: (i16, i16) = (1113, -11);
/// The way to the castle (`right00`, script `gotocastle`) and the wall's
/// edge before its thorns are cleared, with its `investigate2` portal.
const SPLIT_ROAD: i32 = 106_020_400;
const GOTOCASTLE: (i16, i16) = (1388, -24);
const THORNY_WALL_EDGE: i32 = 106_020_500;
const WALL: (i16, i16) = (258, -14);
const PLAYER_SPEAKS: u8 = 2;

/// CHANGE_MAP_SPECIAL: walk into the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// The Secretary's start script for `quest`: yes, then read on.
async fn sent(c: &mut MapleTestConnection, m: &str, quest: i16) -> Trace {
    let mut seen = Trace::default();
    assert_eq!(
        warp(c, MUSHROOM_FOREST_FIELD, "", &mut seen).await,
        MUSHROOM_FOREST_FIELD
    );
    send(c, walk_to(SECRETARY_AT.0, SECRETARY_AT.1)).await;
    let t = step(c, m, build_quest_action(SCRIPT_START, quest, SECRETARY)).await;
    assert_eq!(t.prompts, [(SECRETARY, 1)], "asked");
    let t = step(c, m, answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(SECRETARY, 0)], "told where to go");
    step(c, m, answer(0, 1, None)).await
}

/// Touch `portal` at `at`: the record turns to 1 and the player thinks
/// `key` under balloon `balloon`; touched again, nothing.
async fn look(
    c: &mut MapleTestConnection,
    m: &str,
    quest: i16,
    at: (i16, i16),
    portal: &str,
    key: &str,
    balloon: u8,
) {
    send(c, walk_to(at.0, at.1)).await;
    let t = step(c, m, enter(portal)).await;
    assert_eq!(t.kills.get(&quest).map(String::as_str), Some("001"));
    assert_eq!(t.prompt_speakers, [PLAYER_SPEAKS]);
    assert!(said(&t, key), "{:?}", t.prompt_texts);
    assert_eq!(
        t.images,
        [format!(
            "Effect/OnUserEff.img/normalEffect/mushroomcastle/chatBalloon{balloon}"
        )]
    );
    assert!(t.warps.is_empty());
    step(c, m, answer(0, 1, None)).await;
    let t = step(c, m, enter(portal)).await;
    assert!(
        t.prompts.is_empty() && t.kills.is_empty() && t.images.is_empty(),
        "{t:?}"
    );
}

/// The Secretary completes `quest` for `exp`.
async fn report(c: &mut MapleTestConnection, m: &str, quest: i16, exp: i32) {
    let mut seen = Trace::default();
    assert_eq!(
        warp(c, MUSHROOM_FOREST_FIELD, "", &mut seen).await,
        MUSHROOM_FOREST_FIELD
    );
    send(c, walk_to(SECRETARY_AT.0, SECRETARY_AT.1)).await;
    let t = step(c, m, build_quest_completion(quest, SECRETARY, -1)).await;
    assert_eq!(t.states.get(&quest), Some(&2));
    assert_eq!(t.gains, [exp]);
}

#[tokio::test]
async fn the_secretary_sends_the_player_to_look_at_the_barrier_and_the_wall() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    set_quest(id, 2313, 2, "").await.unwrap();
    set_quest(id, 2321, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();

    let t = sent(c, &m, EXPLORING_MUSHROOM_FOREST).await;
    assert_eq!(t.progress, [(EXPLORING_MUSHROOM_FOREST, 1)]);
    assert_eq!(warp(c, DEEP_INSIDE, "", &mut seen).await, DEEP_INSIDE);
    look(
        c,
        &m,
        EXPLORING_MUSHROOM_FOREST,
        BARRIER,
        "investigate1",
        keys::MC_BARRIER_INVESTIGATED,
        1,
    )
    .await;
    report(c, &m, EXPLORING_MUSHROOM_FOREST, 8300).await;

    // The thorns keep the player back until 2322 has them look at the wall.
    assert_eq!(warp(c, SPLIT_ROAD, "", &mut seen).await, SPLIT_ROAD);
    send(c, walk_to(GOTOCASTLE.0, GOTOCASTLE.1)).await;
    let t = step(c, &m, enter("right00")).await;
    assert!(t.warps.is_empty());
    let t = sent(c, &m, OVER_THE_CASTLE_WALLS).await;
    assert_eq!(t.progress, [(OVER_THE_CASTLE_WALLS, 1)]);
    assert_eq!(warp(c, SPLIT_ROAD, "", &mut seen).await, SPLIT_ROAD);
    send(c, walk_to(GOTOCASTLE.0, GOTOCASTLE.1)).await;
    let t = step(c, &m, enter("right00")).await;
    assert_eq!(t.warps, [THORNY_WALL_EDGE]);
    look(
        c,
        &m,
        OVER_THE_CASTLE_WALLS,
        WALL,
        "investigate2",
        keys::MC_WALLS_INVESTIGATED,
        2,
    )
    .await;
    report(c, &m, OVER_THE_CASTLE_WALLS, 11000).await;
}
