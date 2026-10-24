//! The Ludibrium Maze, its first slice: Rolly in Ludibrium takes those of a
//! party of three to six at levels 51 to 70 who are there into its own copy
//! of the maze, each on a room drawn for them; its rooms' portals lead from
//! room to room; and when the time runs out everyone lands on the way out,
//! where the other Rolly sends them back to Ludibrium. The harness world
//! runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the maze lasts
//! 15 seconds.

use game_data::server_content::keys;
use integration_harness::events::{maps, party, talk, told, waiting_in};
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use net::events::{LUDIBRIUM, LUDIMAZE, MAZE_EXIT, MAZE_GOAL, MAZE_ROOM};
use net::npc_scripts::ROLLY;
use tokio::time::Duration;

/// Rolly on the maze's way out.
const ROLLY_HOME: i32 = 9103003;

/// `key`, with who may play.
fn terms(key: &str) -> String {
    let (team, levels) = (&LUDIMAZE.team, &LUDIMAZE.levels);
    net::content::text().get(
        key,
        &[
            ("least", &team.start().to_string()),
            ("most", &team.end().to_string()),
            ("low", &levels.start().to_string()),
            ("high", &levels.end().to_string()),
        ],
    )
}

#[tokio::test]
async fn rolly_takes_three_in_each_to_a_room_of_their_own_and_home_when_the_time_runs_out() {
    let mut team = Vec::new();
    for _ in 0..3 {
        team.push(waiting_in(LUDIBRIUM, 51, &[]).await);
    }
    party(&mut team).await;

    // Only the leader may take the party in, and only with three there.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut team[2].connection, 221_000_000, "", &mut trace).await,
        221_000_000
    );
    assert_eq!(
        told(&mut team[1], talk(LUDIBRIUM, ROLLY)).await.prompts,
        [(ROLLY, 4)]
    );
    let trace = told(&mut team[1], answer(4, 1, Some(0))).await;
    let not_leader = net::content::text().get(keys::LUDIMAZE_NOT_LEADER, &[]);
    assert!(trace.prompt_texts[0].contains(&not_leader), "{trace:?}");
    told(&mut team[0], talk(LUDIBRIUM, ROLLY)).await;
    let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
    let not_eligible = terms(keys::LUDIMAZE_NOT_ELIGIBLE);
    assert!(trace.prompt_texts[0].contains(&not_eligible), "{trace:?}");

    // With the third back, the three go in, once any party before them is
    // out, each on one of the first 15 rooms, on the clock.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut team[2].connection, LUDIBRIUM, "", &mut trace).await,
        LUDIBRIUM
    );
    let in_room = |map: &i32| (MAZE_ROOM..MAZE_GOAL).contains(map);
    let mut seen = None;
    for _ in 0..20 {
        told(&mut team[0], talk(LUDIBRIUM, ROLLY)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == ROLLY) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Rolly never let the party in");
    let mut rooms = Vec::new();
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.iter().any(in_room) && !t.countdowns.is_empty()
        })
        .await;
        assert!((1..=15).contains(&trace.countdowns[0]), "{trace:?}");
        rooms.push(*trace.warps.iter().find(|map| in_room(map)).unwrap());
    }

    // A room's portals lead on to the next room.
    let field = maps().field(rooms[0]).unwrap();
    let portal = field
        .portals()
        .find(|portal| {
            portal
                .to_map
                .is_some_and(|to| in_room(&to) || to == MAZE_GOAL)
        })
        .unwrap();
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut team[0].connection, -1, &portal.name, &mut trace).await,
        portal.to_map.unwrap()
    );

    // The time runs out: everyone lands on the way out, and Rolly there
    // sends them back to Ludibrium.
    for player in team.iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&MAZE_EXIT)
        })
        .await;
        let trace = told(player, talk(MAZE_EXIT, ROLLY_HOME)).await;
        assert_eq!(trace.prompts, [(ROLLY_HOME, 0)]);
        assert_eq!(told(player, answer(0, 1, None)).await.warps, [LUDIBRIUM]);
    }
}
