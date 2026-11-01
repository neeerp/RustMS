//! Alcadno's Cabinet holds the Secret Document that An Incident, and the
//! Missing Alchemist (3309) ends on. One on the quest takes the dark pipe
//! off Magatia's Alcadno side into the cabinet's room, and the cabinet gives
//! the document up.

use integration_harness::database::start_quest;
use integration_harness::events::{talk, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{ALCADNO_CABINET, ALCADNO_CABINET_ROOM, AN_INCIDENT, SECRET_DOCUMENT};
use tokio::time::Duration;

const PIPE_MAP: i32 = 261020600;

#[tokio::test]
async fn the_dark_pipe_leads_to_alcadnos_cabinet_and_its_secret_document() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, AN_INCIDENT).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, PIPE_MAP, "", &mut seen).await, PIPE_MAP);
    touch_script_portal(&mut session, PIPE_MAP, "up00").await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&ALCADNO_CABINET_ROOM)).await;

    let cabinet = talk(ALCADNO_CABINET_ROOM, ALCADNO_CABINET);
    let mut seen = told(&mut session, cabinet).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(SECRET_DOCUMENT, 1))).await;
    assert!(seen.prompts.is_empty(), "{seen:?}");
}
