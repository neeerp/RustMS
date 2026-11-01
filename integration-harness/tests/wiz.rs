//! Wiz the Librarian in the Helios Tower Library lists the storybooks the
//! player has brought back, by the storybook quests completed.

use game_data::server_content::keys;
use integration_harness::database::set_quest;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::WIZ;
use tokio::time::Duration;

const LIBRARY: i32 = 222020000;
const HONGBU_AND_NOLBU: i16 = 3615;
const HONGBU_AND_NOLBU_BOOK: i32 = 4031235;

#[tokio::test]
async fn wiz_lists_the_storybooks_brought_back() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, HONGBU_AND_NOLBU, 2, "").await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, LIBRARY, "", &mut seen).await, LIBRARY);
    let trace = told(&mut session, talk(LIBRARY, WIZ)).await;
    let book = format!("#v{HONGBU_AND_NOLBU_BOOK}#");
    assert!(
        trace.prompt_texts.iter().any(|said| said.contains(&book)),
        "{trace:?}"
    );
    let trace = told(&mut session, answer(0, 1, None)).await;
    assert!(said(&trace, keys::WIZ_THANKS), "{trace:?}");
}
