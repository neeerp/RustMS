//! Reactors: Amherst's scrap boxes break after four hits, drop their loot,
//! and drop Pio's materials only while his quest (1008) is started.

use integration_harness::connection::MapleTestConnection;
use integration_harness::hunting::{hit_reactor, pickup, walk_to, MapReactor};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, COMPLETE, START};
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};

const AMHERST: i32 = 1_010_000;
const SCRAP_BOX: i32 = 2000;
const PIO: i32 = 10000;
const PIOS_QUEST: i16 = 1008;
const MATERIALS: [i32; 2] = [4031161, 4031162];

/// Stand by a box and hit it until it breaks; every hit before the last is
/// shown to the field.
async fn break_box(c: &mut MapleTestConnection, trace: &mut Trace, target: MapReactor) {
    send(c, walk_to(target.x, target.y)).await;
    for hit in 1..=4u8 {
        send(c, hit_reactor(target.object_id, target.x, target.y)).await;
        if hit < 4 {
            until(c, trace, |t| {
                t.field.reactor_hits.contains(&(target.object_id, hit))
            })
            .await;
        }
    }
    until(c, trace, |t| {
        t.field.reactors_removed.contains(&(target.object_id, 3))
    })
    .await;
    // Give the loot time to fall.
    read_for(c, trace, 500).await;
}

#[tokio::test]
async fn amherst_boxes_break_and_drop_pios_materials_during_his_quest() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, AMHERST, "", &mut trace).await, AMHERST);
    until(c, &mut trace, |t| t.field.reactors.len() >= 5).await;
    assert!(trace
        .field
        .reactors
        .iter()
        .all(|r| r.reactor_id == SCRAP_BOX && r.state == 0));
    // The two boxes on the ground floor.
    let mut ground: Vec<MapReactor> = trace
        .field
        .reactors
        .iter()
        .filter(|r| r.y > 200)
        .copied()
        .collect();
    ground.sort_by_key(|r| r.x);
    let [first, second] = ground[..] else {
        panic!("two ground-floor boxes: {:?}", trace.field.reactors);
    };

    // A box can't be hit from across the map.
    send(c, walk_to(first.x + 1000, first.y)).await;
    send(c, hit_reactor(first.object_id, first.x, first.y)).await;
    read_for(c, &mut trace, 400).await;
    assert!(trace.field.reactor_hits.is_empty());

    // Without the quest, a box drops no materials.
    break_box(c, &mut trace, first).await;
    for item in MATERIALS {
        assert!(trace.field.item_drop(item).is_none(), "{item} dropped");
    }
    // A broken box takes no more hits.
    let hits = trace.field.reactor_hits.len();
    send(c, hit_reactor(first.object_id, first.x, first.y)).await;
    read_for(c, &mut trace, 400).await;
    assert_eq!(trace.field.reactor_hits.len(), hits);

    send(c, walk_to(547, 274)).await;
    act(c, &mut trace, &m, START, PIOS_QUEST, PIO).await;
    break_box(c, &mut trace, second).await;
    for item in MATERIALS {
        let loot = trace
            .field
            .item_drop(item)
            .unwrap_or_else(|| panic!("{item} did not drop: {:?}", trace.field.drops));
        send(c, pickup(loot.object_id, loot.x, loot.y)).await;
        until(c, &mut trace, |t| t.added.iter().any(|a| a.2 == item)).await;
    }

    // Pio takes the materials and pays 200 mesos.
    send(c, walk_to(547, 274)).await;
    act(c, &mut trace, &m, COMPLETE, PIOS_QUEST, PIO).await;
    assert_eq!(trace.states.get(&PIOS_QUEST), Some(&2));
    assert_eq!(trace.stat(stat::MESO), Some(200));
}
