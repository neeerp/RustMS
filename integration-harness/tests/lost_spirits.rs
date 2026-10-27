//! Adonis's "Lost Spirits" (8255), which completes as Adonis tells it, and
//! "The Seed That Swallow Lost" (3609), which Swallow completes with a new
//! seed when talked to again.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{set_job, set_quest};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_START, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const EL_NATH: i32 = 211000000;
const ADONIS: i32 = 9201106;
const KOREAN_FOLK_TOWN: i32 = 222000000;
const SWALLOW: i32 = 2071006;
const LOST_SPIRITS: i16 = 8255;
const THE_LOST_SEED: i16 = 3606;
const THE_SEED_SWALLOW_LOST: i16 = 3609;
const SWALLOWS_SEED: i32 = 4031225;

/// Walk to NPC `npc` on `map` and send `action` to it.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

#[tokio::test]
async fn adonis_tells_of_the_spirits_and_swallow_finds_another_seed() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 40, 100).await.unwrap();
    set_quest(id, THE_LOST_SEED, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Adonis's tale completes Lost Spirits, with its 500 EXP.
    assert_eq!(warp(c, EL_NATH, "", &mut trace).await, EL_NATH);
    let t = quest(
        c,
        &m,
        EL_NATH,
        ADONIS,
        build_quest_action(SCRIPT_START, LOST_SPIRITS, ADONIS),
    )
    .await;
    assert_eq!(t.prompts, [(ADONIS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(ADONIS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&LOST_SPIRITS), Some(&2));
    assert_eq!(t.gains, [500]);

    // Without the seed, Swallow is asked for another, and gives it when
    // talked to again.
    assert_eq!(
        warp(c, KOREAN_FOLK_TOWN, "", &mut trace).await,
        KOREAN_FOLK_TOWN
    );
    let t = quest(
        c,
        &m,
        KOREAN_FOLK_TOWN,
        SWALLOW,
        build_quest_action(START, THE_SEED_SWALLOW_LOST, SWALLOW),
    )
    .await;
    assert_eq!(t.states.get(&THE_SEED_SWALLOW_LOST), Some(&1));
    let t = step(c, &m, talk(KOREAN_FOLK_TOWN, SWALLOW)).await;
    assert_eq!(t.prompts, [(SWALLOW, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(SWALLOWS_SEED, 1)]);
    assert_eq!(t.states.get(&THE_SEED_SWALLOW_LOST), Some(&2));
}
