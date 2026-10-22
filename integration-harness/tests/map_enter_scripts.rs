//! Map entry scripts that need the player: the Time Gate's and Mushroom
//! Castle's theme banners, and the Disciple of the Black Magician (quest
//! 2175) who comes out in his hideout for whoever hunts him, one at a time.

use integration_harness::database::set_quest;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use tokio::time::Duration;

const TIME_GATE_FOREST: i32 = 240070000;
const MUSHROOM_FOREST: i32 = 106020000;
const HIDEOUT: i32 = 100000006;
const HENESYS: i32 = 100000000;
const DISCIPLE: i32 = 9300156;

/// Go to `map` and return what came once the player was in.
async fn enter(player: &mut WorldSession, map: i32) -> Trace {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    barrier(&mut player.connection, &name, &mut trace).await;
    trace
}

fn disciples(trace: &Trace) -> usize {
    let mobs = &trace.field.mobs;
    mobs.iter().filter(|&&(_, mob)| mob == DISCIPLE).count()
}

#[tokio::test]
async fn theme_banners_show_and_the_disciple_comes_out_for_his_hunter() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;

    for (map, banner) in [
        (TIME_GATE_FOREST, "temaD/enter/teraForest"),
        (MUSHROOM_FOREST, "temaD/enter/mushCatle"),
    ] {
        let trace = enter(&mut player, map).await;
        assert_eq!(trace.field_effects, [(3, banner.to_string())], "{map}");
    }

    // Not on the quest: the hideout is empty.
    let trace = enter(&mut player, HIDEOUT).await;
    assert_eq!(disciples(&trace), 0, "{:?}", trace.field.mobs);
    let mut trace = Trace::default();
    warp(&mut player.connection, HENESYS, "", &mut trace).await;
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // On it, he comes out under the player's control, and only once.
    set_quest(id, 2175, 1, "").await.unwrap();
    let mut player = relogin(&config).await;
    let trace = enter(&mut player, HIDEOUT).await;
    assert_eq!(disciples(&trace), 1, "{:?}", trace.field.mobs);
    let (object_id, _) = *trace
        .field
        .mobs
        .iter()
        .find(|&&(_, mob)| mob == DISCIPLE)
        .unwrap();
    assert!(trace.field.controlled.contains(&object_id));
    enter(&mut player, HENESYS).await;
    let trace = enter(&mut player, HIDEOUT).await;
    assert_eq!(disciples(&trace), 1, "{:?}", trace.field.mobs);
}
