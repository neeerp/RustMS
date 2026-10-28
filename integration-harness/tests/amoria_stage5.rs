//! The Amorian Challenge's fifth stage: the team hits its seven gates
//! open, each of whose doors then lets them through, and gathers at Amos
//! on the far side. With the team apart he waits; with a gate still shut
//! he will not take a shortcut; with every gate open the leader's talk
//! clears the stage, the team hears in pink that only the boss is left,
//! and the next talk takes the whole team on to stage 6.

use game_data::server_content::keys;
use integration_harness::events::{cleared, couples_in, enter_amoria, party, said, talk, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::hunting::{hit_reactor, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    AMORIA, AMORIA_GATE, AMORIA_GATES_STAGE, AMORIA_GATE_OPEN, AMORIA_LOBBY, AMORIA_STAGE5,
    AMORIA_STAGE6,
};
use net::npc_scripts::AMOS_TRIALS;

const PINK: u8 = 5;
/// Where Map.nx puts `gate00` to `gate06`.
const GATES: [(i16, i16); 7] = [
    (-8189, 436),
    (-6387, 434),
    (-4590, 434),
    (-901, 434),
    (3596, 435),
    (6300, 435),
    (7198, 436),
];
/// The first door, `gt00PCS`, and the portal past its gate, `gt00PIB`.
const DOOR: &str = "gt00PCS";
const PAST_DOOR: u8 = 2;
/// A point in the stage's last area, by Amos.
const AT_AMOS: (i16, i16) = (8800, 60);

/// The leader hears `key` from Amos, and closes it.
async fn hears(player: &mut WorldSession, key: &str) -> Trace {
    let t = told(player, talk(AMORIA_STAGE5, AMOS_TRIALS)).await;
    assert!(said(&t, key), "{t:?}");
    told(player, answer(0, 1, None)).await;
    t
}

/// The leader, who has seen the gates in `seen`, hits gate `n` open.
async fn open(leader: &mut WorldSession, seen: &mut Trace, n: usize) {
    let (x, y) = GATES[n];
    let gate = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == AMORIA_GATE && (r.x, r.y) == (x, y))
        .unwrap();
    let c = &mut leader.connection;
    send(c, walk_to(x - 60, y)).await;
    for state in 1..=AMORIA_GATE_OPEN {
        send(c, hit_reactor(gate.object_id)).await;
        until(c, seen, |t| {
            t.field.reactor_hits.contains(&(gate.object_id, state))
        })
        .await;
    }
}

/// Everyone in `team` walks over to Amos.
async fn gather(team: &mut [WorldSession]) {
    for player in team.iter_mut() {
        told(player, walk_to(AT_AMOS.0, AT_AMOS.1)).await;
    }
}

// One test: a channel holds one run at a time.
#[tokio::test]
async fn the_whole_team_past_every_gate_at_amos_clears_the_stage() {
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[]).await;
    party(&mut team).await;
    enter_amoria(&mut team).await;
    let mut seen = Trace::default();
    for (n, player) in team.iter_mut().enumerate() {
        let mut t = Trace::default();
        let to = warp(&mut player.connection, AMORIA_STAGE5, "", &mut t).await;
        assert_eq!(to, AMORIA_STAGE5);
        if n == 0 {
            seen = t;
        }
    }
    assert_eq!(AMORIA.stage_on(AMORIA_STAGE5), Some(AMORIA_GATES_STAGE));
    until(&mut team[0].connection, &mut seen, |t| {
        let gates = t.field.reactors.iter();
        gates.filter(|r| r.reactor_id == AMORIA_GATE).count() == GATES.len()
    })
    .await;
    let text = net::content::text();

    // Amos sends the others to the leader, who first hears the task, and
    // then is told the team is not all there.
    hears(&mut team[1], keys::AMORIA_PQ_AMOS_NOT_LEADER).await;
    hears(&mut team[0], keys::AMORIA_PQ_STAGE5_INTRO).await;
    hears(&mut team[0], keys::AMORIA_PQ_STAGE5_APART).await;

    // A shut gate's door turns them back.
    let shut = text.get(keys::AMORIA_PQ_STAGE5_GATE_SHUT, &[]);
    touch_script_portal(&mut team[1], AMORIA_STAGE5, DOOR).await;
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, |t| {
        t.broadcasts.contains(&(PINK, shut.clone()))
    })
    .await;
    assert!(t.arrivals.is_empty(), "{t:?}");

    // Once open, it lets them through.
    for n in 0..GATES.len() - 1 {
        open(&mut team[0], &mut seen, n).await;
    }
    touch_script_portal(&mut team[1], AMORIA_STAGE5, DOOR).await;
    until(&mut team[1].connection, &mut t, |t| {
        t.arrivals
            .iter()
            .any(|a| (a.0, a.1) == (AMORIA_STAGE5, PAST_DOOR))
    })
    .await;

    // All at Amos, but with the last gate shut, as if by the shortcut.
    gather(&mut team).await;
    let t = hears(&mut team[0], keys::AMORIA_PQ_STAGE5_TELEPORTED).await;
    assert!(!cleared(&t) && t.gains.is_empty(), "{t:?}");

    // With every gate open the stage clears.
    open(&mut team[0], &mut seen, GATES.len() - 1).await;
    gather(&mut team[..1]).await;
    hears(&mut team[0], keys::AMORIA_PQ_STAGE5_DONE).await;
    let notice = text.get(keys::AMORIA_PQ_STAGE5_NOTICE, &[]);
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, |t| {
        cleared(t) && !t.gains.is_empty() && t.broadcasts.contains(&(PINK, notice.clone()))
    })
    .await;
    assert_eq!(t.gains, [AMORIA.stage_exp(AMORIA_GATES_STAGE)]);

    // Any member's next talk takes the whole team on, without a word.
    let c = &mut team[1].connection;
    send(c, talk(AMORIA_STAGE5, AMOS_TRIALS)).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| t.warps.contains(&AMORIA_STAGE6)).await;
    assert!(t.prompts.is_empty(), "{t:?}");
    let mut t = Trace::default();
    let c = &mut team[0].connection;
    until(c, &mut t, |t| t.warps.contains(&AMORIA_STAGE6)).await;
}
