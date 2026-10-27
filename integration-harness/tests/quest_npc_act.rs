//! A quest whose completion names an NPC special action (`Act/1/npcAct`):
//! "Restoring the Spirit of the Snow Sculpture" (3110) ends from the data at
//! the Snow Sculpture, which plays `act3110` (0x106 to the NPCs'
//! controller).

use integration_harness::database::{set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const EL_NATH_HOUSE: i32 = 211000100;
const ALCASTER: i32 = 2020005;
const SNOWFIELD: i32 = 211040102;
const SNOW_SCULPTURE: i32 = 2020012;
const SNOWFIELD_GIANT: i16 = 3109;
const SNOW_SCULPTURE_QUEST: i16 = 3110;
const SPIRIT: i32 = 4031791;

#[tokio::test]
async fn the_snow_sculpture_plays_its_action_on_completion() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 50, 100).await.unwrap();
    set_quest(id, SNOWFIELD_GIANT, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, EL_NATH_HOUSE, "", &mut trace).await, EL_NATH_HOUSE);
    let npcs = &maps().field(EL_NATH_HOUSE).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == ALCASTER).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, &m, &mut Trace::default()).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, SNOW_SCULPTURE_QUEST, ALCASTER),
    )
    .await;
    assert_eq!(t.states.get(&SNOW_SCULPTURE_QUEST), Some(&1));
    assert_eq!(t.items, [(SPIRIT, 1)]);

    // The sculpture's object id, from the spawns that follow the warp.
    let mut trace = Trace::default();
    assert_eq!(warp(c, SNOWFIELD, "", &mut trace).await, SNOWFIELD);
    let npcs = &maps().field(SNOWFIELD).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == SNOW_SCULPTURE).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, &m, &mut trace).await;
    let &(sculpture, _) = trace
        .field
        .npcs
        .iter()
        .find(|&&(_, npc)| npc == SNOW_SCULPTURE)
        .unwrap_or_else(|| panic!("{:?}", trace.field.npcs));

    // The quest ends from the data, and the lone player, the NPCs'
    // controller, is asked to have the sculpture play act3110. The field
    // checks the NPC is near before the quest ends, so the step's own
    // barrier can come back first; one more makes sure.
    let mut t = step(
        c,
        &m,
        build_quest_completion(SNOW_SCULPTURE_QUEST, SNOW_SCULPTURE, -1),
    )
    .await;
    barrier(c, &m, &mut t).await;
    assert_eq!(t.states.get(&SNOW_SCULPTURE_QUEST), Some(&2));
    assert_eq!(t.items, [(SPIRIT, -1)]);
    assert_eq!(t.fame_gains, [20]);
    assert_eq!(
        t.field.npc_special_actions,
        [(sculpture, "act3110".to_string())]
    );
}
