//! End scripts that take what their quest asks for and give rewards: Dr.
//! Kim's Mana Elixir Pills for a Bloctopus Key Chain (3452), Mr. Ku's tier
//! for the Bellflowers held (3833), and Muhamad convinced after four
//! answers, for a Lidium (3953).

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const OMEGA_SECTOR_HQ: i32 = 221000300;
const HERB_TOWN: i32 = 251000000;
const ARIANT_HOUSE: i32 = 260000200;
const DR_KIM: i32 = 2050001;
const MR_KU: i32 = 2092000;
const MUHAMAD: i32 = 2100001;
const BLOCTOPUS_KEY_CHAIN: i32 = 4000099;
const MANA_ELIXIR_PILL: i32 = 2000011;
const BELLFLOWER: i32 = 4000294;
const DRIED_SQUID: i32 = 2020007;
const LIDIUM: i32 = 4011008;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn dr_kim_mr_ku_and_muhamad_reward_their_quests() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 60, 50).await.unwrap();
    set_quest(id, 3952, 2, "").await.unwrap();
    grant_stack(id, 4, 1, BLOCTOPUS_KEY_CHAIN, 1).await.unwrap();
    grant_stack(id, 4, 2, BELLFLOWER, 99).await.unwrap();
    grant_stack(id, 4, 3, LIDIUM, 1).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(
        warp(c, OMEGA_SECTOR_HQ, "", &mut trace).await,
        OMEGA_SECTOR_HQ
    );
    send(c, to(OMEGA_SECTOR_HQ, DR_KIM)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, 3452, DR_KIM)).await;
    assert_eq!(t.states.get(&3452), Some(&1));
    let t = step(c, &m, build_quest_action(SCRIPT_END, 3452, DR_KIM)).await;
    assert_eq!(t.prompts, [(DR_KIM, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    let mut items = t.items.clone();
    items.sort();
    assert_eq!(items, [(MANA_ELIXIR_PILL, 50), (BLOCTOPUS_KEY_CHAIN, -1)]);
    assert_eq!(t.gains, [8000]);
    assert_eq!(t.states.get(&3452), Some(&2));

    // 99 Bellflowers: the 50 tier.
    assert_eq!(warp(c, HERB_TOWN, "", &mut trace).await, HERB_TOWN);
    send(c, to(HERB_TOWN, MR_KU)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, 3833, MR_KU)).await;
    assert_eq!(t.states.get(&3833), Some(&1));
    step(c, &m, build_quest_action(SCRIPT_END, 3833, MR_KU)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    let mut items = t.items.clone();
    items.sort();
    assert_eq!(items, [(DRIED_SQUID, 50), (BELLFLOWER, -50)]);
    assert_eq!(t.gains, [10000]);
    assert_eq!(t.states.get(&3833), Some(&2));

    assert_eq!(warp(c, ARIANT_HOUSE, "", &mut trace).await, ARIANT_HOUSE);
    send(c, to(ARIANT_HOUSE, MUHAMAD)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, 3953, MUHAMAD)).await;
    assert_eq!(t.states.get(&3953), Some(&1));
    let t = step(c, &m, build_quest_action(SCRIPT_END, 3953, MUHAMAD)).await;
    assert_eq!(t.prompts, [(MUHAMAD, 4)]);
    for _ in 0..3 {
        let t = step(c, &m, answer(4, 1, Some(0))).await;
        assert_eq!(t.prompts, [(MUHAMAD, 4)]);
    }
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MUHAMAD, 0)]);
    assert_eq!(t.items, [(LIDIUM, -1)]);
    assert_eq!(t.gains, [20000]);
    assert_eq!(t.states.get(&3953), Some(&2));
}
