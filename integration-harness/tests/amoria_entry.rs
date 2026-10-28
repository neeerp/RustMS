//! The Amorian Challenge, its first slice: Amos the Strong on his training
//! ground trades 10 Lip Lock Keys for an Entrance Ticket and takes the
//! ticket to let one into the Entrance; Amos there takes the leader's party
//! of six married men and women in, or sends one back; a member leaving the
//! party leaves too few, which ends it for everyone, who land on the way
//! out; and the Glimmer Man there sends them back to the training ground.

use game_data::server_content::keys;
use integration_harness::events::{
    couples_in, enter_amoria, leave_party, party, said, talk, told, waiting_in,
};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use net::events::{
    AMORIA, AMORIA_ENTRY, AMORIA_EXIT, AMORIA_HALL, AMORIA_LOBBY, AMORIA_TICKET, LIP_LOCK_KEY,
};
use net::npc_scripts::{AMOS_HALL, AMOS_LOBBY, GLIMMER_MAN};

const ETC: u8 = 4;

#[tokio::test]
async fn amos_trades_keys_for_a_ticket_that_lets_one_into_the_entrance() {
    // Nine keys are one short, and with no ticket there is no way in.
    let mut short = waiting_in(AMORIA_HALL, 40, &[(LIP_LOCK_KEY, 9)]).await;
    let menu = told(&mut short, talk(AMORIA_HALL, AMOS_HALL)).await;
    assert_eq!(menu.prompts, [(AMOS_HALL, 4)], "{menu:?}");
    let t = told(&mut short, answer(4, 1, Some(1))).await;
    assert!(said(&t, keys::AMORIA_PQ_HALL_NO_KEYS), "{t:?}");
    told(&mut short, answer(0, 1, None)).await;
    told(&mut short, talk(AMORIA_HALL, AMOS_HALL)).await;
    let t = told(&mut short, answer(4, 1, Some(0))).await;
    assert!(said(&t, keys::AMORIA_PQ_HALL_NO_TICKET), "{t:?}");
    told(&mut short, answer(0, 1, None)).await;

    // Ten of twelve go for a ticket.
    let mut player = waiting_in(AMORIA_HALL, 40, &[(LIP_LOCK_KEY, 12)]).await;
    told(&mut player, talk(AMORIA_HALL, AMOS_HALL)).await;
    let t = told(&mut player, answer(4, 1, Some(1))).await;
    assert!(said(&t, keys::AMORIA_PQ_HALL_TRADE_ASK), "{t:?}");
    send(&mut player.connection, answer(1, 1, None)).await;
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| {
        t.quantities.contains(&(ETC, 1, 2)) && t.added.iter().any(|a| a.2 == AMORIA_TICKET)
    })
    .await;
    let slot = t.added.iter().find(|a| a.2 == AMORIA_TICKET).unwrap().1;

    // One ticket is all one may hold.
    told(&mut player, talk(AMORIA_HALL, AMOS_HALL)).await;
    let t = told(&mut player, answer(4, 1, Some(1))).await;
    assert!(said(&t, keys::AMORIA_PQ_HALL_HAS_TICKET), "{t:?}");
    told(&mut player, answer(0, 1, None)).await;

    // The ticket lets them into the Entrance, and goes.
    told(&mut player, talk(AMORIA_HALL, AMOS_HALL)).await;
    let t = told(&mut player, answer(4, 1, Some(0))).await;
    assert!(said(&t, keys::AMORIA_PQ_HALL_ENTER_ASK), "{t:?}");
    send(&mut player.connection, answer(1, 1, None)).await;
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| {
        t.warps.contains(&AMORIA_LOBBY) && t.removed.contains(&(ETC, slot))
    })
    .await;

    // The Entrance has no way out but Amos, who sends them back.
    told(&mut player, talk(AMORIA_LOBBY, AMOS_LOBBY)).await;
    let t = told(&mut player, answer(4, 1, Some(1))).await;
    assert_eq!(t.warps, [AMORIA_HALL], "{t:?}");
}

#[tokio::test]
async fn amos_takes_three_married_couples_in_and_a_loss_ends_it() {
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[]).await;
    party(&mut team).await;

    // With one of the six away, Amos won't take the party in.
    let mut trace = Trace::default();
    let away = &mut team[5].connection;
    assert_eq!(warp(away, AMORIA_HALL, "", &mut trace).await, AMORIA_HALL);
    told(&mut team[1], talk(AMORIA_LOBBY, AMOS_LOBBY)).await;
    let t = told(&mut team[1], answer(4, 1, Some(0))).await;
    assert!(said(&t, keys::AMORIA_PQ_LOBBY_NOT_LEADER), "{t:?}");
    told(&mut team[1], answer(0, 1, None)).await;
    told(&mut team[0], talk(AMORIA_LOBBY, AMOS_LOBBY)).await;
    let t = told(&mut team[0], answer(4, 1, Some(0))).await;
    let not_eligible = net::content::text().get(
        keys::AMORIA_PQ_LOBBY_NOT_ELIGIBLE,
        &[
            ("least", &AMORIA.least().to_string()),
            ("low", &AMORIA.levels.start().to_string()),
        ],
    );
    assert!(
        t.prompt_texts
            .iter()
            .any(|said| said.contains(&not_eligible)),
        "{t:?}"
    );
    told(&mut team[0], answer(0, 1, None)).await;

    // With them back, all six go in.
    let mut trace = Trace::default();
    let back = &mut team[5].connection;
    assert_eq!(warp(back, AMORIA_LOBBY, "", &mut trace).await, AMORIA_LOBBY);
    enter_amoria(&mut team).await;

    // A member leaves the party: the five left are too few, and everyone
    // lands on the way out.
    send(&mut team[3].connection, leave_party()).await;
    for player in team.iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&AMORIA_EXIT)
        })
        .await;
        assert!(!trace.warps.contains(&AMORIA_ENTRY), "{trace:?}");
    }

    // The Glimmer Man there sends them back to the training ground.
    let t = told(&mut team[0], talk(AMORIA_EXIT, GLIMMER_MAN)).await;
    assert_eq!(t.prompts, [(GLIMMER_MAN, 1)], "{t:?}");
    let t = told(&mut team[0], answer(1, 1, None)).await;
    assert_eq!(t.warps, [AMORIA_HALL], "{t:?}");
}
