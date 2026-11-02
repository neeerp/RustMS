//! The Aran's trail from Orbis to Mu Lung (21738-21742): Spiruna sends the
//! Aran into the Sealed Garden, which the Neglected Strolling Path's portal
//! lets one challenger into and whose guardian waits in its inner garden.
//! Lilin teaches Combo Smash for the news of the stolen Orbis seal stone,
//! and Mr. Do hands over a soaked Hanging Scroll, whose writing comes back
//! as it is fed Special Ink in its raise window.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_quest, set_str, set_weapon_attack,
};
use integration_harness::events::maps;
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const OLD_MANS_HOUSE: i32 = 200050001;
const SPIRUNA: i32 = 2032001;
const ORBIS_PARK: i32 = 200060000;
const GARDEN_WAY: i32 = 920030000;
const GARDEN: i32 = 920030001;
const GUARDIAN: i32 = 9300348;
const INFO_SHOP: i32 = 104000004;
const TRU: i32 = 1002104;
const RIEN: i32 = 140000000;
const LILIN: i32 = 1201000;
const MU_LUNG: i32 = 250000000;
const MR_DO: i32 = 2090004;
const SEALED_GARDEN: i16 = 21738;
const TO_THE_SEALED_GARDEN: i16 = 21739;
const ORBIS_STONE_STOLEN: i16 = 21740;
const INVESTIGATING_MU_LUNG: i16 = 21741;
const THE_BLACK_SCROLL: i16 = 21742;
const WRITING: i16 = 21763;
const COMBO_SMASH: i32 = 21100004;
const HANGING_SCROLL: i32 = 4220151;
const SPECIAL_INK: i32 = 4032342;
/// The Etc slot the ink is granted in.
const INK_SLOT: i16 = 5;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
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

/// Special Ink dragged onto the scroll's window.
fn feed() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RaiseFeed as i16).unwrap();
    p.write_byte(4).unwrap();
    p.write_short(INK_SLOT).unwrap();
    p.write_int(HANGING_SCROLL).unwrap();
    p
}

#[tokio::test]
async fn an_aran_clears_the_sealed_garden_and_restores_mr_dos_scroll() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2110, 54, 50).await.unwrap();
    replace_item(id, -1, -11, 1442000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    set_quest(id, 21737, 2, "").await.unwrap();
    grant_stack(id, 4, INK_SLOT, SPECIAL_INK, 10).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Spiruna tells of the garden, then sends the Aran into it.
    assert_eq!(
        warp(c, OLD_MANS_HOUSE, "", &mut trace).await,
        OLD_MANS_HOUSE
    );
    let t = quest(
        c,
        &m,
        OLD_MANS_HOUSE,
        SPIRUNA,
        build_quest_action(SCRIPT_START, SEALED_GARDEN, SPIRUNA),
    )
    .await;
    assert_eq!(t.prompts, [(SPIRUNA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(SPIRUNA, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&SEALED_GARDEN), Some(&1));
    let t = step(c, &m, build_quest_completion(SEALED_GARDEN, SPIRUNA, -1)).await;
    assert_eq!(t.states.get(&SEALED_GARDEN), Some(&2));
    assert_eq!(t.gains, [1000]);
    let t = step(
        c,
        &m,
        build_quest_action(START, TO_THE_SEALED_GARDEN, SPIRUNA),
    )
    .await;
    assert_eq!(t.states.get(&TO_THE_SEALED_GARDEN), Some(&1));

    // The path's portal leads into the garden's way, and its inner garden
    // holds the guardian.
    assert_eq!(warp(c, ORBIS_PARK, "", &mut trace).await, ORBIS_PARK);
    step(c, &m, walk_to(1231, -323)).await;
    let t = step(c, &m, enter("in00")).await;
    assert_eq!(t.warps, [GARDEN_WAY]);
    step(c, &m, walk_to(587, -1032)).await;
    let mut trace = Trace::default();
    assert_eq!(warp(c, -1, "in00", &mut trace).await, GARDEN);
    until(c, &mut trace, |t| t.field.alive(GUARDIAN).len() == 1).await;
    let guardian = trace.field.alive(GUARDIAN)[0];
    for _ in 0..20 {
        send(c, attack(guardian, 30_000)).await;
        read_for(c, &mut trace, 100).await;
        if trace.field.killed.contains(&guardian) {
            break;
        }
    }
    assert_eq!(trace.kills[&TO_THE_SEALED_GARDEN], "001");

    assert_eq!(
        warp(c, OLD_MANS_HOUSE, "", &mut trace).await,
        OLD_MANS_HOUSE
    );
    let t = quest(
        c,
        &m,
        OLD_MANS_HOUSE,
        SPIRUNA,
        build_quest_action(SCRIPT_END, TO_THE_SEALED_GARDEN, SPIRUNA),
    )
    .await;
    assert_eq!(t.prompts, [(SPIRUNA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&TO_THE_SEALED_GARDEN), Some(&2));
    assert_eq!(t.gains, [29500]);
    // Done, the portal leads to the path again.
    assert_eq!(warp(c, ORBIS_PARK, "", &mut trace).await, ORBIS_PARK);
    step(c, &m, walk_to(1231, -323)).await;
    let t = step(c, &m, enter("in00")).await;
    assert_eq!(t.warps, [200060001]);

    // The stolen seal stone: Lilin teaches Combo Smash.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(SCRIPT_START, ORBIS_STONE_STOLEN, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&ORBIS_STONE_STOLEN), Some(&1));
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        LILIN,
        build_quest_action(SCRIPT_END, ORBIS_STONE_STOLEN, LILIN),
    )
    .await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&ORBIS_STONE_STOLEN), Some(&2));
    assert_eq!(t.skills, [(COMBO_SMASH, 0)]);

    // On to Mr. Do in Mu Lung.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(SCRIPT_START, INVESTIGATING_MU_LUNG, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(TRU, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&INVESTIGATING_MU_LUNG), Some(&1));
    send(c, answer(0, 1, None)).await;
    assert_eq!(warp(c, MU_LUNG, "", &mut trace).await, MU_LUNG);
    let t = quest(
        c,
        &m,
        MU_LUNG,
        MR_DO,
        build_quest_completion(INVESTIGATING_MU_LUNG, MR_DO, -1),
    )
    .await;
    assert_eq!(t.states.get(&INVESTIGATING_MU_LUNG), Some(&2));
    assert_eq!(t.gains, [500]);

    // Mr. Do hands over the soaked scroll.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, THE_BLACK_SCROLL, MR_DO),
    )
    .await;
    assert_eq!(t.prompts, [(MR_DO, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(MR_DO, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&THE_BLACK_SCROLL), Some(&1));
    assert_eq!(t.items, [(HANGING_SCROLL, 1)]);
    step(c, &m, answer(0, 1, None)).await;
    send(c, answer(0, 1, None)).await;

    // Not restored yet: Mr. Do can't end it.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, THE_BLACK_SCROLL, MR_DO),
    )
    .await;
    assert!(t.prompts.is_empty() && t.states.is_empty(), "{t:?}");

    // Each ink brings back 100 of the writing, up to 800.
    for n in 1..=8i16 {
        let t = step(c, &m, feed()).await;
        assert_eq!(t.kills[&WRITING], (n * 100).to_string());
        assert_eq!(t.quantities, [(4, INK_SLOT, 10 - n)]);
    }
    let t = step(c, &m, feed()).await;
    assert!(t.quantities.is_empty() && t.kills.is_empty(), "{t:?}");

    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, THE_BLACK_SCROLL, MR_DO),
    )
    .await;
    assert_eq!(t.prompts, [(MR_DO, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&THE_BLACK_SCROLL), Some(&2));
    assert_eq!(t.items, [(HANGING_SCROLL, -1)]);
    assert_eq!(t.gains, [10000]);
}
