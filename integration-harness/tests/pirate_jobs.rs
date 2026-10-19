//! Kyrin's job advancement: a beginner becomes a Pirate, and a level-30
//! Pirate takes How to become a Brawler (2191), passes its test, and
//! becomes a Brawler.

use game_data::GameData;
use integration_harness::database::{grant_stack, set_dex, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, answer, COMPLETE, START};
use integration_harness::trace::{barrier, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const KYRIN: i32 = 1090000;
const KYRIN_MAP: i32 = 120000101;
const PROCTOR: i32 = 1072008;
const BRAWLER_TEST: i32 = 108000502;
const BRAWLER_QUEST: i16 = 2191;
const CRYSTAL: i32 = 4031856;
const KNUCKLE: i32 = 1482014;
const GUN: i32 = 1492014;
const BULLETS: i32 = 2330006;
const ETC: u8 = 4;
const JOB: i32 = 0x20;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

#[tokio::test]
async fn kyrin_makes_a_pirate_then_a_brawler() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.expect("level 10");
    set_dex(id, 20).await.expect("DEX 20");

    // The first job: a knuckle, a gun, bullets, and a row more in the Equip
    // and Etc bags.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, KYRIN_MAP, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(KYRIN_MAP, KYRIN)).await;
    assert_eq!(t.prompts, [(KYRIN, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(KYRIN, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [500]);
    assert_eq!(
        t.bag_slots,
        [
            (1, session.slot_limits[0] + 4),
            (ETC, session.slot_limits[3] + 4)
        ]
    );
    assert_eq!(t.prompts, [(KYRIN, 0)]);
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let after = relogin(&config).await;
    assert_eq!(after.job, 500);
    assert_eq!(after.items.get(&KNUCKLE), Some(&1));
    assert_eq!(after.items.get(&GUN), Some(&1));
    assert_eq!(after.items.get(&BULLETS), Some(&1200));
    drop(after);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 500, 30, 50).await.expect("level 30 Pirate");

    // Without a quest, a word about them; with Brawler's started, the way
    // into its test.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, KYRIN_MAP, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(KYRIN_MAP, KYRIN)).await;
    assert_eq!(t.prompts, [(KYRIN, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty());
    let mut t = Trace::default();
    act(c, &mut t, &m, START, BRAWLER_QUEST, KYRIN).await;
    assert_eq!(t.states.get(&BRAWLER_QUEST), Some(&1));
    let t = step(c, &m, talk(KYRIN_MAP, KYRIN)).await;
    assert_eq!(t.prompts, [(KYRIN, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [BRAWLER_TEST]);

    // Without the crystals, the proctor only offers the way out.
    let mut t = Trace::default();
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(BRAWLER_TEST, PROCTOR)).await;
    assert_eq!(t.prompts, [(PROCTOR, 1)]);
    let t = step(c, &m, answer(1, 0, None)).await;
    assert!(t.prompts.is_empty() && t.arrivals.is_empty());
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, i16::from(ETC), 20, CRYSTAL, 15)
        .await
        .expect("crystals");

    // With them, back to Kyrin, crystals and all; the quest takes them.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, BRAWLER_TEST, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(BRAWLER_TEST, PROCTOR)).await;
    assert_eq!(t.prompts, [(PROCTOR, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.arrivals, [(KYRIN_MAP, 0, None)]);
    assert!(t.items.is_empty());
    let mut t = Trace::default();
    barrier(c, &m, &mut t).await;
    act(c, &mut t, &m, COMPLETE, BRAWLER_QUEST, KYRIN).await;
    assert_eq!(t.states.get(&BRAWLER_QUEST), Some(&2));
    assert_eq!(t.items, [(CRYSTAL, -15)]);
    let (sp, max_hp, etc_slots) = (session.sp, session.max_hp, session.slot_limits[3]);

    // Kyrin makes a Brawler: 1 SP, 325 HP and a row more in the Etc bag.
    let t = step(c, &m, talk(KYRIN_MAP, KYRIN)).await;
    assert_eq!(t.prompts, [(KYRIN, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(KYRIN, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [510]);
    assert_eq!(t.stat_values(stat::SP), [i64::from(sp) + 1]);
    assert_eq!(t.stat_values(stat::MAX_HP), [i64::from(max_hp) + 325]);
    assert_eq!(t.bag_slots, [(ETC, etc_slots + 4)]);
    assert_eq!(t.prompts, [(KYRIN, 0)]);
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 510);
    assert_eq!(after.slot_limits[3], etc_slots + 4);
    assert_eq!(after.items.get(&CRYSTAL), None);
}
