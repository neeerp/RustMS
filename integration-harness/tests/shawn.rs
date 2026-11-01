//! At the Excavation Site's camp, Shawn tells of Sharenian, the Rubian and
//! the Guild Quest, and the Bulletin Board gives the quest's rules.

use game_data::server_content::keys;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::guild_quest::{BULLETIN_BOARD, CAMP, SHAWN};
use tokio::time::Duration;

#[tokio::test]
async fn shawn_tells_of_the_rubian_and_the_board_gives_the_rules() {
    let config = load_harness_config_or_fail().await;
    drop(login_to_world_session(&config).await.unwrap());
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, CAMP, "", &mut seen).await, CAMP);

    let menu = told(&mut session, talk(CAMP, SHAWN)).await;
    assert_eq!(menu.prompts, [(SHAWN, 4)], "{menu:?}");
    let trace = told(&mut session, answer(4, 1, Some(1))).await;
    assert!(said(&trace, keys::SHAWN_RUBIAN), "{trace:?}");

    // The harness world runs the quest with fewer members, so only the
    // notice's opening is checked.
    let trace = told(&mut session, talk(CAMP, BULLETIN_BOARD)).await;
    assert_eq!(trace.prompts, [(BULLETIN_BOARD, 0)], "{trace:?}");
    let rules = net::content::text().get(keys::GUILD_QUEST_BOARD, &[("least", "\u{1}")]);
    let opening = rules.split('\u{1}').next().unwrap();
    assert!(trace.prompt_texts[0].starts_with(opening), "{trace:?}");
}
