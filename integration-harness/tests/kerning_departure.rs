//! Kerning Square's 7th floor: June (1052125) takes one helping Blake
//! (2286-2288) up to the Spirit of Rock's floors alone, where the way on
//! opens with Blake's requests, and takes 10 VIP Invitations to let one into
//! the VIP Zone (Admission to the VIP Zone, 2291).

use game_data::server_content::keys;
use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::{said, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END};
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::events::{KERNING_SQUARE_7F, ROCK_SPIRIT_FLOOR, ROCK_SPIRIT_VIP_FLOOR};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const JUNE: i32 = 1_052_125;
const JUNE_AT: (i16, i16) = (-995, 87);
const FANCY_AMPLIFIER_FLOOR: i32 = 103_040_420;
const VIP_INVITATION: i32 = 4_032_521;
const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn june_lets_blakes_helper_and_vip_guests_up() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 40, 100).await.unwrap();
    set_quest(id, 2286, 2, "").await.unwrap();
    set_quest(id, 2287, 1, "").await.unwrap();
    set_quest(id, 2290, 2, "").await.unwrap();
    set_quest(id, 2291, 1, "").await.unwrap();
    grant_stack(id, 4, 1, VIP_INVITATION, 10).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, KERNING_SQUARE_7F, "", &mut trace).await,
        KERNING_SQUARE_7F
    );
    step(c, &m, walk_to(JUNE_AT.0, JUNE_AT.1)).await;

    // Helping Blake: up to the Cheap Amplifiers' floor, alone.
    let t = step(c, &m, talk(KERNING_SQUARE_7F, JUNE)).await;
    assert!(said(&t, keys::JUNE_MENU), "{t:?}");
    send(c, answer(4, 1, Some(0))).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| {
        t.warps == [ROCK_SPIRIT_FLOOR] && !t.countdowns.is_empty()
    })
    .await;
    assert_eq!(t.countdowns.len(), 1, "the hour runs");

    // On Composition Fee (2287), the way on to the Fancy Amplifiers opens.
    step(c, &m, walk_to(-1144, 416)).await;
    send(c, enter("left00")).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| t.warps == [FANCY_AMPLIFIER_FLOOR]).await;

    // Not yet on Say "NO" to Plagiarism (2288): the stage stays shut.
    step(c, &m, walk_to(-1175, 91)).await;
    let t = step(c, &m, enter("left01")).await;
    let line = net::content::text().get(keys::DEPART_NO_ACCESS, &[]);
    assert!(t.broadcasts.contains(&(PINK, line)), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");

    // Back down, and out to the 7th floor by the first floor's door.
    step(c, &m, walk_to(1256, 418)).await;
    send(c, enter("right00")).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| t.warps == [ROCK_SPIRIT_FLOOR]).await;
    step(c, &m, walk_to(1259, 85)).await;
    let mut t = Trace::default();
    assert_eq!(warp(c, -1, "right01", &mut t).await, KERNING_SQUARE_7F);

    // June takes the invitations and lets the player into the VIP Zone,
    // which completes 2291.
    step(c, &m, walk_to(JUNE_AT.0, JUNE_AT.1)).await;
    let t = step(c, &m, build_quest_action(SCRIPT_END, 2291, JUNE)).await;
    assert!(said(&t, keys::JUNE_VIP_WELCOME), "{t:?}");
    send(c, answer(0, 1, None)).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| t.warps == [ROCK_SPIRIT_VIP_FLOOR]).await;
    assert_eq!(t.states.get(&2291), Some(&2));
    assert_eq!(t.removed, [(4, 1)]);
}
