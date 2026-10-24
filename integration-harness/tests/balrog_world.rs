//! "Junior Balrog in a Different World" (6108): the Insignificant Being
//! takes a Bowmaster or Marksman on the quest straight into the other world,
//! one at a time, where the Junior Balrogs count toward the quest; its way
//! out leads back, and the next Bowman may go in.

use game_data::server_content::keys;
use integration_harness::database::{
    replace_item, set_job, set_str, set_weapon_attack, start_quest,
};
use integration_harness::events::{said, talk, told};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::scripts::BALROG_WORLD;
use tokio::time::Duration;

const ANOTHER_ENTRANCE: i32 = 105090200;
const INSIGNIFICANT_BEING: i32 = 1061012;
const JUNIOR_BALROG: i32 = 9300085;
const IN_A_DIFFERENT_WORLD: i16 = 6108;

/// A new level-120 Marksman on 6108 hitting very hard, logged in by the
/// Insignificant Being.
async fn marksman() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 322, 120, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    start_quest(id, IN_A_DIFFERENT_WORLD).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, ANOTHER_ENTRANCE, "", &mut seen).await,
        ANOTHER_ENTRANCE
    );
    session
}

/// `player` talks to the Insignificant Being and is taken in.
async fn go_in(player: &mut WorldSession) -> Trace {
    let mut seen = told(player, talk(ANOTHER_ENTRANCE, INSIGNIFICANT_BEING)).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&BALROG_WORLD)
    })
    .await;
    seen
}

#[tokio::test]
async fn the_insignificant_being_lets_one_bowman_at_a_time_hunt_the_junior_balrog() {
    let mut a = marksman().await;
    let mut b = marksman().await;

    // One goes in; the next is turned away while the world is taken.
    let mut seen = go_in(&mut a).await;
    let trace = told(&mut b, talk(ANOTHER_ENTRANCE, INSIGNIFICANT_BEING)).await;
    assert!(said(&trace, keys::BALROG_WORLD_BUSY), "{trace:?}");
    assert!(trace.warps.is_empty(), "{trace:?}");

    // A Junior Balrog there counts toward the quest.
    let c = &mut a.connection;
    until(c, &mut seen, |t| !t.field.alive(JUNIOR_BALROG).is_empty()).await;
    let balrog = seen.field.alive(JUNIOR_BALROG)[0];
    send(c, attack(balrog, 5_000_000)).await;
    until(c, &mut seen, |t| {
        t.kills.get(&IN_A_DIFFERENT_WORLD).map(String::as_str) == Some("001")
    })
    .await;

    // The way out leads back, and the next may go in.
    let mut out = Trace::default();
    assert_eq!(warp(c, -1, "west00", &mut out).await, ANOTHER_ENTRANCE);
    go_in(&mut b).await;
}
