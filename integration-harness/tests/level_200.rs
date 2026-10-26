//! Reaching level 200: the whole world hears of it in a notice (Cosmic;
//! OrionAlpha tells the channel).

use integration_harness::database::{set_exp, set_job};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_000;
const SNAIL: i32 = 100100;
/// The EXP from level 199 to 200 (`game::exp::TO_NEXT_LEVEL[199]`).
const TO_200: i32 = 1_608_855_764;
/// BROADCAST_MSG's notice.
const NOTICE: u8 = 0;

async fn new_character(config: &HarnessConfig) -> (i32, String) {
    let session = login_to_world_session(config).await.expect("first login");
    let id = (session.character_id, session.character_name.clone());
    drop(session);
    drop(relogin(config).await);
    id
}

#[tokio::test]
async fn reaching_level_200_is_told_to_the_world() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let (a_id, a_name) = new_character(&a_config).await;
    new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    // A Fighter one EXP short of level 200.
    set_job(a_id, 110, 199, 100).await.expect("level 199");
    set_exp(a_id, TO_200 - 1).await.expect("almost 200");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;
    let (mut at, mut bt) = (Trace::default(), Trace::default());
    read_for(&mut b.connection, &mut bt, 300).await;

    let c = &mut a.connection;
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut at).await, HUNTING_GROUND);
    until(c, &mut at, |t| !t.field.alive(SNAIL).is_empty()).await;
    let target = at.field.alive(SNAIL)[0];
    send(c, attack(target, 8)).await;
    until(c, &mut at, |t| t.field.killed.contains(&target)).await;

    let told = |t: &Trace| {
        t.broadcasts
            .iter()
            .any(|(kind, text)| *kind == NOTICE && text.contains(&a_name))
    };
    until(&mut b.connection, &mut bt, told).await;
    until(c, &mut at, told).await;
}
