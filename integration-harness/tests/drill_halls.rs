//! Ereve's 3rd Drill Hall takes a Cygnus Knight training for a new skill
//! (20601) in to its boss, one at a time: the next is turned away while
//! the first is in.

use game_data::server_content::keys;
use integration_harness::database::start_quest;
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::scripts::THIRD_DRILL_HALL;
use tokio::time::Duration;

const DRILL_HALL_ENTRANCE: i32 = 130020000;
const NEW_SKILL: i16 = 20601;
const HALL_BOSS: i32 = 9300289;

/// A new character on A New Skill, at the Drill Halls' Entrance.
async fn in_training() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, NEW_SKILL).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, DRILL_HALL_ENTRANCE, "", &mut seen).await,
        DRILL_HALL_ENTRANCE
    );
    session
}

#[tokio::test]
async fn the_third_drill_hall_takes_one_knight_at_a_time_to_its_boss() {
    let mut first = in_training().await;
    let mut late = in_training().await;

    touch_script_portal(&mut first, DRILL_HALL_ENTRANCE, "in02").await;
    let mut seen = Trace::default();
    until(&mut first.connection, &mut seen, |t| {
        t.warps.contains(&THIRD_DRILL_HALL.map) && !t.field.alive(HALL_BOSS).is_empty()
    })
    .await;

    touch_script_portal(&mut late, DRILL_HALL_ENTRANCE, "in02").await;
    let occupied = net::content::text().get(keys::SOLO_MAP_OCCUPIED, &[]);
    let mut seen = Trace::default();
    until(&mut late.connection, &mut seen, |t| {
        t.broadcasts.iter().any(|(_, line)| *line == occupied)
    })
    .await;
    read_for(&mut late.connection, &mut seen, 300).await;
    assert!(!seen.warps.contains(&THIRD_DRILL_HALL.map), "{seen:?}");
}
