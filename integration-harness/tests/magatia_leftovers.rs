//! Magatia's leftovers: Russellon's pill for "Life Alchemy, and the Missing
//! Alchemist" (3314), the quests a won Romeo and Juliet run opens (3365,
//! 3369, record 7072), and Yulete's marbles for Yulete's Reward (3382).

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job, set_quest, wear_equip};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::{use_item, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const ALCADNO_LAB: i32 = 261020000;
const RUSSELLON: i32 = 2111009;
const UPPER_LAB: i32 = 261020200;
const PILL_DESK: i32 = 2111015;
const ROMEOS_MAP: i32 = 261000011;
const ROMEO: i32 = 2112004;
const YULETES_LAB: i32 = 926130000;
const YULETE: i32 = 2112014;
const LIFE_ALCHEMY: i16 = 3314;
const YULETES_REQUEST: i16 = 3365;
const ROMEOS_PRESENT: i16 = 3369;
const YULETES_REWARD: i16 = 3382;
const MAGATIA_WON: i16 = 7072;
const ALCADNO_CAPE: i32 = 1102136;
const RUSSELLONS_PILLS: i32 = 2022198;
const ZENUMIST_MARBLE: i32 = 4001159;
const ALCADNO_MARBLE: i32 = 4001160;
const HORUS_EYE: i32 = 1122010;
const ROCK_OF_WISDOM: i32 = 2041212;
/// Mob skill 125 (Poison) at level 7, as the pill's buff names it.
const PILL_POISON: i32 = 125 | 7 << 16;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// Walk to NPC `npc` on `map` and send `action` to it.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    send(c, to(map, npc)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

#[tokio::test]
async fn russellons_pill_and_yuletes_marbles() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 200, 70, 500).await.unwrap();
    wear_equip(id, -9, ALCADNO_CAPE).await.unwrap();
    set_quest(id, 3313, 2, "").await.unwrap();
    for quest_id in [3366, 3367, 3368] {
        set_quest(id, quest_id, 2, "").await.unwrap();
    }
    set_quest(id, MAGATIA_WON, 1, "1").await.unwrap();
    grant_stack(id, 4, 20, ZENUMIST_MARBLE, 35).await.unwrap();
    grant_stack(id, 4, 21, ALCADNO_MARBLE, 35).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Wearing Alcadno's cape, Russellon has a favour to ask.
    assert_eq!(warp(c, ALCADNO_LAB, "", &mut trace).await, ALCADNO_LAB);
    let t = quest(
        c,
        &m,
        ALCADNO_LAB,
        RUSSELLON,
        build_quest_action(START, LIFE_ALCHEMY, RUSSELLON),
    )
    .await;
    assert_eq!(t.states.get(&LIFE_ALCHEMY), Some(&1));
    // Before the pill, Russellon sends the player to take one.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, LIFE_ALCHEMY, RUSSELLON),
    )
    .await;
    assert_eq!(t.prompts, [(RUSSELLON, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.states.is_empty(), "{t:?}");

    // The desk upstairs has a pill, one at a time.
    assert_eq!(warp(c, UPPER_LAB, "", &mut trace).await, UPPER_LAB);
    let t = quest(c, &m, UPPER_LAB, PILL_DESK, talk(UPPER_LAB, PILL_DESK)).await;
    assert_eq!(t.prompts, [(PILL_DESK, 0)]);
    assert_eq!(t.prompt_speakers, [2]);
    let &[(2, slot, RUSSELLONS_PILLS)] = &t.added[..] else {
        panic!("{t:?}");
    };
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, talk(UPPER_LAB, PILL_DESK)).await;
    assert!(t.prompts.is_empty(), "{t:?}");

    // The pill poisons its taker as mob skill 125 would.
    let t = step(c, &m, use_item(slot, RUSSELLONS_PILLS)).await;
    assert_eq!(t.removed, [(2, slot)]);
    assert_eq!(t.buffs, [(PILL_POISON, 50, 300_000)]);
    let t = step(c, &m, talk(UPPER_LAB, PILL_DESK)).await;
    assert!(t.prompts.is_empty(), "{t:?}");

    // Russellon sees it work, ends the quest with the data's rewards and
    // cures the player.
    assert_eq!(warp(c, ALCADNO_LAB, "", &mut trace).await, ALCADNO_LAB);
    let t = quest(
        c,
        &m,
        ALCADNO_LAB,
        RUSSELLON,
        build_quest_action(SCRIPT_END, LIFE_ALCHEMY, RUSSELLON),
    )
    .await;
    assert_eq!(t.prompts, [(RUSSELLON, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&LIFE_ALCHEMY), Some(&2));
    assert_eq!(t.gains, [12500]);
    // 20 of one of the data's potions.
    let &[(potion, 20)] = &t.items[..] else {
        panic!("{t:?}");
    };
    assert!((2022224..=2022228).contains(&potion) || potion == 2022199);
    assert_eq!(t.cancelled_buffs.len(), 1, "{t:?}");

    // A won Romeo and Juliet run (record 7072) opens Romeo's Present and
    // Yulete's Request.
    assert_eq!(warp(c, ROMEOS_MAP, "", &mut trace).await, ROMEOS_MAP);
    let t = quest(
        c,
        &m,
        ROMEOS_MAP,
        ROMEO,
        build_quest_action(START, ROMEOS_PRESENT, ROMEO),
    )
    .await;
    assert_eq!(t.states.get(&ROMEOS_PRESENT), Some(&1));
    assert_eq!(warp(c, YULETES_LAB, "", &mut trace).await, YULETES_LAB);
    let t = quest(
        c,
        &m,
        YULETES_LAB,
        YULETE,
        build_quest_action(START, YULETES_REQUEST, YULETE),
    )
    .await;
    assert_eq!(t.states.get(&YULETES_REQUEST), Some(&1));
    let t = step(c, &m, build_quest_completion(YULETES_REQUEST, YULETE, -1)).await;
    assert_eq!(t.states.get(&YULETES_REQUEST), Some(&2));

    // Yulete's Reward: 25 of each marble make Horus' Eye, and then, with
    // the eye owned, 10 of each make a Rock of Wisdom.
    for (made, took) in [(HORUS_EYE, 25), (ROCK_OF_WISDOM, 10)] {
        let t = step(c, &m, build_quest_action(START, YULETES_REWARD, YULETE)).await;
        assert_eq!(t.states.get(&YULETES_REWARD), Some(&1));
        let t = step(
            c,
            &m,
            build_quest_action(SCRIPT_END, YULETES_REWARD, YULETE),
        )
        .await;
        assert_eq!(t.prompts, [(YULETE, 0)]);
        let t = step(c, &m, answer(0, 1, None)).await;
        assert_eq!(
            t.items,
            [(ZENUMIST_MARBLE, -took), (ALCADNO_MARBLE, -took), (made, 1)]
        );
        assert_eq!(t.states.get(&YULETES_REWARD), Some(&2));
    }
    // With fewer than 10 left, Yulete asks for more.
    let t = step(c, &m, build_quest_action(START, YULETES_REWARD, YULETE)).await;
    assert_eq!(t.states.get(&YULETES_REWARD), Some(&1));
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, YULETES_REWARD, YULETE),
    )
    .await;
    assert_eq!(t.prompts, [(YULETE, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.items.is_empty() && t.states.is_empty(), "{t:?}");
}
