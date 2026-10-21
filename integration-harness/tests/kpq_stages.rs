//! Kerning City's party quest, stages 2 to 4: on each, Cloto sets the
//! leader a puzzle, three of the map's areas (ropes, platforms, barrels) to
//! be taken one member to each. Asked while the team stands wrong, she says
//! so and everyone on the map sees and hears it; asked while it stands
//! right, she clears the stage. The harness world pins the puzzle
//! (`RUSTMS_KPQ_COMBO`) to the second set of three areas: 0, 1 and 3.

use integration_harness::events::{enter_kpq, in_lobby, maps, party, talk, CLOTO};
use integration_harness::hunting::walk_to;
use integration_harness::packets::build_change_map;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{combos, KPQ};
use net::packet::op::SendOpcode;

/// The puzzle the harness world pins, counted from 0.
const COMBO: usize = 1;

/// What `player` is shown on talking to Cloto on `map`: one text page, and
/// the trace that came with it.
async fn ask_cloto(player: &mut WorldSession, map: i32) -> (String, Trace) {
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, talk(map, CLOTO)).await;
    assert_eq!(trace.prompts, [(CLOTO, 0)], "{trace:?}");
    (trace.prompt_texts[0].clone(), trace)
}

/// The middle of each of `map`'s areas.
fn area_centres(map: i32) -> Vec<(i16, i16)> {
    maps()
        .field(map)
        .unwrap()
        .areas
        .iter()
        .map(|a| (((a.x1 + a.x2) / 2) as i16, ((a.y1 + a.y2) / 2) as i16))
        .collect()
}

/// Move each player to the spot beside them, and wait until the field
/// has it.
async fn stand_on(team: &mut [WorldSession], spots: &[(i16, i16)]) {
    for (player, &(x, y)) in team.iter_mut().zip(spots) {
        let name = player.character_name.clone();
        step(&mut player.connection, &name, walk_to(x, y)).await;
    }
}

/// What a player on the map is shown of a wrong answer: the X, and its
/// sound.
fn wrong_effects() -> Vec<(u8, String)> {
    vec![
        (3, "quest/party/wrong_kor".into()),
        (4, "Party1/Failed".into()),
    ]
}

fn clear_effects() -> Vec<(u8, String)> {
    vec![
        (3, "quest/party/clear".into()),
        (4, "Party1/Clear".into()),
        (2, "gate".into()),
    ]
}

#[tokio::test]
async fn the_team_takes_cloto_s_areas_to_clear_stages_two_to_four() {
    let mut team = vec![
        in_lobby(25, &[]).await,
        in_lobby(25, &[]).await,
        in_lobby(25, &[]).await,
    ];
    party(&mut team).await;
    enter_kpq(&mut team).await;

    // Past stage 1 by debug warp, which no gate checks.
    for player in team.iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut player.connection, KPQ.stage_map(2), "", &mut trace).await,
            KPQ.stage_map(2)
        );
    }

    for stage in 2..=4u8 {
        let map = KPQ.stage_map(stage);
        let centres = area_centres(map);
        let combo = combos(centres.len()).nth(COMBO).unwrap();
        assert_eq!(combo, 0b1011, "areas 0, 1 and 3");

        // The way on is shut.
        send(
            &mut team[0].connection,
            build_change_map(-1, "next00").unwrap(),
        )
        .await;
        let mut trace = Trace::default();
        until(&mut team[0].connection, &mut trace, |t| {
            !t.blocked.is_empty()
        })
        .await;
        assert_eq!(trace.blocked, [(SendOpcode::BlockedMap as i16, 1)]);

        // A member is told to follow the leader, who is told the puzzle.
        let (follow, _) = ask_cloto(&mut team[1], map).await;
        let (intro, _) = ask_cloto(&mut team[0], map).await;
        assert_ne!(follow, intro);
        assert_eq!(ask_cloto(&mut team[2], map).await.0, follow);

        // Areas 0, 1 and 2 are wrong: everyone on the map is shown so, and
        // nothing is cleared.
        stand_on(&mut team, &[centres[0], centres[1], centres[2]]).await;
        let (wrong, trace) = ask_cloto(&mut team[0], map).await;
        assert_ne!(wrong, intro);
        assert_eq!(trace.field_effects, wrong_effects());
        assert!(trace.gains.is_empty(), "{trace:?}");
        for member in &mut team[1..] {
            let mut trace = Trace::default();
            until(&mut member.connection, &mut trace, |t| {
                t.field_effects.len() >= 2
            })
            .await;
            assert_eq!(trace.field_effects, wrong_effects());
        }

        // Areas 0, 1 and 3 are right, but only the leader may answer.
        stand_on(&mut team[2..], &[centres[3]]).await;
        let (told, trace) = ask_cloto(&mut team[1], map).await;
        assert_eq!(told, follow);
        assert!(trace.field_effects.is_empty(), "{trace:?}");

        // The leader's answer clears the stage, and everyone gets its EXP.
        let (right, mut trace) = ask_cloto(&mut team[0], map).await;
        assert!(right != wrong && right != intro, "{right}");
        until(&mut team[0].connection, &mut trace, |t| !t.gains.is_empty()).await;
        assert_eq!(trace.field_effects, clear_effects());
        assert_eq!(trace.gains, [KPQ.stage_exp(stage)]);
        for member in &mut team[1..] {
            let mut trace = Trace::default();
            until(&mut member.connection, &mut trace, |t| !t.gains.is_empty()).await;
            assert_eq!(trace.field_effects, clear_effects());
            assert_eq!(trace.gains, [KPQ.stage_exp(stage)]);
        }

        // Asked again, Cloto says the stage is done, and the gate is open.
        let (done, trace) = ask_cloto(&mut team[1], map).await;
        assert!(done != follow && done != right, "{done}");
        assert!(trace.gains.is_empty(), "{trace:?}");
        for player in team.iter_mut() {
            let mut trace = Trace::default();
            assert_eq!(
                warp(&mut player.connection, -1, "next00", &mut trace).await,
                KPQ.stage_map(stage + 1)
            );
        }
    }
}
