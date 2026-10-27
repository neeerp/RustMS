//! Gathering Some Strange Information (21733): the Puppeteer calls an Aran
//! to Lith Harbor's Info Shop in Tru's name and waits there. His fall sets
//! the record the quest's completion reads, and Tru, the real one, teaches
//! Polearm Mastery.

use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::events::maps;
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const LITH_HARBOR: i32 = 104000000;
const INFO_SHOP: i32 = 104000004;
const FAKE_TRU: i32 = 1204007;
const TRU: i32 = 1002104;
const STRANGE_INFORMATION: i16 = 21733;
const RECORD: i16 = 21762;
const PUPPETEER: i32 = 9300345;
const POLEARM_MASTERY: i32 = 21100000;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn the_puppeteer_waits_in_the_info_shop_and_tru_rewards_his_fall() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2110, 31, 50).await.unwrap();
    replace_item(id, -1, -11, 1442000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    set_quest(id, 21732, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Before the call, the shop is quiet.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    read_for(c, &mut trace, 500).await;
    assert!(trace.field.alive(PUPPETEER).is_empty());

    // The call comes wherever the Aran is (the quest alert).
    assert_eq!(warp(c, LITH_HARBOR, "", &mut trace).await, LITH_HARBOR);
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, STRANGE_INFORMATION, FAKE_TRU),
    )
    .await;
    assert_eq!(t.prompts, [(FAKE_TRU, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&STRANGE_INFORMATION), Some(&1));

    // Not beaten yet: Tru can't end it.
    let mut trace = Trace::default();
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    until(c, &mut trace, |t| t.field.alive(PUPPETEER).len() == 1).await;
    send(c, to(INFO_SHOP, TRU)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, STRANGE_INFORMATION, TRU),
    )
    .await;
    assert!(t.prompts.is_empty() && t.states.is_empty(), "{t:?}");

    let puppeteer = trace.field.alive(PUPPETEER)[0];
    for _ in 0..20 {
        send(c, attack(puppeteer, 30_000)).await;
        read_for(c, &mut trace, 100).await;
        if trace.field.killed.contains(&puppeteer) {
            break;
        }
    }
    assert_eq!(trace.kills[&RECORD], "2");

    // Beaten, he doesn't come back.
    assert_eq!(warp(c, LITH_HARBOR, "", &mut trace).await, LITH_HARBOR);
    let mut trace = Trace::default();
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    read_for(c, &mut trace, 500).await;
    assert!(trace.field.alive(PUPPETEER).is_empty());

    send(c, to(INFO_SHOP, TRU)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, STRANGE_INFORMATION, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&STRANGE_INFORMATION), Some(&2));
    assert_eq!(t.gains, [8000]);
    assert_eq!(t.skills, [(POLEARM_MASTERY, 0)]);
}
