//! First job advancement: Dances with Balrog makes a beginner a Warrior.

use game_data::GameData;
use integration_harness::database::{set_job, set_str};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const DANCES_WITH_BALROG: i32 = 1022000;
const WARRIORS_SANCTUARY: i32 = 102000003;
const SWORD: i32 = 1302077;
const JOB: i32 = 0x20;
const MAX_MP: i32 = 0x2000;

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
async fn dances_with_balrog_makes_a_strong_beginner_a_warrior() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 12, 50).await.expect("level 12");
    set_str(id, 4).await.expect("weak");

    // Too weak: the introduction, then a refusal.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(
        warp(c, WARRIORS_SANCTUARY, "", &mut t).await,
        WARRIORS_SANCTUARY
    );
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.stat_updates.is_empty());
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_str(id, 35).await.expect("strong");

    // Strong enough: yes makes a Warrior, with a row more in each bag.
    let mut before = relogin(&config).await;
    let c = &mut before.connection;
    let mut t = Trace::default();
    warp(c, WARRIORS_SANCTUARY, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    step(c, &m, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [100]);
    // (12 - 10) * 3 + 1 SP, and a Warrior's 225 more HP.
    assert_eq!(t.stat_values(stat::SP), [i64::from(before.sp) + 7]);
    assert_eq!(
        t.stat_values(stat::MAX_HP),
        [i64::from(before.max_hp) + 225]
    );
    assert_eq!(t.stat_values(MAX_MP), [50]);
    let grown: Vec<(u8, u8)> = (1..=4)
        .map(|kind| (kind, before.slot_limits[usize::from(kind) - 1] + 4))
        .collect();
    assert_eq!(t.bag_slots, grown);
    assert_eq!(
        t.added.iter().map(|a| (a.0, a.2)).collect::<Vec<_>>(),
        [(1, SWORD)]
    );
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty());

    // Now a Warrior: one page, and nothing more to give.
    let t = step(c, &m, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.stat_updates.is_empty());
    let (sp, max_hp, slot_limits) = (before.sp, before.max_hp, before.slot_limits);
    drop(before);

    let after = relogin(&config).await;
    assert_eq!(after.job, 100);
    assert_eq!(after.sp, sp + 7);
    assert_eq!(after.max_hp, max_hp + 225);
    assert_eq!(
        &after.slot_limits[..4],
        &grown.iter().map(|g| g.1).collect::<Vec<_>>()[..]
    );
    assert_eq!(after.slot_limits[4], slot_limits[4]);
    assert_eq!(after.items.get(&SWORD), Some(&1));
}
