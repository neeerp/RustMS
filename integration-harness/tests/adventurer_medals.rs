//! The Adventurer titles (29900 to 29903): each start script starts its
//! quest wherever the player is, with no dialogue, and Dalair's end script
//! gives the title's medal.

use integration_harness::database::set_job;
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const HENESYS: i32 = 100000000;
const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
/// The titles open to a level 30 Fighter: quest and medal.
const TITLES: [(i16, i32); 2] = [(29900, 1142107), (29901, 1142108)];
const VETERAN_ADVENTURER: i16 = 29902;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn titles_start_anywhere_and_dalair_gives_the_medals() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 30, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Henesys's Dalair is not on this map.
    assert_eq!(warp(c, HENESYS, "", &mut trace).await, HENESYS);
    barrier(c, &m, &mut trace).await;
    for (quest, _) in TITLES {
        let t = step(c, &m, build_quest_action(SCRIPT_START, quest, DALAIR)).await;
        assert!(t.prompts.is_empty(), "{t:?}");
        assert_eq!(t.states.get(&quest), Some(&1), "{quest}");
        assert_eq!(t.info_texts.len(), 1, "{t:?}");
    }
    // Not before level 70.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, VETERAN_ADVENTURER, DALAIR),
    )
    .await;
    assert!(t.states.is_empty(), "{t:?}");

    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    send(c, to(PERION, DALAIR)).await;
    barrier(c, &m, &mut trace).await;
    for (quest, medal) in TITLES {
        let t = step(c, &m, build_quest_action(SCRIPT_END, quest, DALAIR)).await;
        assert_eq!(t.prompts, [(DALAIR, 0)], "{quest}");
        let t = step(c, &m, answer(0, 1, None)).await;
        assert_eq!(t.states.get(&quest), Some(&2), "{quest}");
        assert_eq!(t.items, [(medal, 1)], "{quest}");
    }
}
