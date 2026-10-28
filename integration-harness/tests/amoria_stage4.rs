//! The Amorian Challenge's fourth stage: its monsters each drop a Cupid
//! Code Piece, and the leader, having heard Amos out, hands him 50. That
//! clears the stage and opens its way on, and with five minutes or more
//! left the clock is cut to four; the team hears in pink that time is
//! short. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the cut clock shows 4 seconds and the time
//! then runs out at once.

use game_data::server_content::keys;
use integration_harness::events::{cleared, couples_in, enter_amoria, party, talk, told};
use integration_harness::hunting::{attack, pickup};
use integration_harness::packets::build_change_map;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    AMORIA, AMORIA_CODES, AMORIA_CODES_STAGE, AMORIA_EXIT, AMORIA_LOBBY, AMORIA_RUSH_MINUTES,
    AMORIA_STAGE4, CUPID_CODE_PIECE,
};
use net::npc_scripts::AMOS_TRIALS;

const STAGE5: i32 = 670_010_600;
const PINK: u8 = 5;
const MOBS: [i32; 3] = [9_400_524, 9_400_530, 9_400_531];

/// `key`, with the stage's item and count.
fn about(key: &str) -> String {
    let (item, n) = (CUPID_CODE_PIECE.to_string(), AMORIA_CODES.to_string());
    net::content::text().get(key, &[("item", &item), ("n", &n)])
}

/// `player` hears `text` from Amos, and closes it.
async fn hears(player: &mut WorldSession, text: &str) -> Trace {
    let t = told(player, talk(AMORIA_STAGE4, AMOS_TRIALS)).await;
    assert!(t.prompt_texts.iter().any(|s| s.contains(text)), "{t:?}");
    told(player, answer(0, 1, None)).await;
    t
}

// One test: a channel holds one run at a time.
#[tokio::test]
async fn fifty_cupid_code_pieces_open_the_way_on_and_cut_the_clock() {
    let short = AMORIA_CODES - 1;
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[(CUPID_CODE_PIECE, short)]).await;
    party(&mut team).await;
    enter_amoria(&mut team).await;
    let mut seen = Trace::default();
    for (n, player) in team.iter_mut().enumerate() {
        let mut t = Trace::default();
        let to = warp(&mut player.connection, AMORIA_STAGE4, "", &mut t).await;
        assert_eq!(to, AMORIA_STAGE4);
        if n == 0 {
            seen = t;
        }
    }
    assert_eq!(AMORIA.stage_on(AMORIA_STAGE4), Some(AMORIA_CODES_STAGE));
    // The leader sees the stage's monsters before the talks read on.
    let alive = |t: &Trace| (MOBS.iter()).flat_map(|&mob| t.field.alive(mob)).next();
    until(&mut team[0].connection, &mut seen, |t| alive(t).is_some()).await;
    let text = net::content::text();

    // Amos sends the others to the leader, who first hears the task; the
    // way on stays shut.
    hears(
        &mut team[1],
        &text.get(keys::AMORIA_PQ_AMOS_NOT_LEADER, &[]),
    )
    .await;
    let t = hears(&mut team[0], &about(keys::AMORIA_PQ_STAGE4_INTRO)).await;
    assert!(t.removed.is_empty() && t.quantities.is_empty(), "{t:?}");
    let mut t = Trace::default();
    let c = &mut team[1].connection;
    send(c, build_change_map(-1, "next00").unwrap()).await;
    until(c, &mut t, |t| !t.blocked.is_empty()).await;
    assert!(t.warps.is_empty(), "{t:?}");

    // One short is no hand-in.
    let t = hears(&mut team[0], &about(keys::AMORIA_PQ_STAGE4_SHORT)).await;
    assert!(t.removed.is_empty() && !cleared(&t), "{t:?}");

    // A monster's piece makes 50.
    let c = &mut team[0].connection;
    let mob = alive(&seen).unwrap();
    send(c, attack(mob, 30_000)).await;
    until(c, &mut seen, |t| {
        t.field.item_drop(CUPID_CODE_PIECE).is_some()
    })
    .await;
    let piece = seen.field.item_drop(CUPID_CODE_PIECE).unwrap();
    send(c, pickup(piece.object_id, piece.x, piece.y)).await;
    until(c, &mut seen, |t| {
        (t.quantities.iter()).any(|q| q.0 == 4 && q.2 == AMORIA_CODES)
    })
    .await;

    // Handing them in clears the stage, and cuts the clock.
    let done = text.get(keys::AMORIA_PQ_STAGE4_DONE, &[]);
    let notice = text.get(keys::AMORIA_PQ_STAGE4_NOTICE, &[]);
    let t = hears(&mut team[0], &done).await;
    assert_eq!(t.removed, [(4, 1)], "{t:?}");
    let mut t = Trace::default();
    let c = &mut team[1].connection;
    until(c, &mut t, |t| {
        cleared(t) && !t.gains.is_empty() && t.broadcasts.contains(&(PINK, notice.clone()))
    })
    .await;
    assert_eq!(t.gains, [AMORIA.stage_exp(AMORIA_CODES_STAGE)]);
    let rush = i32::try_from(AMORIA_RUSH_MINUTES).unwrap();
    assert!(
        (t.countdowns.last()).is_some_and(|&s| (1..=rush).contains(&s)),
        "{t:?}"
    );

    // The way on is open, until the cut clock runs out.
    send(c, build_change_map(-1, "next00").unwrap()).await;
    until(c, &mut t, |t| t.warps.contains(&STAGE5)).await;
    until(c, &mut t, |t| t.warps.contains(&AMORIA_EXIT)).await;
    let c = &mut team[0].connection;
    let mut t = Trace::default();
    until(c, &mut t, |t| t.warps.contains(&AMORIA_EXIT)).await;
}
