//! The Amorian Challenge's bonus. Once the Geist Fang has won it, the
//! leader's talk with Amos takes a team of couples to their own bonus for
//! two minutes, where each box hit open brings out seven monsters. There
//! the stage's portals, or the leader's next talk with Amos, lead on to the
//! bonus for a minute, where the other Amos only hurries them, and when
//! that time is up the team leaves. The harness world runs events 60 times
//! as fast (`RUSTMS_EVENT_SCALE`), so these last 2 seconds and 1, and
//! their clocks show at most that.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::events::{cleared, couples_in, enter_amoria, party, said, talk, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::hunting::{hit_reactor, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use net::events::{
    AMORIA_BONUS, AMORIA_BOXES, AMORIA_BOX_MOBS, AMORIA_BOX_MOB_COUNT, AMORIA_COUPLES_BONUS,
    AMORIA_EXIT, AMORIA_LOBBY, AMORIA_STAGE6, GEIST_FANG,
};
use net::npc_scripts::{AMOS_BONUS, AMOS_TRIALS};

/// The box's states before the hit that breaks it.
const BOX_STATES: u8 = 3;
/// One of the couples' bonus's portals on, `up00`.
const WAY_ON: &str = "up00";

/// The trace of arriving on `map`, and the clock then shown (the stage
/// clear shows the old one first).
async fn arrives(c: &mut MapleTestConnection, map: i32) -> (Trace, i32) {
    let mut t = Trace::default();
    until(c, &mut t, |t| t.warps.contains(&map)).await;
    let shown = t.countdowns.len();
    until(c, &mut t, |t| t.countdowns.len() > shown).await;
    let clock = *t.countdowns.last().unwrap();
    (t, clock)
}

// One test: a channel holds one run at a time.
#[tokio::test]
async fn the_won_challenge_s_couples_play_both_bonuses_and_leave() {
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[(GEIST_FANG, 1)]).await;
    party(&mut team).await;
    enter_amoria(&mut team).await;
    for player in team.iter_mut() {
        let mut t = Trace::default();
        let to = warp(&mut player.connection, AMORIA_STAGE6, "", &mut t).await;
        assert_eq!(to, AMORIA_STAGE6);
    }

    // The leader's Fang wins it; the others wait on the leader.
    let t = told(&mut team[0], talk(AMORIA_STAGE6, AMOS_TRIALS)).await;
    let fang = GEIST_FANG.to_string();
    let done = net::content::text().get(keys::AMORIA_PQ_STAGE6_DONE, &[("item", &fang)]);
    assert!(t.prompt_texts.iter().any(|s| s.contains(&done)), "{t:?}");
    told(&mut team[0], answer(0, 1, None)).await;
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, cleared).await;
    let t = told(&mut team[1], talk(AMORIA_STAGE6, AMOS_TRIALS)).await;
    assert!(said(&t, keys::AMORIA_PQ_STAGE6_WAIT), "{t:?}");
    told(&mut team[1], answer(0, 1, None)).await;

    // The leader's next talk takes the couples to their bonus, on its clock.
    send(&mut team[0].connection, talk(AMORIA_STAGE6, AMOS_TRIALS)).await;
    let mut seen = Trace::default();
    for (n, player) in team.iter_mut().enumerate() {
        let (t, clock) = arrives(&mut player.connection, AMORIA_COUPLES_BONUS).await;
        assert!(t.prompts.is_empty() && clock <= 2, "{t:?}");
        if n == 0 {
            seen = t;
        }
    }

    // A box hit open brings out seven monsters.
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        t.field
            .reactors
            .iter()
            .any(|r| AMORIA_BOXES.contains(&r.reactor_id))
    })
    .await;
    let boxed = *(seen.field.reactors.iter())
        .find(|r| AMORIA_BOXES.contains(&r.reactor_id))
        .unwrap();
    send(c, walk_to(boxed.x, boxed.y)).await;
    for state in 1..=BOX_STATES {
        send(c, hit_reactor(boxed.object_id)).await;
        until(c, &mut seen, |t| {
            t.field.reactor_hits.contains(&(boxed.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(boxed.object_id)).await;
    let out = |t: &Trace| {
        (t.field.mobs.iter())
            .filter(|&&(_, mob)| AMORIA_BOX_MOBS.contains(&mob))
            .count()
    };
    until(c, &mut seen, |t| out(t) == AMORIA_BOX_MOB_COUNT).await;

    // A portal there takes one on, and the leader's talk the rest.
    touch_script_portal(&mut team[1], AMORIA_COUPLES_BONUS, WAY_ON).await;
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, |t| {
        t.warps.contains(&AMORIA_BONUS)
    })
    .await;
    let c = &mut team[0].connection;
    send(c, talk(AMORIA_COUPLES_BONUS, AMOS_TRIALS)).await;
    let (t, clock) = arrives(c, AMORIA_BONUS).await;
    assert!(t.prompts.is_empty() && clock <= 1, "{t:?}");

    // The other Amos only hurries them, and the time up sees them out.
    let mut t = told(&mut team[0], talk(AMORIA_BONUS, AMOS_BONUS)).await;
    assert!(said(&t, keys::AMORIA_PQ_BONUS_HURRY), "{t:?}");
    // The leader's line may already have seen the time up.
    let c = &mut team[0].connection;
    until(c, &mut t, |t| t.warps.contains(&AMORIA_EXIT)).await;
    for player in team[1..].iter_mut() {
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            t.warps.contains(&AMORIA_EXIT)
        })
        .await;
    }
}
