//! A grassy lump along Henesys's Pet-Walking Road hides Pet poop, which a
//! yes pulls out.

use game_data::server_content::keys;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{GRASSY_LUMP, PET_POOP};

const PET_WALKING_ROAD: i32 = 100000202;

#[tokio::test]
async fn a_grassy_lump_gives_up_pet_poop() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    let at = warp(c, PET_WALKING_ROAD, "", &mut seen).await;
    assert_eq!(at, PET_WALKING_ROAD);
    let asked = told(&mut session, talk(PET_WALKING_ROAD, GRASSY_LUMP)).await;
    assert!(said(&asked, keys::GRASSY_LUMP_ASK), "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(PET_POOP, 1))).await;
    assert!(said(&seen, keys::GRASSY_LUMP_POOP), "{seen:?}");
}
