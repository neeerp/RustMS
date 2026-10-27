//! Collecting Meteorite Samples (3421): each of the six meteorites in Kulan
//! Field gives one sample, marking its character of the quest's info
//! string, and six complete the quest.

use integration_harness::database::{set_job, set_quest};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const OMEGA_SECTOR: i32 = 221030000;
const YELLOW_MESORANGER: i32 = 2041011;
const METEORITE_SAMPLES: i16 = 3421;
const METEORITE_SAMPLE: i32 = 4031117;
const MANA_ELIXIR: i32 = 2022003;
/// The meteorites, each on its own map, in the info string's order.
const METEORITES: [(i32, i32); 6] = [
    (221040000, 2050014),
    (221040200, 2050015),
    (221040300, 2050016),
    (221040100, 2050017),
    (221040201, 2050018),
    (221040400, 2050019),
];

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn each_meteorite_gives_one_sample_and_six_complete_the_quest() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 44, 50).await.unwrap();
    set_quest(id, 3420, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, OMEGA_SECTOR, "", &mut trace).await, OMEGA_SECTOR);
    send(c, to(OMEGA_SECTOR, YELLOW_MESORANGER)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, METEORITE_SAMPLES, YELLOW_MESORANGER),
    )
    .await;
    assert_eq!(t.states.get(&METEORITE_SAMPLES), Some(&1));

    let mut info = *b"000000";
    for (n, (map, meteorite)) in METEORITES.into_iter().enumerate() {
        assert_eq!(warp(c, map, "", &mut trace).await, map);
        send(c, to(map, meteorite)).await;
        barrier(c, &m, &mut trace).await;
        let t = step(c, &m, talk(map, meteorite)).await;
        assert!(t.prompts.is_empty(), "{t:?}");
        assert_eq!(t.items, [(METEORITE_SAMPLE, 1)]);
        info[n] = b'1';
        assert_eq!(
            t.kills[&METEORITE_SAMPLES].as_bytes(),
            info,
            "{map} {meteorite}"
        );
        // Not twice.
        let t = step(c, &m, talk(map, meteorite)).await;
        assert!(t.items.is_empty() && t.kills.is_empty(), "{t:?}");
    }

    assert_eq!(warp(c, OMEGA_SECTOR, "", &mut trace).await, OMEGA_SECTOR);
    send(c, to(OMEGA_SECTOR, YELLOW_MESORANGER)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(METEORITE_SAMPLES, YELLOW_MESORANGER, -1),
    )
    .await;
    assert_eq!(t.states.get(&METEORITE_SAMPLES), Some(&2));
    assert_eq!(t.gains, [16000]);
    let mut items = t.items.clone();
    items.sort();
    assert_eq!(items, [(MANA_ELIXIR, 50), (METEORITE_SAMPLE, -6)]);
}
