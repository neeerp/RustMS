//! The Amorian Challenge's last stage: the Geist Balrog waits there from
//! the start, and comes back twice as it dies; only its last form drops
//! the Geist Fang. Amos takes anyone below up to it, sends the others to
//! the leader, and the leader's Fang wins the challenge; the others then
//! wait on the leader (`amoria_bonus`).

use game_data::server_content::keys;
use integration_harness::events::{cleared, couples_in, enter_amoria, party, talk, told};
use integration_harness::hunting::{attack, pickup, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    AMORIA, AMORIA_GEIST_STAGE, AMORIA_LOBBY, AMORIA_STAGE6, GEIST_BALROG, GEIST_FANG,
};
use net::npc_scripts::AMOS_TRIALS;

/// The Geist Balrog's three forms, each coming out of the one before.
const FORMS: [i32; 3] = [GEIST_BALROG, 9_400_537, 9_400_514];
/// A point in the map's first area, below Amos, and the id of the portal
/// he takes them up to, `st01`.
const BELOW: (i16, i16) = (-260, 51);
const UP: u8 = 18;
/// A hit within the cap of `couples_in`'s stats.
const HIT: i32 = 30_000;

/// `player` hears `key` from Amos, with the Fang for its item, and closes
/// it.
async fn hears(player: &mut WorldSession, key: &str) -> Trace {
    let fang = GEIST_FANG.to_string();
    let text = net::content::text().get(key, &[("item", &fang)]);
    let t = told(player, talk(AMORIA_STAGE6, AMOS_TRIALS)).await;
    assert!(t.prompt_texts.iter().any(|s| s.contains(&text)), "{t:?}");
    told(player, answer(0, 1, None)).await;
    t
}

// One test: a channel holds one run at a time.
#[tokio::test]
async fn the_geist_balrog_s_fang_wins_the_amorian_challenge() {
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[]).await;
    party(&mut team).await;
    enter_amoria(&mut team).await;
    let mut seen = Trace::default();
    for (n, player) in team.iter_mut().enumerate() {
        let mut t = Trace::default();
        let to = warp(&mut player.connection, AMORIA_STAGE6, "", &mut t).await;
        assert_eq!(to, AMORIA_STAGE6);
        if n == 0 {
            seen = t;
        }
    }
    assert_eq!(AMORIA.stage_on(AMORIA_STAGE6), Some(AMORIA_GEIST_STAGE));
    // The boss is there from the start.
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| !t.field.alive(GEIST_BALROG).is_empty()).await;

    // Amos takes anyone below up, without a word.
    told(&mut team[1], walk_to(BELOW.0, BELOW.1)).await;
    let c = &mut team[1].connection;
    send(c, talk(AMORIA_STAGE6, AMOS_TRIALS)).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| {
        t.arrivals.iter().any(|a| (a.0, a.1) == (AMORIA_STAGE6, UP))
    })
    .await;
    assert!(t.prompts.is_empty(), "{t:?}");

    // Up there, he sends the others to the leader, who must bring the
    // Fang.
    hears(&mut team[1], keys::AMORIA_PQ_AMOS_NOT_LEADER).await;
    let t = hears(&mut team[0], keys::AMORIA_PQ_STAGE6_FANG).await;
    assert!(t.removed.is_empty() && !cleared(&t), "{t:?}");

    // Each form comes out of the last; the last drops the Fang.
    let c = &mut team[0].connection;
    for _ in 0..40 {
        if seen.field.item_drop(GEIST_FANG).is_some() {
            break;
        }
        let alive: Vec<i32> = (FORMS.iter())
            .flat_map(|&mob| seen.field.alive(mob))
            .collect();
        for oid in alive {
            for _ in 0..10 {
                send(c, attack(oid, HIT)).await;
            }
        }
        read_for(c, &mut seen, 1_000).await;
    }
    for form in FORMS {
        assert!(
            seen.field.mobs.iter().any(|&(_, mob)| mob == form),
            "{form} {seen:?}"
        );
    }
    let fang = seen.field.item_drop(GEIST_FANG).unwrap();
    send(c, walk_to(fang.x, fang.y)).await;
    send(c, pickup(fang.object_id, fang.x, fang.y)).await;
    until(c, &mut seen, |t| t.added.iter().any(|a| a.2 == GEIST_FANG)).await;

    // The leader's Fang wins it.
    let t = hears(&mut team[0], keys::AMORIA_PQ_STAGE6_DONE).await;
    assert_eq!(t.removed, [(4, 1)], "{t:?}");
    // The others' shares of the kills come first.
    let exp = AMORIA.stage_exp(AMORIA_GEIST_STAGE);
    let mut t = Trace::default();
    until(&mut team[1].connection, &mut t, |t| {
        cleared(t) && t.gains.last() == Some(&exp)
    })
    .await;
    hears(&mut team[1], keys::AMORIA_PQ_STAGE6_WAIT).await;
}
