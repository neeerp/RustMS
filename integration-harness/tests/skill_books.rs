//! A skill book teaches its skill's master level to a job that has the
//! skill and is spent; a book for another job is refused and kept.

use integration_harness::database::{grant_item, set_job};
use integration_harness::hunting::use_skill_book;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const USE: i16 = 2;
const ARCH_MAGE_FP: i16 = 212;
/// [Skill Book] Fire Demon: master level 10, always works.
const FIRE_DEMON_BOOK: i32 = 2280000;
const FIRE_DEMON: i32 = 2121003;
/// [Mastery Book] Monster Magnet, for Heroes, Paladins and Dark Knights.
const MONSTER_MAGNET_BOOK: i32 = 2290000;

#[tokio::test]
async fn a_skill_book_teaches_its_job_and_no_other() {
    let config = load_harness_config_or_fail().await;
    let mage_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(mage_id, ARCH_MAGE_FP, 120, 5000)
        .await
        .expect("make an Arch Mage");
    grant_item(mage_id, USE, 1, FIRE_DEMON_BOOK)
        .await
        .expect("grant the skill book");
    grant_item(mage_id, USE, 2, MONSTER_MAGNET_BOOK)
        .await
        .expect("grant the mastery book");

    let mut mage = relogin(&config).await;
    let c = &mut mage.connection;

    let t = step(c, "magnet", use_skill_book(2, MONSTER_MAGNET_BOOK)).await;
    assert_eq!(t.books, [(mage_id, 0, 0, false, false)]);
    assert!(t.removed.is_empty() && t.skills.is_empty());

    // A slot holding something else is refused without a result.
    let t = step(c, "wrong slot", use_skill_book(2, FIRE_DEMON_BOOK)).await;
    assert!(t.books.is_empty() && t.removed.is_empty());

    let t = step(c, "fire demon", use_skill_book(1, FIRE_DEMON_BOOK)).await;
    assert_eq!(t.books, [(mage_id, FIRE_DEMON, 10, true, true)]);
    assert_eq!(t.removed, [(USE as u8, 1)]);
    assert_eq!(t.skills, [(FIRE_DEMON, 0)]);
}
