//! So Gong, in the Mu Lung Dojo's hall, sells belts for training points,
//! resets the points, and takes a challenger's try for a medal; all of it
//! stays after logging out.

use game_data::server_content::keys;
use integration_harness::database::{set_dojo_points, set_job};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::DOJO;
use net::npc_scripts::SO_GONG;
use tokio::time::Duration;

const WHITE_BELT: i32 = 1132000;

/// Open So Gong's hall menu and pick `choice`.
async fn pick(player: &mut WorldSession, choice: i32) -> Trace {
    told(player, talk(DOJO.lobby, SO_GONG)).await;
    told(player, answer(4, 1, Some(choice))).await
}

fn shows(trace: &Trace, key: &str, params: &[(&str, &str)]) -> bool {
    let text = net::content::text().get(key, params);
    trace.prompt_texts.iter().any(|said| said.contains(&text))
}

async fn in_the_hall(config: &integration_harness::HarnessConfig) -> WorldSession {
    let mut player = relogin(config).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, DOJO.lobby, "", &mut seen).await, DOJO.lobby);
    player
}

#[tokio::test]
async fn so_gong_sells_a_belt_resets_the_points_and_takes_a_medal_try() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    set_dojo_points(id, 250).await.unwrap();
    let mut player = in_the_hall(&config).await;

    // The white belt costs 200 of the 250 points.
    let belts = pick(&mut player, 2).await;
    assert!(
        shows(&belts, keys::DOJO_SO_GONG_BELTS, &[("points", "250")]),
        "{belts:?}"
    );
    let white = told(&mut player, answer(4, 1, Some(0))).await;
    let belt = WHITE_BELT.to_string();
    assert!(
        shows(&white, keys::DOJO_SO_GONG_BELT_GIVEN, &[("belt", &belt)]),
        "{white:?}"
    );
    assert!(white.items.contains(&(WHITE_BELT, 1)), "{white:?}");
    send(&mut player.connection, answer(0, -1, None)).await;
    let belts = pick(&mut player, 2).await;
    assert!(
        shows(&belts, keys::DOJO_SO_GONG_BELTS, &[("points", "50")]),
        "{belts:?}"
    );
    assert!(
        shows(&belts, keys::DOJO_SO_GONG_BELT_HELD, &[("belt", &belt)]),
        "{belts:?}"
    );
    send(&mut player.connection, answer(0, -1, None)).await;

    // Resetting them leaves none, and the medal is tried for.
    let reset = pick(&mut player, 3).await;
    assert!(said(&reset, keys::DOJO_SO_GONG_RESET_ASK), "{reset:?}");
    let done = told(&mut player, answer(1, 1, None)).await;
    assert!(said(&done, keys::DOJO_SO_GONG_RESET_DONE), "{done:?}");
    send(&mut player.connection, answer(0, -1, None)).await;
    let medal = pick(&mut player, 4).await;
    let mano = [("medal", "1142033")];
    assert!(
        shows(&medal, keys::DOJO_SO_GONG_MEDAL_TRY, &mano),
        "{medal:?}"
    );
    send(&mut player.connection, answer(1, 1, None)).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    drop(player);

    // All of it is still so after logging out.
    let mut player = in_the_hall(&config).await;
    let belts = pick(&mut player, 2).await;
    assert!(
        shows(&belts, keys::DOJO_SO_GONG_BELTS, &[("points", "0")]),
        "{belts:?}"
    );
    send(&mut player.connection, answer(0, -1, None)).await;
    let medal = pick(&mut player, 4).await;
    let more = [("medal", "1142033"), ("kills", "100")];
    assert!(
        shows(&medal, keys::DOJO_SO_GONG_MEDAL_MORE, &more),
        "{medal:?}"
    );
}
