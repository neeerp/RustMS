//! Neinheart at the end of the Cygnus Knights' opening takes a player who
//! says yes to Ereve, and one who says no back to Henesys.

use integration_harness::events::{talk, told, waiting_as};
use integration_harness::quests::answer;
use integration_harness::trace::until;
use net::npc_scripts::{OPENING_EREVE, OPENING_HENESYS, OPENING_NEINHEART};

const OPENING_END: i32 = 913040006;

#[tokio::test]
async fn neinheart_sends_one_to_ereve_or_back_to_henesys() {
    for (response, to) in [(1, OPENING_EREVE), (0, OPENING_HENESYS)] {
        let mut player = waiting_as(OPENING_END, 10, 0).await;
        let asked = told(&mut player, talk(OPENING_END, OPENING_NEINHEART)).await;
        assert_eq!(asked.prompts, [(OPENING_NEINHEART, 1)], "{asked:?}");
        let mut seen = told(&mut player, answer(1, response, None)).await;
        until(&mut player.connection, &mut seen, |t| t.warps.contains(&to)).await;
    }
}
