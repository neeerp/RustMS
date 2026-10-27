//! Title Challenge - Outstanding Citizen (29508): Dalair starts it, and
//! Dalair's end script gives the Outstanding Citizen Medal only once the
//! player is married, in a guild, and has a Junior.

use integration_harness::database::{found_guild, join_family, marry, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use packet::Packet;
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const OUTSTANDING_CITIZEN: i16 = 29508;
const OUTSTANDING_CITIZEN_MEDAL: i32 = 1142081;
const MOONSTONE_BOX: i32 = 2240000;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

async fn new_character(config: &HarnessConfig, level: i16) -> i32 {
    let id = login_to_world_session(config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, level, 50).await.unwrap();
    id
}

/// Log in, then go to Dalair in Perion.
async fn visit(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let m = &config.character_name;
    let mut trace = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    send(c, to(PERION, DALAIR)).await;
    barrier(c, m, &mut trace).await;
    session
}

#[tokio::test]
async fn dalair_gives_the_medal_to_a_married_guilded_senior() {
    let (config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let m = config.character_name.clone();
    let id = new_character(&config, 30).await;
    let other = new_character(&other_config, 15).await;

    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(
        c,
        &m,
        build_quest_action(START, OUTSTANDING_CITIZEN, DALAIR),
    )
    .await;
    assert_eq!(t.states.get(&OUTSTANDING_CITIZEN), Some(&1));
    // Not married, guildless, and no Junior: not yet.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, OUTSTANDING_CITIZEN, DALAIR),
    )
    .await;
    assert_eq!(t.prompts, [(DALAIR, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.states.is_empty() && t.items.is_empty(), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    marry(id, other, MOONSTONE_BOX).await.unwrap();
    found_guild(id, &format!("Citizen{id}")).await.unwrap();
    join_family(id, other).await.unwrap();
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, OUTSTANDING_CITIZEN, DALAIR),
    )
    .await;
    assert_eq!(t.prompts, [(DALAIR, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&OUTSTANDING_CITIZEN), Some(&2));
    assert_eq!(t.items, [(OUTSTANDING_CITIZEN_MEDAL, 1)]);
}
