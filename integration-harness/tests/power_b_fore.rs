//! Power B. Fore by Henesys sends a player under level 20 into one of the
//! five Bowman Training Centers, unless five are training there already,
//! and lets nobody of level 20 or more in.

use game_data::server_content::keys;
use integration_harness::events::{said, talk, told, waiting_as};
use integration_harness::quests::answer;
use integration_harness::trace::until;
use net::npc_scripts::{POWER_B_FORES, TRAINING_CENTER_ROOM};

const HUNTING_GROUND: i32 = 100010000;

#[tokio::test]
async fn power_b_fore_sends_a_novice_into_a_training_center_with_room() {
    let (npc, first) = POWER_B_FORES[0];

    // A novice picks the first Training Center and goes in.
    let mut novice = waiting_as(HUNTING_GROUND, 10, 0).await;
    let menu = told(&mut novice, talk(HUNTING_GROUND, npc)).await;
    assert_eq!(menu.prompts, [(npc, 4)], "{menu:?}");
    let mut seen = told(&mut novice, answer(4, 1, Some(0))).await;
    until(&mut novice.connection, &mut seen, |t| {
        t.warps.contains(&first)
    })
    .await;

    // With five in the second, another is turned away from it.
    let mut crowd = Vec::new();
    for _ in 0..TRAINING_CENTER_ROOM {
        crowd.push(waiting_as(first + 1, 10, 0).await);
    }
    let mut late = waiting_as(HUNTING_GROUND, 10, 0).await;
    told(&mut late, talk(HUNTING_GROUND, npc)).await;
    let trace = told(&mut late, answer(4, 1, Some(1))).await;
    assert!(said(&trace, keys::POWER_B_FORE_FULL), "{trace:?}");
    assert!(!trace.warps.contains(&(first + 1)), "{trace:?}");

    // A player of level 20 is not let in at all.
    let mut grown = waiting_as(HUNTING_GROUND, 20, 0).await;
    let trace = told(&mut grown, talk(HUNTING_GROUND, npc)).await;
    assert!(said(&trace, keys::POWER_B_FORE_TOO_HIGH), "{trace:?}");
    drop(crowd);
}
