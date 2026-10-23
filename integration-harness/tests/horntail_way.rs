//! The way to Horntail: Keroben turns back a level-100 character who is
//! neither a dragon nor holds the Dragon Squad's certificate; with it he
//! sells them Power Elixirs and lets them into the Cave of Life, where the
//! Encrypted Slate of the Squad opens the way on to the Entrance to
//! Horntail's Cave. The Crystal of Roots and the Road in Between lead out.

use game_data::server_content::keys;
use integration_harness::database::{grant_stack, set_job, set_meso};
use integration_harness::events::{maps as fields, said, talk};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const KEROBEN: i32 = 2081005;
const SLATE: i32 = 2083000;
const CRYSTAL: i32 = 2083002;
const CAVE_OF_LIFE_ENTRANCE: i32 = 240040700;
const CAVE_ENTRANCE: i32 = 240050000;
const HORNTAIL_CAVE_ENTRANCE: i32 = 240050400;
const ROAD_IN_BETWEEN: i32 = 240050600;
const HORNTAILS_CAVE: i32 = 240060200;
const CERTIFICATE: i32 = 4001086;
const POWER_ELIXIR: i32 = 2000005;
const ETC: i16 = 4;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn maps(t: &Trace) -> Vec<(i32, u8)> {
    t.arrivals.iter().map(|a| (a.0, a.1)).collect()
}

#[tokio::test]
async fn the_dragon_squad_goes_on_to_horntails_cave_and_back_out() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 100, 50).await.expect("level 100");
    set_meso(id, 100_000).await.expect("100,000 mesos");

    // Neither a dragon nor holding the certificate: turned back.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, CAVE_OF_LIFE_ENTRANCE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(CAVE_OF_LIFE_ENTRANCE, KEROBEN)).await;
    assert_eq!(t.prompts, [(KEROBEN, 0)]);
    assert!(said(&t, keys::HORNTAIL_KEROBEN_AWAY), "{t:?}");
    step(c, &m, answer(0, 0, None)).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 20, CERTIFICATE, 1)
        .await
        .expect("the Dragon Squad's certificate");

    // With it, Keroben sells ten Power Elixirs, then lets them in.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, CAVE_OF_LIFE_ENTRANCE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(CAVE_OF_LIFE_ENTRANCE, KEROBEN)).await;
    assert_eq!(t.prompts, [(KEROBEN, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.items, [(POWER_ELIXIR, 10)], "{t:?}");
    assert_eq!(t.meso_gains, [-100_000], "{t:?}");
    assert!(said(&t, keys::HORNTAIL_KEROBEN_BOUGHT), "{t:?}");
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, talk(CAVE_OF_LIFE_ENTRANCE, KEROBEN)).await;
    let t = step(c, &m, answer(4, 1, Some(1))).await;
    assert_eq!(maps(&t), [(CAVE_ENTRANCE, 0)]);

    // The slate takes the certificate's holder on.
    let t = step(c, &m, talk(CAVE_ENTRANCE, SLATE)).await;
    assert_eq!(t.prompts, [(SLATE, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(maps(&t), [(HORNTAIL_CAVE_ENTRANCE, 0)]);

    // From the entrance, the Crystal of Roots leads out of the cave.
    let t = step(c, &m, talk(HORNTAIL_CAVE_ENTRANCE, CRYSTAL)).await;
    assert_eq!(t.prompts, [(CRYSTAL, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    let field = fields().field(CAVE_OF_LIFE_ENTRANCE).unwrap();
    let out00 = field.portals().find(|p| p.name == "out00").unwrap().id as u8;
    assert_eq!(maps(&t), [(CAVE_OF_LIFE_ENTRANCE, out00)]);

    // From beyond it, to the Road in Between, which leads back.
    let mut t = Trace::default();
    warp(c, HORNTAILS_CAVE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    step(c, &m, talk(HORNTAILS_CAVE, CRYSTAL)).await;
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(maps(&t), [(ROAD_IN_BETWEEN, 0)]);
    let t = step(c, &m, enter("next00")).await;
    assert_eq!(maps(&t), [(HORNTAIL_CAVE_ENTRANCE, 0)]);
}
