//! Monster Expert (29512): Dalair gives the medal only once the Monster
//! Book holds 30 different cards (`Check/1/mbmin`), however many copies of
//! each.

use integration_harness::database::{add_book_card, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const MONSTER_EXPERT: i16 = 29512;
const MONSTER_EXPERT_MEDAL: i32 = 1142083;
const SNAIL_CARD: i32 = 2380000;

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
async fn dalair_wants_thirty_different_cards_in_the_book() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    // 29 different cards, five of the first: 33 cards in all, but too few
    // kinds.
    add_book_card(id, SNAIL_CARD, 5).await.unwrap();
    for card in SNAIL_CARD + 1..SNAIL_CARD + 29 {
        add_book_card(id, card, 1).await.unwrap();
    }

    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_action(START, MONSTER_EXPERT, DALAIR)).await;
    assert_eq!(t.states.get(&MONSTER_EXPERT), Some(&1));
    let t = step(c, &m, build_quest_completion(MONSTER_EXPERT, DALAIR, -1)).await;
    assert!(t.states.is_empty() && t.items.is_empty(), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    add_book_card(id, SNAIL_CARD + 29, 1).await.unwrap();
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_completion(MONSTER_EXPERT, DALAIR, -1)).await;
    assert_eq!(t.states.get(&MONSTER_EXPERT), Some(&2));
    assert_eq!(t.items, [(MONSTER_EXPERT_MEDAL, 1)]);
}
