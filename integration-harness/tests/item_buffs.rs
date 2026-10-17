//! A pill raises a stat for its `time` as a buff of id `-item_id`, which
//! right-clicking its icon (CANCEL_ITEM_EFFECT) ends; All Cure is spent
//! with nothing to cure.

use integration_harness::database::grant_stack;
use integration_harness::hunting::{cancel_item_effect, use_item};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const USE: i16 = 2;
/// Warrior Pill: +5 weapon attack for three minutes.
const WARRIOR_PILL: i32 = 2_002_004;
const ALL_CURE: i32 = 2_050_004;
/// Weapon attack's CANCEL_BUFF bit (second mask).
const PAD_MASK: i64 = 0x1_0000_0000;

#[tokio::test]
async fn a_pill_buffs_until_its_icon_is_right_clicked() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, USE, 20, WARRIOR_PILL, 2)
        .await
        .expect("grant pills");
    grant_stack(id, USE, 21, ALL_CURE, 1)
        .await
        .expect("grant All Cure");
    let mut player = relogin(&config).await;
    let c = &mut player.connection;

    let t = step(c, "pill", use_item(20, WARRIOR_PILL)).await;
    assert_eq!(t.buffs, [(-WARRIOR_PILL, 5, 180_000)]);

    let t = step(c, "cancel", cancel_item_effect(-WARRIOR_PILL)).await;
    assert_eq!(t.cancelled_buffs, [PAD_MASK]);
    let t = step(c, "cancel again", cancel_item_effect(-WARRIOR_PILL)).await;
    assert!(t.cancelled_buffs.is_empty());

    let t = step(c, "all cure", use_item(21, ALL_CURE)).await;
    assert!(t.buffs.is_empty() && t.cancelled_buffs.is_empty());
    assert_eq!(t.removed, [(USE as u8, 21)]);
}
