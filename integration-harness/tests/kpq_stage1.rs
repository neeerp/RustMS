//! Kerning City's party quest, stage 1: Cloto asks each member but the
//! leader for a number of coupons, gives a pass for exactly that many, and
//! the leader clears the stage by handing in a pass from each of them,
//! which opens the way on to stage 2 and gives everyone its EXP. The
//! harness world pins her question (`RUSTMS_KPQ_QUESTION`), so the
//! members' coupons can be granted before they log in.

use integration_harness::events::{enter_kpq, in_lobby, party, talk, CLOTO, NELLA};
use integration_harness::hunting::pickup;
use integration_harness::packets::build_change_map;
use integration_harness::quests::answer;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{ANSWERS, COUPON, KPQ, PASS};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const ETC: u8 = 4;
/// The question the harness world pins, 1 to 7.
const QUESTION: usize = 7;

/// Throw `quantity` of the Etc stack in `slot` on the ground.
fn throw_down(slot: i16, quantity: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(ETC).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(quantity).unwrap();
    p
}

/// A new level-25 character holding `quantity` of `item_id` in Etc slot 1,
/// logged in and waiting in Kerning City.
async fn player(item_id: i32, quantity: i16) -> WorldSession {
    in_lobby(25, &[(item_id, quantity)]).await
}

/// What `player` is shown on talking to Cloto: one text page, and the
/// trace that came with it.
async fn ask_cloto(player: &mut WorldSession) -> (String, Trace) {
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, talk(KPQ.entry, CLOTO)).await;
    assert_eq!(trace.prompts, [(CLOTO, 0)], "{trace:?}");
    (trace.prompt_texts[0].clone(), trace)
}

/// What a member on the stage's map is shown on its clear: the clear on
/// screen, its sound, and the gate opening.
fn clear_effects() -> Vec<(u8, String)> {
    vec![
        (3, "quest/party/clear".into()),
        (4, "Party1/Clear".into()),
        (2, "gate".into()),
    ]
}

fn nothing_changed_hands(trace: &Trace) {
    assert!(
        trace.added.is_empty() && trace.removed.is_empty() && trace.quantities.is_empty(),
        "{trace:?}"
    );
}

#[tokio::test]
async fn cloto_gives_passes_for_coupons_and_the_leader_clears_stage_one() {
    let answer_count = ANSWERS[QUESTION - 1] as i16;
    let mut team = vec![
        player(PASS, 1).await,
        player(COUPON, answer_count).await,
        player(COUPON, answer_count - 1).await,
    ];
    party(&mut team).await;

    enter_kpq(&mut team).await;
    let [leader, right, wrong] = &mut team[..] else {
        unreachable!()
    };

    // The way on is shut until the stage is cleared, for everyone.
    for player in [&mut *leader, &mut *wrong] {
        send(
            &mut player.connection,
            build_change_map(-1, "next00").unwrap(),
        )
        .await;
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            !t.blocked.is_empty()
        })
        .await;
        assert_eq!(trace.blocked, [(SendOpcode::BlockedMap as i16, 1)]);
        assert!(trace.warps.is_empty(), "{trace:?}");
    }

    // Each member is asked first, whatever they hold.
    let (question, trace) = ask_cloto(right).await;
    nothing_changed_hands(&trace);
    assert_eq!(ask_cloto(wrong).await.0, question);

    // One coupon short is wrong, and costs nothing; she asks again.
    let (told, trace) = ask_cloto(wrong).await;
    nothing_changed_hands(&trace);
    assert!(told != question && told.contains(&question), "{told}");

    // The right number earns a pass, once.
    let (_, trace) = ask_cloto(right).await;
    assert_eq!(trace.removed, [(ETC, 1)]);
    let pass_slot = match trace.added.as_slice() {
        [(ETC, slot, PASS)] => *slot,
        other => panic!("{other:?}"),
    };
    nothing_changed_hands(&ask_cloto(right).await.1);

    // The leader needs a pass from each of the others.
    nothing_changed_hands(&ask_cloto(leader).await.1);

    // Passes change hands: dropped, and picked up.
    send(&mut right.connection, throw_down(pass_slot, 1)).await;
    let mut trace = Trace::default();
    until(&mut leader.connection, &mut trace, |t| {
        t.field.item_drop(PASS).is_some()
    })
    .await;
    let pass = trace.field.item_drop(PASS).unwrap();
    send(
        &mut leader.connection,
        pickup(pass.object_id, pass.x, pass.y),
    )
    .await;
    until(&mut leader.connection, &mut trace, |t| {
        t.quantities.contains(&(ETC, 1, 2))
    })
    .await;

    // Both passes clear the stage: everyone on it sees and hears it, the
    // gate opens, and each gets the stage's EXP.
    let (cleared, mut trace) = ask_cloto(leader).await;
    assert_eq!(trace.removed, [(ETC, 1)]);
    until(&mut leader.connection, &mut trace, |t| !t.gains.is_empty()).await;
    assert_eq!(trace.field_effects, clear_effects());
    assert_eq!(trace.gains, [KPQ.stage_exp(1)]);
    for member in [&mut *right, &mut *wrong] {
        let mut trace = Trace::default();
        until(&mut member.connection, &mut trace, |t| !t.gains.is_empty()).await;
        assert_eq!(trace.field_effects, clear_effects());
        assert_eq!(trace.gains, [KPQ.stage_exp(1)]);
    }

    // Cloto says so to everyone.
    let (told, trace) = ask_cloto(wrong).await;
    nothing_changed_hands(&trace);
    assert!(!told.contains(&question), "{told}");
    assert_ne!(told, cleared);

    // Through it the leader goes on to stage 2, and back on stage 1 the
    // gate is still open.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut leader.connection, -1, "next00", &mut trace).await,
        KPQ.stage_map(2)
    );
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut leader.connection, KPQ.entry, "", &mut trace).await,
        KPQ.entry
    );
    until(&mut leader.connection, &mut trace, |t| {
        !t.field_effects.is_empty()
    })
    .await;
    assert_eq!(trace.field_effects, [(2, "gate".to_string())]);

    // Leaving takes the quest's own items: the coupons still held.
    let name = wrong.character_name.clone();
    let trace = step(&mut wrong.connection, &name, talk(KPQ.entry, NELLA)).await;
    assert_eq!(trace.prompts, [(NELLA, 1)], "{trace:?}");
    let mut trace = step(&mut wrong.connection, &name, answer(1, 1, None)).await;
    until(&mut wrong.connection, &mut trace, |t| {
        t.warps.contains(&KPQ.exit) && !t.removed.is_empty()
    })
    .await;
    assert_eq!(trace.removed, [(ETC, 1)]);
}
