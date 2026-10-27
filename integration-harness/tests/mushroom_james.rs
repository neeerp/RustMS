//! James's Whereabouts (2325-2327) and Killer Mushroom Spores, Again
//! (2338): the Head Security Officer sends the player to find his brother
//! James, whose end script has the player answer him; James then takes a
//! Mushroom Disguise and starts his escape in his script. Scarrs gives a
//! spare Killer Mushroom Spore to one who used theirs, completing 2338 as
//! it starts.

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const MUSHROOM_FOREST_FIELD: i32 = 106020000;
const JAMES_ROOM: i32 = 106021201;
const OFFICER: i32 = 1300005;
const SCARRS: i32 = 1300007;
const JAMES: i32 = 1300008;
const MUSHROOM_DISGUISE: i32 = 4001317;
const KILLER_MUSHROOM_SPORE: i32 = 2430014;
const PLAYER_SPEAKS: u8 = 2;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn james_is_found_and_scarrs_has_a_spare_spore() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    set_quest(id, 2323, 2, "").await.unwrap();
    set_quest(id, 2318, 2, "").await.unwrap();
    grant_stack(id, 4, 1, MUSHROOM_DISGUISE, 1).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(
        warp(c, MUSHROOM_FOREST_FIELD, "", &mut trace).await,
        MUSHROOM_FOREST_FIELD
    );
    send(c, to(MUSHROOM_FOREST_FIELD, OFFICER)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, 2325, OFFICER)).await;
    assert_eq!(t.states.get(&2325), Some(&1));

    // Scarrs's spare, while 2322 hasn't started.
    send(c, to(MUSHROOM_FOREST_FIELD, SCARRS)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, 2338, SCARRS)).await;
    assert_eq!(t.prompts, [(SCARRS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(KILLER_MUSHROOM_SPORE, 1)]);
    assert_eq!(t.states.get(&2338), Some(&2));

    assert_eq!(warp(c, JAMES_ROOM, "", &mut trace).await, JAMES_ROOM);
    send(c, to(JAMES_ROOM, JAMES)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_END, 2325, JAMES)).await;
    assert_eq!(t.prompts, [(JAMES, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompt_speakers, [PLAYER_SPEAKS], "the player answers");
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&2325), Some(&2));
    assert_eq!(t.gains, [6000]);

    let t = step(c, &m, build_quest_action(START, 2326, JAMES)).await;
    assert_eq!(t.states.get(&2326), Some(&1));
    let t = step(c, &m, build_quest_completion(2326, JAMES, -1)).await;
    assert_eq!(t.states.get(&2326), Some(&2));
    assert_eq!(t.items, [(MUSHROOM_DISGUISE, -1)]);

    let t = step(c, &m, build_quest_action(SCRIPT_START, 2327, JAMES)).await;
    assert_eq!(t.prompts, [(JAMES, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(2327, 1)]);
    let t = step(c, &m, build_quest_completion(2327, JAMES, -1)).await;
    assert_eq!(t.states.get(&2327), Some(&2));
    assert_eq!(t.gains, [8000]);
}
