//! The Demon's Doorways in Victoria Island let one on their job's demon
//! quest, holding the demon's emblem, into its strolling path; Astaroth's
//! Doorway leads from the demon's strolling place to its hiding place and
//! from there back out to the swamp.

use game_data::server_content::keys;
use integration_harness::database::{grant_item, start_quest};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{
    ASTAROTHS_DOORWAY, ASTAROTH_HIDING_PLACE, ASTAROTH_STROLLING_PLACE, ASTAROTH_SWAMP,
    DEMONS_DOORWAYS,
};
use tokio::time::Duration;

const ETC: i16 = 4;
const PERION_STREET: i32 = 102020300;

#[tokio::test]
async fn the_doorway_lets_one_on_dances_with_andras_with_the_emblem_in() {
    let andras = &DEMONS_DOORWAYS[0];
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, andras.quest).await.unwrap();

    // On the quest without the emblem, the doorway stays shut.
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, PERION_STREET, "", &mut seen).await, PERION_STREET);
    let trace = told(&mut session, talk(PERION_STREET, andras.npc)).await;
    assert!(said(&trace, keys::DEMONS_DOORWAY_EMBLEM), "{trace:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, andras.emblem).await.unwrap();

    // With it, a yes goes through.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, PERION_STREET, "", &mut seen).await, PERION_STREET);
    let trace = told(&mut session, talk(PERION_STREET, andras.npc)).await;
    let path = andras.path.to_string();
    let ask = net::content::text().get(keys::DEMONS_DOORWAY_ASK, &[("map", &path)]);
    assert_eq!(trace.prompt_texts, [ask], "{trace:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&andras.path)).await;
}

#[tokio::test]
async fn astaroths_doorway_leads_on_to_the_hiding_place_and_back_out() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    let at = warp(c, ASTAROTH_STROLLING_PLACE, "", &mut seen).await;
    assert_eq!(at, ASTAROTH_STROLLING_PLACE);
    told(
        &mut session,
        talk(ASTAROTH_STROLLING_PLACE, ASTAROTHS_DOORWAY),
    )
    .await;
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&ASTAROTH_HIDING_PLACE)).await;

    let trace = told(&mut session, talk(ASTAROTH_HIDING_PLACE, ASTAROTHS_DOORWAY)).await;
    assert!(said(&trace, keys::ASTAROTHS_DOORWAY_LEAVE), "{trace:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&ASTAROTH_SWAMP)).await;
}
