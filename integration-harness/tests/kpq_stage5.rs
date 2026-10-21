//! Kerning City's party quest, stage 5: the monsters drop the passes, and
//! the leader hands Cloto ten of them to clear the stage, which wins the
//! party quest. Its clock stops, and it no longer ends at its time, when
//! the party breaks up, or when the leader leaves: only when the last of
//! the team has left. The harness world's event lasts 30 seconds.

use integration_harness::events::{
    enter_kpq, in_lobby, leave_party, party, talk, throw_down, CLOTO,
};
use integration_harness::hunting::pickup;
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{KPQ, PASS};
use tokio::time::{sleep_until, Duration, Instant};

const ETC: u8 = 4;
const OP_LEAVE: u8 = 0x0C;

/// What `player` is shown on talking to Cloto on stage 5: one text page,
/// and the trace that came with it.
async fn ask_cloto(player: &mut WorldSession) -> (String, Trace) {
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, talk(KPQ.stage_map(5), CLOTO)).await;
    assert_eq!(trace.prompts, [(CLOTO, 0)], "{trace:?}");
    (trace.prompt_texts[0].clone(), trace)
}

/// Debug-warp `player` to `map` within the event, and return what came
/// with it once the field has them.
async fn go_to(player: &mut WorldSession, map: i32) -> Trace {
    let name = player.character_name.clone();
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    barrier(&mut player.connection, &name, &mut trace).await;
    trace
}

fn nothing_changed_hands(trace: &Trace) {
    assert!(
        trace.added.is_empty() && trace.removed.is_empty() && trace.quantities.is_empty(),
        "{trace:?}"
    );
}

fn clear_effects() -> Vec<(u8, String)> {
    vec![
        (3, "quest/party/clear".into()),
        (4, "Party1/Clear".into()),
        (2, "gate".into()),
    ]
}

#[tokio::test]
async fn ten_passes_win_kpq_which_then_lasts_until_its_team_has_left() {
    let mut team = vec![
        in_lobby(25, &[(PASS, 9)]).await,
        in_lobby(25, &[(PASS, 1)]).await,
        in_lobby(25, &[]).await,
    ];
    party(&mut team).await;
    enter_kpq(&mut team).await;

    // Down to stage 5 by debug warp, the clock still running (shown again
    // there, after stage 1's if that was still unread).
    let mut deadline = None;
    for player in team.iter_mut() {
        let trace = go_to(player, KPQ.stage_map(5)).await;
        let &left = trace.countdowns.last().expect("a countdown");
        deadline.get_or_insert(Instant::now() + Duration::from_secs(left as u64 + 1));
    }
    let [leader, giver, other] = &mut team[..] else {
        unreachable!()
    };

    // Members are told to gather the passes; the leader is short of ten.
    let (gather, trace) = ask_cloto(giver).await;
    nothing_changed_hands(&trace);
    assert_eq!(ask_cloto(other).await.0, gather);
    let (short, trace) = ask_cloto(leader).await;
    nothing_changed_hands(&trace);
    assert_ne!(short, gather);

    // A member's pass makes ten.
    send(&mut giver.connection, throw_down(1, 1)).await;
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
        t.quantities.contains(&(ETC, 1, 10))
    })
    .await;

    // Cloto takes the ten: the stage is cleared, everyone gets its EXP,
    // and everyone's clock stops.
    let (won, mut trace) = ask_cloto(leader).await;
    assert!(won != short && won != gather, "{won}");
    assert_eq!(trace.removed, [(ETC, 1)]);
    until(&mut leader.connection, &mut trace, |t| !t.gains.is_empty()).await;
    assert_eq!(trace.field_effects, clear_effects());
    assert_eq!(trace.gains, [KPQ.stage_exp(5)]);
    assert_eq!(trace.clocks_stopped, 1);
    for member in [&mut *giver, &mut *other] {
        let mut trace = Trace::default();
        until(&mut member.connection, &mut trace, |t| !t.gains.is_empty()).await;
        assert_eq!(trace.field_effects, clear_effects());
        assert_eq!(trace.gains, [KPQ.stage_exp(5)]);
        assert_eq!(trace.clocks_stopped, 1);
    }

    // Cloto says so to everyone, and takes nothing more.
    let (told, trace) = ask_cloto(other).await;
    nothing_changed_hands(&trace);
    assert!(told != gather && told != short && told != won, "{told}");
    assert_eq!(ask_cloto(leader).await.0, told);

    // Past the event's time nobody is sent out.
    sleep_until(deadline.unwrap() + Duration::from_secs(2)).await;
    for player in [&mut *leader, &mut *giver, &mut *other] {
        let mut trace = Trace::default();
        read_for(&mut player.connection, &mut trace, 300).await;
        assert!(trace.warps.is_empty(), "{trace:?}");
    }

    // Nor when the leader breaks up the party, then leaves.
    send(&mut leader.connection, leave_party()).await;
    for player in [&mut *giver, &mut *other] {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.party_ops.contains(&OP_LEAVE)
        })
        .await;
        read_for(&mut player.connection, &mut trace, 1000).await;
        assert!(trace.warps.is_empty(), "{trace:?}");
    }
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut leader.connection, KPQ.exit, "", &mut trace).await,
        KPQ.exit
    );
    let mut trace = Trace::default();
    read_for(&mut giver.connection, &mut trace, 1000).await;
    assert!(trace.warps.is_empty(), "{trace:?}");

    // The others are still in it, on its maps, with no clock.
    assert_eq!(ask_cloto(giver).await.0, told);
    let trace = go_to(other, KPQ.stage_map(4)).await;
    assert!(trace.countdowns.is_empty(), "{trace:?}");
}
