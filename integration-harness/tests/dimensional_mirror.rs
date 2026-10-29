//! The Dimensional Mirror (9010022): a slide menu (NPC dialog type 14) of
//! the places open at the player's level. Picking one keeps the town for
//! the way back; the Monster Carnival's two fields share their way back.

use game_data::server_content::keys;
use integration_harness::events::{talk, told, waiting_in};
use integration_harness::quests::answer;
use net::carnival::CARNIVAL2_LOBBY;
use net::npc_scripts::DIMENSIONAL_MIRROR;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const HENESYS: i32 = 100_000_000;
const PYRAMID_LOBBY: i32 = 926_010_000;
const ARIANT_LOBBY: i32 = 980_010_000;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn text(key: &str) -> String {
    net::content::text().get(key, &[])
}

#[tokio::test]
async fn the_mirror_slides_to_the_pyramid_and_the_way_out_comes_back() {
    let mut player = waiting_in(HENESYS, 45, &[]).await;
    let menu = told(&mut player, talk(HENESYS, DIMENSIONAL_MIRROR)).await;
    assert_eq!(menu.prompts, [(DIMENSIONAL_MIRROR, 14)], "{menu:?}");
    assert_eq!(menu.slide_sets, [0]);
    let expected = format!(
        "#1#{}#2#{}#5#{}",
        text(keys::MIRROR_DOJO),
        text(keys::MIRROR_CARNIVAL),
        text(keys::MIRROR_PYRAMID)
    );
    assert_eq!(menu.prompt_texts, [expected]);

    let went = told(&mut player, answer(14, 1, Some(5))).await;
    assert!(
        matches!(went.arrivals[..], [(PYRAMID_LOBBY, 4, _)]),
        "{went:?}"
    );
    let back = told(&mut player, enter("out00")).await;
    assert!(matches!(back.arrivals[..], [(HENESYS, _, _)]), "{back:?}");
}

#[tokio::test]
async fn from_level_51_the_mirror_slides_to_carnival_field_2_and_back() {
    let mut player = waiting_in(HENESYS, 60, &[]).await;
    let menu = told(&mut player, talk(HENESYS, DIMENSIONAL_MIRROR)).await;
    let expected = format!(
        "#1#{}#3#{}#5#{}",
        text(keys::MIRROR_DOJO),
        text(keys::MIRROR_CARNIVAL2),
        text(keys::MIRROR_PYRAMID)
    );
    assert_eq!(menu.prompt_texts, [expected]);

    let went = told(&mut player, answer(14, 1, Some(3))).await;
    assert!(
        matches!(went.arrivals[..], [(CARNIVAL2_LOBBY, 3, _)]),
        "{went:?}"
    );
    let back = told(&mut player, enter("out00")).await;
    assert!(matches!(back.arrivals[..], [(HENESYS, _, _)]), "{back:?}");
}

#[tokio::test]
async fn from_level_20_the_mirror_slides_to_the_ariant_coliseum_and_back() {
    let mut player = waiting_in(HENESYS, 22, &[]).await;
    let menu = told(&mut player, talk(HENESYS, DIMENSIONAL_MIRROR)).await;
    let expected = format!("#0#{}", text(keys::MIRROR_ARIANT));
    assert_eq!(menu.prompt_texts, [expected]);

    let went = told(&mut player, answer(14, 1, Some(0))).await;
    assert!(
        matches!(went.arrivals[..], [(ARIANT_LOBBY, 3, _)]),
        "{went:?}"
    );
    let back = told(&mut player, enter("out00")).await;
    assert!(matches!(back.arrivals[..], [(HENESYS, _, _)]), "{back:?}");
}

#[tokio::test]
async fn under_level_20_the_mirror_has_nowhere_to_send_anyone() {
    let mut player = waiting_in(HENESYS, 15, &[]).await;
    let menu = told(&mut player, talk(HENESYS, DIMENSIONAL_MIRROR)).await;
    assert_eq!(menu.prompts, [(DIMENSIONAL_MIRROR, 14)], "{menu:?}");
    let nowhere = format!("#-1#{}", text(keys::MIRROR_NOWHERE));
    assert_eq!(menu.prompt_texts, [nowhere]);

    // Only CANCEL works, and a forged pick of the empty entry goes nowhere.
    let closed = told(&mut player, answer(14, 0, None)).await;
    assert!(closed.arrivals.is_empty(), "{closed:?}");
    let menu = told(&mut player, talk(HENESYS, DIMENSIONAL_MIRROR)).await;
    assert_eq!(menu.prompts.len(), 1, "{menu:?}");
    let forged = told(&mut player, answer(14, 1, Some(-1))).await;
    assert!(forged.arrivals.is_empty(), "{forged:?}");
}
