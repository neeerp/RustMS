//! The Amorian Challenge's second stage: five of the team hang on its
//! three ropes, as many on each as Amos wants. He tells the leader how
//! each try weighs, the map hears it in pink, and the seventh wrong try
//! in a row brings out monsters and draws the ropes anew; the right one
//! clears the stage and opens its way on. The harness world pins the ropes
//! to two, two and one (`RUSTMS_AMORIA_ROPES`).

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
    AMORIA, AMORIA_LOBBY, AMORIA_ROPES_STAGE, AMORIA_ROPE_MOBS, AMORIA_ROPE_MOBS_AT,
    AMORIA_ROPE_MOBS_EACH, AMORIA_ROPE_TRIES, AMORIA_STAGE2,
};
use net::npc_scripts::AMOS_STAGE;

const STAGE: i32 = AMORIA_STAGE2[0];
const STAGE3: i32 = 670_010_400;
const PINK: u8 = 5;
/// A point on each rope (Map.nx `area` 0-2), and one on the ground off
/// them.
const ROPES: [(i16, i16); 3] = [(-628, -1700), (-45, -1800), (540, -1950)];
const GROUND: (i16, i16) = (300, -845);

/// The members after the leader hang `on` each rope, in turn, and the
/// rest stand on the ground.
async fn hang(team: &mut [WorldSession], on: [usize; 3]) {
    let mut at = (0..3).flat_map(|rope| std::iter::repeat_n(ROPES[rope], on[rope]));
    for player in team[1..].iter_mut() {
        let (x, y) = at.next().unwrap_or(GROUND);
        told(player, walk_to(x, y)).await;
    }
}

/// The leader hears `key` from Amos, and closes it.
async fn hears(player: &mut WorldSession, key: &str) -> Trace {
    let t = told(player, talk(STAGE, AMOS_STAGE)).await;
    assert!(said(&t, key), "{t:?}");
    told(player, answer(0, 1, None)).await;
    t
}

/// A wrong try: Amos says `key`, everyone sees the wrong answer, and a
/// member hears `notice` in pink.
async fn missed(team: &mut [WorldSession], key: &str, notice: &str) {
    let t = hears(&mut team[0], key).await;
    assert!(wrong(&t), "{t:?}");
    let notice = net::content::text().get(notice, &[]);
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, |t| {
        t.broadcasts.contains(&(PINK, notice.clone())) && wrong(t)
    })
    .await;
}

/// `player` takes the stage's `next00` and is turned back.
async fn shut(player: &mut WorldSession) {
    let mut t = Trace::default();
    send(
        &mut player.connection,
        build_change_map(-1, "next00").unwrap(),
    )
    .await;
    until(&mut player.connection, &mut t, |t| !t.blocked.is_empty()).await;
    assert!(t.warps.is_empty(), "{t:?}");
}

// One test: a channel holds one run at a time.
#[tokio::test]
async fn five_on_the_ropes_as_amos_wants_open_the_way_on() {
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[]).await;
    party(&mut team).await;
    enter_amoria(&mut team).await;
    for player in team.iter_mut() {
        let mut t = Trace::default();
        assert_eq!(warp(&mut player.connection, STAGE, "", &mut t).await, STAGE);
    }
    assert_eq!(AMORIA.stage_on(STAGE), Some(AMORIA_ROPES_STAGE));

    // Amos sends the others to the leader, who first hears the task.
    hears(&mut team[1], keys::AMORIA_PQ_AMOS_NOT_LEADER).await;
    let t = hears(&mut team[0], keys::AMORIA_PQ_STAGE2_INTRO).await;
    assert!(!wrong(&t), "{t:?}");
    shut(&mut team[1]).await;

    // Four on the ropes is no try.
    hang(&mut team, [2, 2, 0]).await;
    let t = hears(&mut team[0], keys::AMORIA_PQ_STAGE2_FEW).await;
    assert!(!wrong(&t), "{t:?}");

    // None right, then one.
    hang(&mut team, [0, 0, 5]).await;
    let (differ, differ_notice) = (
        keys::AMORIA_PQ_STAGE2_ALL_DIFFER,
        keys::AMORIA_PQ_STAGE2_ALL_DIFFER_NOTICE,
    );
    missed(&mut team, differ, differ_notice).await;
    hang(&mut team, [2, 0, 3]).await;
    let (same, same_notice) = (
        keys::AMORIA_PQ_STAGE2_ONE_SAME,
        keys::AMORIA_PQ_STAGE2_ONE_SAME_NOTICE,
    );
    missed(&mut team, same, same_notice).await;
    hang(&mut team, [0, 0, 5]).await;
    for _ in 2..AMORIA_ROPE_TRIES - 1 {
        missed(&mut team, differ, differ_notice).await;
    }

    // The seventh brings out the monsters, two of each below each point.
    let mut seen = told(&mut team[0], talk(STAGE, AMOS_STAGE)).await;
    assert!(said(&seen, keys::AMORIA_PQ_STAGE2_RESET), "{seen:?}");
    assert!(wrong(&seen), "{seen:?}");
    let each = AMORIA_ROPE_MOBS_AT.len() * usize::from(AMORIA_ROPE_MOBS_EACH);
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        AMORIA_ROPE_MOBS
            .iter()
            .all(|&mob| t.field.alive(mob).len() == each)
    })
    .await;
    told(&mut team[0], answer(0, 1, None)).await;
    let notice = keys::AMORIA_PQ_STAGE2_RESET_NOTICE;
    let notice = net::content::text().get(notice, &[]);
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, |t| {
        t.broadcasts.contains(&(PINK, notice.clone()))
    })
    .await;

    // No try while they are about; the right one, once they are beaten,
    // clears the stage without a word.
    hang(&mut team, [2, 2, 1]).await;
    hears(&mut team[0], keys::AMORIA_PQ_STAGE2_MOBS).await;
    let c = &mut team[0].connection;
    for _ in 0..10 {
        let mobs: Vec<i32> = (AMORIA_ROPE_MOBS.iter())
            .flat_map(|&mob| seen.field.alive(mob))
            .collect();
        if mobs.is_empty() {
            break;
        }
        for &oid in &mobs {
            send(c, attack(oid, 30_000)).await;
        }
        read_for(c, &mut seen, 2_000).await;
    }
    assert!(
        (AMORIA_ROPE_MOBS.iter()).all(|&mob| seen.field.alive(mob).is_empty()),
        "{seen:?}"
    );
    send(&mut team[0].connection, talk(STAGE, AMOS_STAGE)).await;
    let mut t = Trace::default();
    until(&mut team[0].connection, &mut t, |t| {
        cleared(t) && !t.gains.is_empty()
    })
    .await;
    assert!(t.prompts.is_empty(), "{t:?}");
    assert_eq!(t.gains, [AMORIA.stage_exp(AMORIA_ROPES_STAGE)]);

    // The way on is open, and Amos says so.
    let mut t = Trace::default();
    let c = &mut team[1].connection;
    send(c, build_change_map(-1, "next00").unwrap()).await;
    until(c, &mut t, |t| t.warps.contains(&STAGE3)).await;
    hears(&mut team[0], keys::AMORIA_PQ_AMOS_CLEARED).await;
}
