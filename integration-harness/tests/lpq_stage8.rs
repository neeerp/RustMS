//! Ludibrium's party quest, stage 8: the Blue Balloon sets the leader five
//! of the map's nine crates, to be taken one member to each. Asked with
//! other than five on the crates, it asks for five; asked while five stand
//! wrong, it says so and everyone on the map sees and hears it; asked while
//! they stand right, it clears the stage. The harness world pins the
//! puzzle (`RUSTMS_KPQ_COMBO`) to the second set of five crates.

use game_data::server_content::keys;
use integration_harness::events::{enter_lpq, maps, party, talk, waiting_in};
use integration_harness::hunting::walk_to;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{combos, LPQ};
use net::npc_scripts::BLUE_BALLOON;

/// The puzzle the harness world pins, counted from 0.
const COMBO: usize = 1;
const STAGE: u8 = 8;

/// What `player` is shown on talking to the balloon: one text page, and
/// the trace that came with it.
async fn ask_balloon(player: &mut WorldSession) -> (String, Trace) {
    let name = player.character_name.clone();
    let packet = talk(LPQ.stage_map(STAGE), BLUE_BALLOON);
    let trace = step(&mut player.connection, &name, packet).await;
    assert_eq!(trace.prompts, [(BLUE_BALLOON, 0)], "{trace:?}");
    (trace.prompt_texts[0].clone(), trace)
}

/// The middle of each of the crates `combo` takes.
fn crates(combo: u32) -> Vec<(i16, i16)> {
    let field = maps().field(LPQ.stage_map(STAGE)).unwrap();
    (field.areas.iter().enumerate())
        .filter(|&(i, _)| combo >> i & 1 == 1)
        .map(|(_, a)| (((a.x1 + a.x2) / 2) as i16, ((a.y1 + a.y2) / 2) as i16))
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

fn says(said: &str, key: &str) {
    let line = net::content::text().get(key, &[]);
    assert!(said.contains(&line), "{said}");
}

#[tokio::test]
async fn the_team_takes_the_blue_balloon_s_five_crates_to_clear_stage_8() {
    let mut a = Vec::new();
    for _ in 0..5 {
        a.push(waiting_in(LPQ.lobby, 35, &[]).await);
    }
    party(&mut a).await;
    enter_lpq(&mut a).await;
    let map = LPQ.stage_map(STAGE);
    for player in a.iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    }

    // A member is sent to fetch the leader; the leader first hears what
    // the stage asks.
    says(
        &ask_balloon(&mut a[1]).await.0,
        keys::LPQ_BALLOON_NOT_LEADER,
    );
    says(&ask_balloon(&mut a[0]).await.0, keys::LPQ_BALLOON_INTRO_8);

    let areas = maps().field(map).unwrap().areas.len();
    let all: Vec<u32> = combos(areas, LPQ.areas_taken).collect();
    let (wrong, right) = (crates(all[0]), crates(all[COMBO]));
    assert_eq!((areas, right.len()), (9, 5));

    // Three on the crates aren't enough: the balloon asks for five, and
    // shows nothing.
    stand_on(&mut a[..3], &right).await;
    let (said, trace) = ask_balloon(&mut a[0]).await;
    says(&said, keys::LPQ_BALLOON_FEW);
    assert!(trace.field_effects.is_empty(), "{trace:?}");

    // Five on the wrong crates: everyone on the map sees and hears it.
    stand_on(&mut a, &wrong).await;
    let (said, trace) = ask_balloon(&mut a[0]).await;
    says(&said, keys::LPQ_BALLOON_WRONG);
    let wrong_effects = [
        (3, "quest/party/wrong_kor".to_string()),
        (4, "Party1/Failed".to_string()),
    ];
    assert_eq!(trace.field_effects, wrong_effects);
    let mut trace = Trace::default();
    until(&mut a[4].connection, &mut trace, |t| {
        t.field_effects.len() == 2
    })
    .await;
    assert_eq!(trace.field_effects, wrong_effects);

    // Five on the right ones clear it for everyone.
    stand_on(&mut a, &right).await;
    let (said, trace) = ask_balloon(&mut a[0]).await;
    says(&said, keys::LPQ_BALLOON_DONE);
    let clear_effects = [
        (3, "quest/party/clear".to_string()),
        (4, "Party1/Clear".to_string()),
        (2, "gate".to_string()),
    ];
    let mut seen = Some(trace);
    for player in a.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| !t.gains.is_empty()).await;
        assert!(trace.field_effects.ends_with(&clear_effects), "{trace:?}");
        assert_eq!(trace.gains, [LPQ.stage_exp(STAGE)]);
    }
}
