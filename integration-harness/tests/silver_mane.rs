//! Silver Mane (6006): Kenta starts it only for a mount of level 3 or more
//! (`tamingmoblevelmin`), and completes it for the Hog, the materials and
//! the fee, giving the Silver Mane.

use integration_harness::database::{
    grant_item, grant_stack, set_job, set_meso, set_mount, set_quest,
};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use tokio::time::Duration;

const AQUARIUM_ZOO: i32 = 230_000_003;
const KENTA: i32 = 2_060_005;
const SILVER_MANE: i16 = 6006;
const HOG: i32 = 1_902_000;
const SILVER_MANE_MOUNT: i32 = 1_902_001;
const MATERIALS: [(i32, i16); 3] = [(4_000_264, 600), (4_000_266, 400), (4_000_267, 400)];

/// Log in and walk to Kenta.
async fn visit(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let m = &config.character_name;
    assert_eq!(
        warp(c, AQUARIUM_ZOO, "", &mut Trace::default()).await,
        AQUARIUM_ZOO
    );
    let npcs = &maps().field(AQUARIUM_ZOO).unwrap().map_npcs;
    let kenta = npcs.iter().find(|n| n.npc_id == KENTA).unwrap();
    step(c, m, walk_to(kenta.x, kenta.y)).await;
    session
}

#[tokio::test]
async fn kenta_gives_the_silver_mane_to_a_rider_of_level_three() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 112, 120, 50).await.unwrap();
    set_quest(id, 6003, 2, "").await.unwrap();
    set_mount(id, 2, 0, 0).await.unwrap();
    grant_item(id, 1, 1, HOG).await.unwrap();
    for (slot, (item, count)) in (1..).zip(MATERIALS) {
        grant_stack(id, 4, slot, item, count).await.unwrap();
    }
    set_meso(id, 50_000_000).await.unwrap();

    // A mount of level 2 is too young.
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_action(START, SILVER_MANE, KENTA)).await;
    assert!(t.progress.is_empty(), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_mount(id, 3, 0, 0).await.unwrap();
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_action(START, SILVER_MANE, KENTA)).await;
    assert_eq!(t.progress, [(SILVER_MANE, 1)]);
    let t = step(c, &m, build_quest_completion(SILVER_MANE, KENTA, -1)).await;
    assert_eq!(t.progress, [(SILVER_MANE, 2)]);
    assert!(t.items.contains(&(HOG, -1)), "{t:?}");
    assert!(t.items.contains(&(SILVER_MANE_MOUNT, 1)), "{t:?}");
    for (item, count) in MATERIALS {
        assert!(t.items.contains(&(item, -i32::from(count))), "{t:?}");
    }
    assert_eq!(t.meso_gains, [-50_000_000]);
}
