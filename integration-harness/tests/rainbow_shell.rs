//! Pia's "A rainbow snail shell that makes wishes come true!?" (2156): Pia
//! takes the shell from Mano, the quest ends with its rewards, and Pia plays
//! the shell's effect (NPC special action `act2156`, 0x106 to the NPCs'
//! controller).

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HENESYS_PARK: i32 = 100000200;
const PIA: i32 = 1012102;
const SNAIL_HUNT: i16 = 2155;
const RAINBOW_SHELL_QUEST: i16 = 2156;
const RAINBOW_SNAIL_SHELL: i32 = 2210006;

#[tokio::test]
async fn pia_takes_the_rainbow_snail_shell() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 100).await.unwrap();
    set_quest(id, SNAIL_HUNT, 2, "").await.unwrap();
    grant_stack(id, 2, 1, RAINBOW_SNAIL_SHELL, 1).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, HENESYS_PARK, "", &mut trace).await, HENESYS_PARK);
    // Pia's object id, from the spawns that follow the warp.
    let npcs = &maps().field(HENESYS_PARK).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == PIA).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, &m, &mut trace).await;
    let &(pia, _) = trace
        .field
        .npcs
        .iter()
        .find(|&&(_, npc)| npc == PIA)
        .unwrap_or_else(|| panic!("{:?}", trace.field.npcs));

    let t = step(c, &m, build_quest_action(START, RAINBOW_SHELL_QUEST, PIA)).await;
    assert_eq!(t.states.get(&RAINBOW_SHELL_QUEST), Some(&1));

    // Pia takes the shell, the quest ends with its rewards, and the lone
    // player, the NPCs' controller, is asked to have Pia play act2156.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, RAINBOW_SHELL_QUEST, PIA),
    )
    .await;
    assert_eq!(t.prompts, [(PIA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&RAINBOW_SHELL_QUEST), Some(&2));
    assert_eq!(t.removed, [(2, 1)]);
    assert_eq!(t.gains, [7500]);
    assert_eq!(t.meso_gains, [30000]);
    assert_eq!(t.fame_gains, [3]);
    assert_eq!(t.field.npc_special_actions, [(pia, "act2156".to_string())]);
}
