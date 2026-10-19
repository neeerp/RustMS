//! Mortal Blow: a Ranger's whack can come up as a shot of the skill, which
//! the server takes like any other shot, arrow and all.

use integration_harness::database::{grant_skill, grant_stack, replace_item, set_job};
use integration_harness::hunting::ranged_attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const MORTAL_BLOW: i32 = 3110001;
const WAR_BOW: i32 = 1452002;
const ARROW: i32 = 2060000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const USE: i16 = 2;
const ARROWS: i16 = 20;

#[tokio::test]
async fn a_mortal_blow_is_shot_like_any_other_skill() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 311, 70, 1000).await.expect("ranger");
    grant_skill(id, MORTAL_BLOW, 20).await.expect("Mortal Blow");
    replace_item(id, EQUIPPED, WEAPON, WAR_BOW)
        .await
        .expect("give a bow");
    grant_stack(id, USE, ARROWS, ARROW, 100)
        .await
        .expect("give arrows");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];

    let t = step(c, "blow", ranged_attack(MORTAL_BLOW, ARROWS, snail, 8)).await;
    assert_eq!(t.quantities, vec![(USE as u8, ARROWS, 99)]);
    assert!(!t.gains.is_empty(), "the snail died");
}
