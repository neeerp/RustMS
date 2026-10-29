//! Two of the Mushroom Castle's portal scripts. On the Watch's trap
//! (`trap00`, `TD_MC_faild`) has the guard spot the player: its look, a
//! line in the chat log, and two seconds on the foot of the Shadow Cliffs.
//! The Central Castle Tower's `find_james` has James call out under a
//! balloon until James's Whereabouts (1) is done.
use game_data::server_content::keys;
use integration_harness::database::set_quest;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::{Duration, Instant};

const ON_THE_WATCH: i32 = 106_020_601;
const TRAP_AT: (i16, i16) = (-936, -139);
const SHADOW_CLIFFS: i32 = 106_020_403;
const CASTLE_TOWER: i32 = 106_021_201;
const JAMES_AT: (i16, i16) = (-46, 96);
const JAMES_FOUND: i16 = 2325;
const BALLOON: &str = "Effect/OnUserEff.img/normalEffect/mushroomcastle/chatBalloon2";

/// Going through a scripted portal.
fn touch(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn the_guard_on_the_watch_sends_one_it_spots_to_the_cliffs_foot() {
    let config = load_harness_config_or_fail().await;
    let name = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, ON_THE_WATCH, "", &mut seen).await, ON_THE_WATCH);
    step(c, &name, walk_to(TRAP_AT.0, TRAP_AT.1)).await;

    // Seen: the guard's look and the line, and the player stays a while.
    let touched = Instant::now();
    let t = step(c, &name, touch("trap00")).await;
    let line = net::content::text().get(keys::MC_WATCH_SPOTTED, &[]);
    assert_eq!(
        t.scene_parts,
        ["Effect/Direction2.img/mushCatle/nugu"],
        "{t:?}"
    );
    assert_eq!(t.info_texts, [line], "{t:?}");
    assert!(t.arrivals.is_empty(), "{t:?}");

    // Then down to the cliffs' foot.
    let mut t = Trace::default();
    until(c, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == SHADOW_CLIFFS)
    })
    .await;
    assert!(touched.elapsed() >= Duration::from_millis(1_500), "{t:?}");
}

#[tokio::test]
async fn james_calls_out_from_the_tower_until_found() {
    let config = load_harness_config_or_fail().await;
    let name = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, CASTLE_TOWER, "", &mut seen).await, CASTLE_TOWER);
    step(c, &name, walk_to(JAMES_AT.0, JAMES_AT.1)).await;
    let t = step(c, &name, touch("find_james")).await;
    assert_eq!(t.images, [BALLOON], "{t:?}");

    // Found, James is quiet.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, JAMES_FOUND, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, CASTLE_TOWER, "", &mut seen).await, CASTLE_TOWER);
    step(c, &name, walk_to(JAMES_AT.0, JAMES_AT.1)).await;
    let t = step(c, &name, touch("find_james")).await;
    assert!(t.images.is_empty(), "{t:?}");
}
