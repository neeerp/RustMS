//! The Forest of Poison Haze's rewards. On the prize map, the fountain
//! takes the Poison Golem's marble and gives everyone there 52,000 EXP;
//! once it has, the way home takes the forest's items and gives an Altaire
//! Fragment. Ellin trades ten fragments for Altaire Earrings. The prize map
//! is one map for every party, and its fountain stays used for five
//! minutes, so a second run on the same stack within them fails.

use game_data::server_content::keys;
use integration_harness::events::{go_to, maps, said, talk, throw_down, told, waiting_in};
use integration_harness::hunting::walk_to;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    ALTAIRE_FRAGMENT, ELLIN_EXIT, ELLIN_FOUNTAIN_EXP, ELLIN_LOBBY, GOLEMS_MARBLE, MONSTER_MARBLE,
};
use net::npc_scripts::ELLIN as ELLIN_NPC;
use net::packet::op::RecvOpcode;
use net::reactors::ELLIN_FOUNTAIN;
use net::scripts::ELLIN_HOME;
use packet::io::write::PktWrite;
use packet::Packet;

const ALTAIRE_EARRINGS: i32 = 1_032_060;

/// Walk to the prize map's way home and press Up, then wait to be home.
async fn go_home(player: &mut WorldSession) -> Trace {
    let field = maps().field(ELLIN_EXIT).unwrap();
    let portal = (field.portals())
        .find(|p| p.script.as_deref() == Some(ELLIN_HOME))
        .unwrap();
    let c = &mut player.connection;
    send(c, walk_to(portal.x as i16, portal.y as i16)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(&portal.name).unwrap();
    p.write_short(0).unwrap();
    send(c, p).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&ELLIN_LOBBY)).await;
    trace
}

#[tokio::test]
async fn the_fountain_gives_everyone_exp_and_then_the_way_home_a_fragment() {
    let mut thrower = waiting_in(ELLIN_LOBBY, 50, &[(GOLEMS_MARBLE, 1), (MONSTER_MARBLE, 3)]).await;
    let mut other = waiting_in(ELLIN_LOBBY, 50, &[]).await;
    let mut seen_by_other = Trace::default();
    let c = &mut other.connection;
    assert_eq!(
        warp(c, ELLIN_EXIT, "", &mut seen_by_other).await,
        ELLIN_EXIT
    );
    // The way home finds the fountain as the map's unnamed reactor.
    assert_eq!(maps().field(ELLIN_EXIT).unwrap().reactors.len(), 1);
    let (mut seen, fountain) = go_to(&mut thrower, ELLIN_EXIT, ELLIN_FOUNTAIN).await;
    assert_eq!(fountain.state, 0, "used in the last five minutes");

    // The marble on the fountain: EXP for both.
    let c = &mut thrower.connection;
    send(c, walk_to(fountain.x, fountain.y - 10)).await;
    send(c, throw_down(1, 1)).await;
    until(c, &mut seen, |t| {
        t.field.reactor_hits.contains(&(fountain.object_id, 1))
            && t.gains.contains(&ELLIN_FOUNTAIN_EXP)
    })
    .await;
    until(&mut other.connection, &mut seen_by_other, |t| {
        t.gains.contains(&ELLIN_FOUNTAIN_EXP)
    })
    .await;

    // Home, each with a fragment, the thrower without the Monster Marbles.
    let home = go_home(&mut other).await;
    assert!(
        home.added
            .iter()
            .any(|&(_, _, item)| item == ALTAIRE_FRAGMENT),
        "{home:?}"
    );
    let home = go_home(&mut thrower).await;
    assert!(home.removed.contains(&(4, 2)), "{home:?}");
    assert!(
        home.added
            .iter()
            .any(|&(_, _, item)| item == ALTAIRE_FRAGMENT),
        "{home:?}"
    );
}

#[tokio::test]
async fn ellin_trades_ten_fragments_for_altaire_earrings() {
    let mut player = waiting_in(ELLIN_LOBBY, 50, &[(ALTAIRE_FRAGMENT, 10)]).await;
    for (choice, expect) in [(0, None), (1, Some(keys::ELLIN_PRIZE_NO_ALTAIRE))] {
        told(&mut player, talk(ELLIN_LOBBY, ELLIN_NPC)).await;
        let menu = told(&mut player, answer(4, 1, Some(2))).await;
        assert_eq!(menu.prompts, [(ELLIN_NPC, 4)], "{menu:?}");
        let trace = told(&mut player, answer(4, 1, Some(choice))).await;
        match expect {
            None => {
                assert!(
                    (trace.added.iter()).any(|&(_, _, item)| item == ALTAIRE_EARRINGS),
                    "{trace:?}"
                );
                assert!(trace.removed.contains(&(4, 1)), "{trace:?}");
            }
            Some(key) => assert!(said(&trace, key), "{trace:?}"),
        }
    }
}
