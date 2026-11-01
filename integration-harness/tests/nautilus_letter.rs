//! Two stops on the Nautilus's port: the trash can holds the Crumpled Paper
//! that starts The Half-written Letter (2162), and Muirhat hands one on
//! Disciples of the Black Magician (2175) a Nautilus Return Scroll and sends
//! them to the Disciples' hideout.

use game_data::server_content::keys;
use integration_harness::database::start_quest;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{MUIRHAT, NAUTILUS_TRASH_CAN};
use net::scripts::{DISCIPLES_HIDEOUT, DISCIPLES_QUEST};
use tokio::time::Duration;

const NAUTILUS_PORT: i32 = 120000100;
const CRUMPLED_PAPER: i32 = 4031839;
const NAUTILUS_RETURN_SCROLL: i32 = 2030019;

#[tokio::test]
async fn the_trash_can_gives_the_letter_and_muirhat_sends_one_after_the_disciples() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, DISCIPLES_QUEST).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, NAUTILUS_PORT, "", &mut seen).await, NAUTILUS_PORT);

    // The letter comes out of the trash can once, and only once.
    let trace = told(&mut session, talk(NAUTILUS_PORT, NAUTILUS_TRASH_CAN)).await;
    assert!(said(&trace, keys::NAUTILUS_TRASH_CAN_FOUND), "{trace:?}");
    let mut seen = told(&mut session, answer(0, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(CRUMPLED_PAPER, 1))).await;
    let trace = told(&mut session, talk(NAUTILUS_PORT, NAUTILUS_TRASH_CAN)).await;
    assert!(said(&trace, keys::KERNING_TRASH_CAN_EMPTY), "{trace:?}");
    told(&mut session, answer(0, 1, None)).await;

    // Muirhat sends the pirate on 2175 to the hideout with a scroll back.
    let trace = told(&mut session, talk(NAUTILUS_PORT, MUIRHAT)).await;
    assert!(said(&trace, keys::MUIRHAT_SCROLL), "{trace:?}");
    let mut seen = told(&mut session, answer(0, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&DISCIPLES_HIDEOUT)).await;
    assert!(
        seen.items.contains(&(NAUTILUS_RETURN_SCROLL, 1)),
        "{seen:?}"
    );
}
