//! The Amorian Challenge's third stage: five of the team stand one on each
//! of the five of its nine platforms Amos wants. A wrong try draws no word
//! from him, only a slime for each platform they got right; the right one
//! clears the stage and opens its way on. The harness world pins the draw
//! (`RUSTMS_KPQ_COMBO`, `"1"`: the second way of taking five of nine).

use game_data::server_content::keys;
use integration_harness::events::{
    cleared, couples_in, enter_amoria, party, said, talk, told, wrong,
};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::packets::build_change_map;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    combos, AMORIA, AMORIA_LOBBY, AMORIA_PLATFORMS_STAGE, AMORIA_SLIMES, AMORIA_STAGE3,
};
use net::npc_scripts::AMOS_STAGE;

const STAGE4: i32 = 670_010_500;
/// A point on each platform (Map.nx `area` 0-8), and one on the ground off
/// them.
const PLATFORMS: [(i16, i16); 9] = [
    (1465, 48),
    (1631, 49),
    (1792, 49),
    (1952, 48),
    (1378, 201),
    (1543, 201),
    (1708, 202),
    (1871, 200),
    (2030, 201),
];
const GROUND: (i16, i16) = (742, 122);

/// The members after the leader stand on `on`, in turn; `None` for the
/// ground.
async fn stand(team: &mut [WorldSession], on: [Option<usize>; 5]) {
    for (player, on) in team[1..].iter_mut().zip(on) {
        let (x, y) = on.map_or(GROUND, |platform| PLATFORMS[platform]);
        told(player, walk_to(x, y)).await;
    }
}

/// The leader hears `key` from Amos, and closes it.
async fn hears(player: &mut WorldSession, key: &str) -> Trace {
    let t = told(player, talk(AMORIA_STAGE3, AMOS_STAGE)).await;
    assert!(said(&t, key), "{t:?}");
    told(player, answer(0, 1, None)).await;
    t
}

/// The slimes alive in `t`.
fn slimes(t: &Trace) -> Vec<i32> {
    (AMORIA_SLIMES.iter())
        .flat_map(|&mob| t.field.alive(mob))
        .collect()
}

/// A wrong try: Amos says nothing, everyone sees the wrong answer, and
/// `right` slimes come out. Returns the leader's trace.
async fn missed(team: &mut [WorldSession], right: usize) -> Trace {
    send(&mut team[0].connection, talk(AMORIA_STAGE3, AMOS_STAGE)).await;
    let mut seen = Trace::default();
    until(&mut team[0].connection, &mut seen, |t| {
        wrong(t) && slimes(t).len() == right
    })
    .await;
    read_for(&mut team[0].connection, &mut seen, 1_000).await;
    assert_eq!(slimes(&seen).len(), right, "{seen:?}");
    assert!(seen.prompts.is_empty(), "{seen:?}");
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, wrong).await;
    seen
}

// One test: a channel holds one run at a time.
#[tokio::test]
async fn one_on_each_platform_amos_wants_opens_the_way_on() {
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[]).await;
    party(&mut team).await;
    enter_amoria(&mut team).await;
    for player in team.iter_mut() {
        let mut t = Trace::default();
        let to = warp(&mut player.connection, AMORIA_STAGE3, "", &mut t).await;
        assert_eq!(to, AMORIA_STAGE3);
    }
    assert_eq!(AMORIA.stage_on(AMORIA_STAGE3), Some(AMORIA_PLATFORMS_STAGE));
    // The pinned draw: platforms 0-3 and 5.
    assert_eq!(combos(9, 5).nth(1), Some(0b10_1111));

    // Amos sends the others to the leader, who first hears the task.
    hears(&mut team[1], keys::AMORIA_PQ_AMOS_NOT_LEADER).await;
    let t = hears(&mut team[0], keys::AMORIA_PQ_STAGE3_INTRO).await;
    assert!(!wrong(&t), "{t:?}");
    let mut t = Trace::default();
    let c = &mut team[1].connection;
    send(c, build_change_map(-1, "next00").unwrap()).await;
    until(c, &mut t, |t| !t.blocked.is_empty()).await;
    assert!(t.warps.is_empty(), "{t:?}");

    // Four on the platforms is no try.
    stand(&mut team, [Some(0), Some(1), Some(2), Some(3), None]).await;
    let t = hears(&mut team[0], keys::AMORIA_PQ_STAGE3_FEW).await;
    assert!(!wrong(&t), "{t:?}");

    // Four right: four slimes, which must be beaten before the next try.
    stand(&mut team, [Some(0), Some(1), Some(2), Some(3), Some(4)]).await;
    let mut seen = missed(&mut team, 4).await;
    hears(&mut team[0], keys::AMORIA_PQ_STAGE2_MOBS).await;
    let c = &mut team[0].connection;
    for _ in 0..10 {
        let mobs = slimes(&seen);
        if mobs.is_empty() {
            break;
        }
        for &oid in &mobs {
            send(c, attack(oid, 30_000)).await;
        }
        read_for(c, &mut seen, 2_000).await;
    }
    assert!(slimes(&seen).is_empty(), "{seen:?}");

    // Two on one right platform, and none on the others: no slime.
    stand(&mut team, [Some(0), Some(0), Some(6), Some(7), Some(8)]).await;
    missed(&mut team, 0).await;

    // The right try clears the stage without a word.
    stand(&mut team, [Some(5), Some(3), Some(2), Some(1), Some(0)]).await;
    send(&mut team[0].connection, talk(AMORIA_STAGE3, AMOS_STAGE)).await;
    let mut t = Trace::default();
    until(&mut team[0].connection, &mut t, |t| {
        cleared(t) && !t.gains.is_empty()
    })
    .await;
    assert!(t.prompts.is_empty(), "{t:?}");
    assert!(!wrong(&t), "{t:?}");
    assert_eq!(t.gains, [AMORIA.stage_exp(AMORIA_PLATFORMS_STAGE)]);

    // The way on is open, and Amos says so.
    let mut t = Trace::default();
    let c = &mut team[1].connection;
    send(c, build_change_map(-1, "next00").unwrap()).await;
    until(c, &mut t, |t| t.warps.contains(&STAGE4)).await;
    hears(&mut team[0], keys::AMORIA_PQ_AMOS_CLEARED).await;
}
