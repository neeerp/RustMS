//! Title Challenge - Veteran Hunter (29400): Dalair's start writes the
//! count and the goal (`mon`, `mg`) beside the quest log, a kill of a mob
//! above the player's level adds one, and the end gives the Veteran Hunter
//! Medal once a million are hunted.

use integration_harness::database::{
    replace_item, set_job, set_quest_ex, set_str, set_weapon_attack,
};
use integration_harness::events::maps;
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const VETERAN_HUNTER: i16 = 29400;
const VETERAN_HUNTER_MEDAL: i32 = 1142004;
/// Perion's East Rocky Mountain V, with Slimes (level 6) and Octopuses
/// (level 12).
const ROCKY_MOUNTAIN: i32 = 102040000;
const SLIME: i32 = 210100;
const OCTOPUS: i32 = 1120100;

/// Log in, then go to Dalair in Perion.
async fn visit(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let m = &config.character_name;
    let mut trace = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    let npcs = &maps().field(PERION).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == DALAIR).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut trace).await;
    session
}

#[tokio::test]
async fn veteran_hunter_counts_mobs_above_the_players_level() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 10, 50).await.unwrap();

    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, VETERAN_HUNTER, DALAIR),
    )
    .await;
    assert_eq!(t.prompts, [(DALAIR, 0)], "{t:?}");
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&VETERAN_HUNTER), Some(&1), "{t:?}");
    assert_eq!(
        t.quest_records.last(),
        Some(&(VETERAN_HUNTER, "mon=0;mg=1000000".to_string())),
        "{t:?}"
    );

    // One short of the goal, a Slime below the player's level doesn't
    // count, and an Octopus above it does.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest_ex(id, VETERAN_HUNTER, "mon=999999;mg=1000000")
        .await
        .unwrap();
    // A sword strong enough to kill either in one blow.
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, ROCKY_MOUNTAIN, "", &mut trace).await,
        ROCKY_MOUNTAIN
    );
    until(c, &mut trace, |t| {
        !t.field.alive(SLIME).is_empty() && !t.field.alive(OCTOPUS).is_empty()
    })
    .await;
    let slime = trace.field.alive(SLIME)[0];
    send(c, attack(slime, 500)).await;
    until(c, &mut trace, |t| t.field.killed.contains(&slime)).await;
    barrier(c, &m, &mut trace).await;
    assert!(trace.quest_records.is_empty(), "{trace:?}");
    let octopus = trace.field.alive(OCTOPUS)[0];
    send(c, attack(octopus, 500)).await;
    until(c, &mut trace, |t| {
        t.quest_records.last() == Some(&(VETERAN_HUNTER, "mon=1000000;mg=1000000".to_string()))
    })
    .await;

    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, VETERAN_HUNTER, DALAIR),
    )
    .await;
    assert_eq!(t.prompts, [(DALAIR, 0)], "{t:?}");
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&VETERAN_HUNTER), Some(&2), "{t:?}");
    assert_eq!(t.items, [(VETERAN_HUNTER_MEDAL, 1)], "{t:?}");
}
