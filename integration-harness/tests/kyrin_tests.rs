//! Kyrin's 4th-job tests (6330, 6370): Kyrin sends a Pirate on one into its
//! room; its way out holds them for a minute, and after two leads past the
//! room, where Kyrin marks the test passed and sends them back to town to
//! complete the quest. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the two minutes are two seconds.

use game_data::server_content::keys;
use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{maps, said, talk, told};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_completion};
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use net::scripts::{KYRIN_PASSED, KYRIN_PASSED_ROOM, KYRIN_TESTS};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const KYRIN: i32 = 1090000;
const KYRIN_MAP: i32 = 120000101;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// A new level-120 Outlaw on Kyrin's Battleship test, logged in by her.
async fn outlaw(quest: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 522, 120, 50).await.unwrap();
    start_quest(id, quest).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, KYRIN_MAP, "", &mut seen).await,
        KYRIN_MAP
    );
    session
}

/// The name of the portal in `map` running one of Kyrin's rooms' scripts.
fn way_out(map: i32) -> String {
    let field = maps().field(map).unwrap();
    let portal = field.portals().find(|p| p.script.is_some()).unwrap();
    portal.name.clone()
}

#[tokio::test]
async fn kyrins_test_holds_a_pirate_two_minutes_and_marks_it_passed() {
    let test = &KYRIN_TESTS[1];
    let mut a = outlaw(test.quest).await;

    // Kyrin offers the test and sends the Outlaw into its room.
    let trace = told(&mut a, talk(KYRIN_MAP, KYRIN)).await;
    assert!(said(&trace, keys::KYRIN_TEST_ASK), "{trace:?}");
    let mut seen = told(&mut a, answer(1, 1, None)).await;
    until(&mut a.connection, &mut seen, |t| {
        t.warps.contains(&test.room)
    })
    .await;

    // Too soon, the way out holds.
    let wait = net::content::text().get(keys::KYRIN_TEST_WAIT, &[]);
    let trace = told(&mut a, enter(&way_out(test.room))).await;
    assert!(trace.broadcasts.contains(&(5, wait)), "{trace:?}");
    assert!(trace.warps.is_empty(), "{trace:?}");

    // Two (scaled) minutes on, it leads past the room.
    tokio::time::sleep(Duration::from_millis(2_300)).await;
    let trace = told(&mut a, enter(&way_out(test.room))).await;
    assert_eq!(trace.warps, [KYRIN_PASSED_ROOM], "{trace:?}");

    // There Kyrin marks it passed and sends the Outlaw back to town.
    let trace = told(&mut a, talk(KYRIN_PASSED_ROOM, KYRIN)).await;
    assert!(said(&trace, keys::KYRIN_TEST_PASSED), "{trace:?}");
    let mut seen = told(&mut a, answer(0, 1, None)).await;
    until(&mut a.connection, &mut seen, |t| {
        t.kills.get(&test.record).map(String::as_str) == Some(KYRIN_PASSED)
            && t.warps.contains(&KYRIN_MAP)
    })
    .await;

    // She completes the quest on the record.
    let field = maps().field(KYRIN_MAP).unwrap();
    let kyrin = field.map_npcs.iter().find(|n| n.npc_id == KYRIN).unwrap();
    told(&mut a, walk_to(kyrin.x, kyrin.y)).await;
    let trace = told(&mut a, build_quest_completion(test.quest, KYRIN, -1)).await;
    assert_eq!(trace.states.get(&test.quest), Some(&2), "{trace:?}");
}
