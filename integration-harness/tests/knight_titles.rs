//! The Knight titles (29906 to 29909): a Cygnus Knight's start script starts
//! its quest wherever the player is, with no dialogue, and Cygnus completes
//! it in the quest window with the title's medal.

use integration_harness::database::set_job;
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HENESYS: i32 = 100000000;
const EREVE: i32 = 130000000;
const CYGNUS: i32 = 1101000;
const KNIGHT_IN_TRAINING: i16 = 29906;
const KNIGHT_IN_TRAINING_MEDAL: i32 = 1142066;
const OFFICIAL_KNIGHT: i16 = 29907;

#[tokio::test]
async fn a_knight_title_starts_anywhere_and_cygnus_gives_its_medal() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    // A Dawn Warrior at level 30, before the 2nd job.
    set_job(id, 1100, 30, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, HENESYS, "", &mut trace).await, HENESYS);
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, KNIGHT_IN_TRAINING, CYGNUS),
    )
    .await;
    assert!(t.prompts.is_empty(), "{t:?}");
    assert_eq!(t.states.get(&KNIGHT_IN_TRAINING), Some(&1), "{t:?}");
    assert_eq!(t.info_texts.len(), 1, "{t:?}");
    // Not before the 2nd job.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, OFFICIAL_KNIGHT, CYGNUS),
    )
    .await;
    assert!(t.states.is_empty(), "{t:?}");

    assert_eq!(warp(c, EREVE, "", &mut trace).await, EREVE);
    let npcs = &maps().field(EREVE).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == CYGNUS).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(KNIGHT_IN_TRAINING, CYGNUS, -1),
    )
    .await;
    assert_eq!(t.states.get(&KNIGHT_IN_TRAINING), Some(&2), "{t:?}");
    assert_eq!(t.items, [(KNIGHT_IN_TRAINING_MEDAL, 1)], "{t:?}");
}
