//! The Aran's trail from the Puppeteer to Orbis (21734-21736): Tru sends
//! the Aran down the Secret Passage, whose end statue lets them into the
//! Puppeteer's arena, where the Puppeteer waits; its fall earns Combo
//! Drain. Tru then hands over the Seal Stone of Victoria Island for Lilin
//! and sends the Aran on to Lisa in Orbis.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START,
};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const INFO_SHOP: i32 = 104000004;
const TRU: i32 = 1002104;
const CAVE: i32 = 105040201;
const PASSAGE: i32 = 910510100;
const ARENA: i32 = 910510202;
const END_STATUE: i32 = 1063017;
const EXIT_STATUE: i32 = 1063016;
const PUPPETEER: i32 = 9300346;
const RIEN: i32 = 140000000;
const LILIN: i32 = 1201000;
const ORBIS: i32 = 200000000;
const LISA: i32 = 2012012;
const TRACES: i16 = 21734;
const SEAL_STONE: i16 = 21735;
const TO_ORBIS: i16 = 21736;
const COMBO_DRAIN: i32 = 21100005;
const VICTORIA_SEAL_STONE: i32 = 4032323;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// Walk to NPC `npc` on `map` and talk to it.
async fn touch(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32) -> Trace {
    send(c, to(map, npc)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, talk(map, npc)).await
}

/// Walk to NPC `npc` on `map` and send a quest action to it.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    send(c, to(map, npc)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

/// Going through a scripted portal.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn an_aran_beats_the_puppeteer_in_its_arena_and_heads_for_orbis() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2110, 45, 50).await.unwrap();
    replace_item(id, -1, -11, 1442000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    set_quest(id, 21733, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Tru sends the Aran after the Puppeteer.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(SCRIPT_START, TRACES, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&TRACES), Some(&1));

    // The cave's door leads into the passage, and its end statue into the
    // arena, where the Puppeteer waits.
    assert_eq!(warp(c, CAVE, "", &mut trace).await, CAVE);
    step(c, &m, walk_to(277, 60)).await;
    let t = step(c, &m, enter("in00")).await;
    assert_eq!(t.warps, [PASSAGE]);
    let t = touch(c, &m, PASSAGE, END_STATUE).await;
    assert_eq!(t.prompts, [(END_STATUE, 1)]);
    send(c, answer(1, 1, None)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [ARENA] && t.field.alive(PUPPETEER).len() == 1
    })
    .await;
    let puppeteer = trace.field.alive(PUPPETEER)[0];
    for _ in 0..20 {
        send(c, attack(puppeteer, 30_000)).await;
        read_for(c, &mut trace, 100).await;
        if trace.field.killed.contains(&puppeteer) {
            break;
        }
    }
    assert_eq!(trace.kills[&TRACES], "001");

    // Beaten, it doesn't come back; the exit statue leads out.
    assert_eq!(warp(c, PASSAGE, "", &mut trace).await, PASSAGE);
    let mut trace = Trace::default();
    assert_eq!(warp(c, ARENA, "", &mut trace).await, ARENA);
    read_for(c, &mut trace, 500).await;
    assert!(trace.field.alive(PUPPETEER).is_empty());
    assert_eq!(warp(c, PASSAGE, "", &mut trace).await, PASSAGE);
    let t = touch(c, &m, PASSAGE, EXIT_STATUE).await;
    assert_eq!(t.prompts, [(EXIT_STATUE, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [CAVE]);

    // Tru teaches Combo Drain.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(SCRIPT_END, TRACES, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&TRACES), Some(&2));
    assert_eq!(t.gains, [12500]);
    assert_eq!(t.skills, [(COMBO_DRAIN, 0)]);

    // The seal stone, for Lilin, who keeps it.
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(SCRIPT_START, SEAL_STONE, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&SEAL_STONE), Some(&1));
    assert_eq!(t.items, [(VICTORIA_SEAL_STONE, 1)]);
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        LILIN,
        build_quest_action(SCRIPT_END, SEAL_STONE, LILIN),
    )
    .await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&SEAL_STONE), Some(&2));
    assert_eq!(t.items, [(VICTORIA_SEAL_STONE, -1)]);
    assert_eq!(t.gains, [6037]);

    // On to Orbis.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(SCRIPT_START, TO_ORBIS, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(TRU, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&TO_ORBIS), Some(&1));
    assert_eq!(warp(c, ORBIS, "", &mut trace).await, ORBIS);
    let t = quest(
        c,
        &m,
        ORBIS,
        LISA,
        build_quest_completion(TO_ORBIS, LISA, -1),
    )
    .await;
    assert_eq!(t.states.get(&TO_ORBIS), Some(&2));
    assert_eq!(t.gains, [500]);
}
