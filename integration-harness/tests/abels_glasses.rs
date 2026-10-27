//! Help Me Find My Glasses (2186): a bush in Nautilus Harbor gives one pair
//! of glasses at a time; Abel's own goes through the quest's end script,
//! and another pair shown to Abel completes it for less.

use integration_harness::database::set_job;
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const NAUTILUS_HARBOR: i32 = 120000000;
const ABEL: i32 = 1094001;
const BUSH: i32 = 1094003;
const FIND_MY_GLASSES: i16 = 2186;
const ABELS_GLASSES: i32 = 4031853;
const NAUTILUS_RETURN: i32 = 2030019;

/// A walk to NPC `npc`.
fn to(npc: i32) -> Packet {
    let npcs = &maps().field(NAUTILUS_HARBOR).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn a_bush_gives_glasses_and_abel_takes_them() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 20, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(
        warp(c, NAUTILUS_HARBOR, "", &mut trace).await,
        NAUTILUS_HARBOR
    );
    send(c, to(ABEL)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, FIND_MY_GLASSES, ABEL)).await;
    assert_eq!(t.states.get(&FIND_MY_GLASSES), Some(&1));

    send(c, to(BUSH)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(NAUTILUS_HARBOR, BUSH)).await;
    assert_eq!(t.prompts, [(BUSH, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    let [(glasses, 1)] = t.items[..] else {
        panic!("{t:?}");
    };
    // One pair at a time.
    let t = step(c, &m, talk(NAUTILUS_HARBOR, BUSH)).await;
    assert_eq!(t.prompts, [(BUSH, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.items.is_empty(), "{t:?}");

    send(c, to(ABEL)).await;
    barrier(c, &m, &mut trace).await;
    let (t, exp, scrolls) = if glasses == ABELS_GLASSES {
        let t = step(c, &m, build_quest_action(SCRIPT_END, FIND_MY_GLASSES, ABEL)).await;
        assert_eq!(t.prompts, [(ABEL, 0)]);
        (step(c, &m, answer(0, 1, None)).await, 1700, 10)
    } else {
        let t = step(c, &m, talk(NAUTILUS_HARBOR, ABEL)).await;
        assert_eq!(t.prompts, [(ABEL, 0)]);
        (step(c, &m, answer(0, 1, None)).await, 1000, 5)
    };
    let mut items = t.items.clone();
    items.sort();
    assert_eq!(items, [(NAUTILUS_RETURN, scrolls), (glasses, -1)]);
    assert_eq!(t.gains, [exp]);
    assert_eq!(t.states.get(&FIND_MY_GLASSES), Some(&2));
    // Abel plays the quest's `npcAct`.
    let &(abel, _) = trace
        .field
        .npcs
        .iter()
        .find(|&&(_, npc)| npc == ABEL)
        .unwrap_or_else(|| panic!("{:?}", trace.field.npcs));
    assert_eq!(t.field.npc_special_actions, [(abel, "quest".to_string())]);
}
