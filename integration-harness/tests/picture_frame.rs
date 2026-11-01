//! The picture frame in Magatia's Home of the Missing Alchemist hides the
//! Silver Pendant that Phyllia's Pendant (3322) ends on, and gives it up
//! while the quest is under way.

use game_data::server_content::keys;
use integration_harness::database::start_quest;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::PICTURE_FRAME;
use tokio::time::Duration;

const ALCHEMISTS_HOME: i32 = 261000001;
const PHYLLIAS_PENDANT: i16 = 3322;
const SILVER_PENDANT: i32 = 4031697;

#[tokio::test]
async fn the_picture_frame_gives_up_the_silver_pendant() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, PHYLLIAS_PENDANT).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, ALCHEMISTS_HOME, "", &mut seen).await,
        ALCHEMISTS_HOME
    );
    let trace = told(&mut session, talk(ALCHEMISTS_HOME, PICTURE_FRAME)).await;
    assert!(said(&trace, keys::PICTURE_FRAME_FOUND), "{trace:?}");
    let mut seen = told(&mut session, answer(0, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(SILVER_PENDANT, 1))).await;
}
