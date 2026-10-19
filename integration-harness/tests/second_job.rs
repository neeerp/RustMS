//! Second job advancement: Dances with Balrog sends a level-30 Warrior to
//! the examiner's test, and takes its proof for the Fighter's job.

use game_data::GameData;
use integration_harness::database::{grant_stack, set_job};
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
const EXAMINER: i32 = 1072000;
const EXAMINER_MAP: i32 = 102020300;
const PROCTOR: i32 = 1072004;
const TEST_MAP: i32 = 108000300;
const LETTER: i32 = 4031008;
const PROOF_OF_A_HERO: i32 = 4031012;
const DARK_MARBLE: i32 = 4031013;
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
async fn a_warrior_passes_the_test_and_becomes_a_fighter() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.expect("level 30 Warrior");

    // Balrog writes a letter to the examiner, who lets its holder in.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, WARRIORS_SANCTUARY, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(LETTER, 1)]);
    assert!(t.prompts.is_empty());

    let mut t = Trace::default();
    warp(c, EXAMINER_MAP, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    step(c, &m, talk(EXAMINER_MAP, EXAMINER)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(EXAMINER, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.arrivals, [(TEST_MAP, 0, None)]);
    assert!(
        t.items.is_empty(),
        "the letter stays until the test is passed"
    );

    // Without the marbles, the proctor only offers the way out.
    let mut t = Trace::default();
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(TEST_MAP, PROCTOR)).await;
    assert_eq!(t.prompts, [(PROCTOR, 1)]);
    let t = step(c, &m, answer(1, 0, None)).await;
    assert!(t.prompts.is_empty() && t.arrivals.is_empty());
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, i16::from(ETC), 20, DARK_MARBLE, 30)
        .await
        .expect("marbles");

    // With them, the proof, and back to the examiner.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, TEST_MAP, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(TEST_MAP, PROCTOR)).await;
    assert_eq!(t.prompts, [(PROCTOR, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(
        t.items,
        [(DARK_MARBLE, -30), (LETTER, -1), (PROOF_OF_A_HERO, 1)]
    );
    assert_eq!(t.arrivals, [(EXAMINER_MAP, 2, None)]);
    let (sp, max_hp, etc_slots) = (session.sp, session.max_hp, session.slot_limits[3]);

    // Balrog takes the proof: a Fighter, with 1 SP, 325 HP and a row more
    // in the Etc bag.
    let mut t = Trace::default();
    warp(c, WARRIORS_SANCTUARY, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [110]);
    assert_eq!(t.stat_values(stat::SP), [i64::from(sp) + 1]);
    assert_eq!(t.stat_values(stat::MAX_HP), [i64::from(max_hp) + 325]);
    assert_eq!(t.bag_slots, [(ETC, etc_slots + 4)]);
    assert_eq!(t.items, [(PROOF_OF_A_HERO, -1)]);
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 110);
    assert_eq!(after.slot_limits[3], etc_slots + 4);
    for item in [LETTER, PROOF_OF_A_HERO, DARK_MARBLE] {
        assert_eq!(after.items.get(&item), None);
    }
}
