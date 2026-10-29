//! "A Mysterious Small Egg" (2230): the quest starts from the data, with a
//! Mysterious Egg in the player's pocket, and Mar the Fairy's end script
//! (`q2230e`) hatches it into a Rune Snail that lives five hours: a pet with
//! its own cash serial and name, saved with the character.

use integration_harness::database::{carried_pet, item_flags_and_hours, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, START};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

/// The forest path in Ellinia where Mar hides behind the bellflower.
const BELLFLOWER: i32 = 101_000_200;
const MAR: i32 = 1_032_102;
const EGG_QUEST: i16 = 2230;
const EGG: i32 = 4_032_086;
const RUNE_SNAIL: i32 = 5_000_054;
const CASH: u8 = 5;
/// NPC dialogue types: a page, a yes/no and a menu.
const SAY: u8 = 0;
const ASK: u8 = 1;
const MENU: u8 = 4;

#[tokio::test]
async fn mar_hatches_the_egg_into_a_rune_snail() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 15, 50).await.expect("level 15");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(
        warp(c, BELLFLOWER, "", &mut Trace::default()).await,
        BELLFLOWER
    );
    let field = maps().field(BELLFLOWER).unwrap();
    let mar = field.map_npcs.iter().find(|n| n.npc_id == MAR).unwrap();
    step(c, &m, walk_to(mar.x, mar.y)).await;

    // The start puts the egg in the player's pocket.
    let t = step(c, &m, build_quest_action(START, EGG_QUEST, MAR)).await;
    assert_eq!(t.progress, [(EGG_QUEST, 1)]);
    assert_eq!(t.items, [(EGG, 1)]);

    // Mar tells of pets, and the egg hatches once the player agrees.
    let t = step(c, &m, build_quest_action(SCRIPT_END, EGG_QUEST, MAR)).await;
    assert_eq!(t.prompts, [(MAR, MENU)]);
    let t = step(c, &m, answer(MENU, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MAR, SAY)]);
    for _ in 0..3 {
        step(c, &m, answer(SAY, 1, None)).await;
    }
    let t = step(c, &m, answer(SAY, 1, None)).await;
    assert_eq!(t.prompts, [(MAR, ASK)]);
    let t = step(c, &m, answer(ASK, 1, None)).await;
    assert_eq!(t.prompts, [(MAR, SAY)]);
    assert!(t.progress.is_empty(), "not before the last page");
    let t = step(c, &m, answer(SAY, 1, None)).await;
    assert_eq!(t.progress, [(EGG_QUEST, 2)]);
    assert_eq!(t.items, [(EGG, -1), (RUNE_SNAIL, 1)]);
    assert!(
        t.added
            .iter()
            .any(|&(kind, _, item)| (kind, item) == (CASH, RUNE_SNAIL)),
        "{:?}",
        t.added
    );
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    let (serial, name) = carried_pet(id, RUNE_SNAIL).await.expect("the pet saved");
    assert!(serial > 0);
    assert!(!name.is_empty());
    let (_, hours) = item_flags_and_hours(id, RUNE_SNAIL).await.unwrap();
    assert_eq!(hours, Some(4), "five hours, less the test's seconds");
    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&EGG_QUEST), Some(&2));
}
