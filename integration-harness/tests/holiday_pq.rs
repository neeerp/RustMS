//! The Holiday PQ, its first slice: in each of its three lobbies, the Snow
//! Spirit takes those of a party of three to six of the difficulty's ten
//! levels who are there into the difficulty's own arena, on a clock (25
//! minutes on the hardest, for levels 41 to 50); the Snowman there lets a
//! member out to the exit map, which leaves too few and ends it for
//! everyone, who land there too; the exit's portal leads back to the
//! lobby, and the lobby's Snowman home to Happyville. The harness world
//! runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the clock reads
//! seconds. In Happyville, Santa tells off anyone going down the chimney
//! there.

use game_data::server_content::keys;
use integration_harness::events::{enter_holiday, maps, party, said, talk, told, waiting_in};
use integration_harness::hunting::walk_to;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{HAPPYVILLE, HOLIDAY};
use net::npc_scripts::{SANTA, SANTAS_CHIMNEY, SNOWMAN, SNOW_SPIRIT};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

/// The hardest difficulty, for levels 41 to 50.
const HARD: u8 = 3;

/// CHANGE_MAP_SPECIAL: walk to `map`'s portal `name` and press Up.
async fn take(player: &mut WorldSession, map: i32, name: &str) {
    let field = maps().field(map).unwrap();
    let portal = field.portal_by_name(name).unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

#[tokio::test]
async fn santa_greets_and_tells_off_one_going_down_the_chimney() {
    let mut player = waiting_in(HAPPYVILLE, 30, &[]).await;
    let hello = told(&mut player, talk(HAPPYVILLE, SANTA)).await;
    assert!(said(&hello, keys::HOLIDAY_SANTA_GREETING), "{hello:?}");
    told(&mut player, answer(0, 0, None)).await;

    let mut seen = Trace::default();
    take(&mut player, HAPPYVILLE, SANTAS_CHIMNEY).await;
    until(&mut player.connection, &mut seen, |t| {
        said(t, keys::HOLIDAY_SANTA_CHIMNEY)
    })
    .await;
    assert!(seen.arrivals.is_empty(), "{seen:?}");
}

#[tokio::test]
async fn the_snow_spirit_takes_three_into_the_arena_and_the_snowman_lets_them_out() {
    let hard = &HOLIDAY[usize::from(HARD) - 1];
    let mut team = Vec::new();
    for level in [41, 45, 50, 51] {
        team.push(waiting_in(hard.lobby, level, &[]).await);
    }
    party(&mut team).await;

    // Only the leader may take the party in, and only with three of levels
    // 41 to 50 there: with one away, the one of level 51 makes no third.
    let mut trace = Trace::default();
    let away = &mut team[2].connection;
    assert_eq!(warp(away, HAPPYVILLE, "", &mut trace).await, HAPPYVILLE);
    told(&mut team[1], talk(hard.lobby, SNOW_SPIRIT)).await;
    let trace = told(&mut team[1], answer(4, 1, Some(0))).await;
    assert!(said(&trace, keys::HOLIDAY_NOT_LEADER), "{trace:?}");
    let menu = told(&mut team[0], talk(hard.lobby, SNOW_SPIRIT)).await;
    assert_eq!(menu.prompts, [(SNOW_SPIRIT, 4)], "{menu:?}");
    let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
    let not_eligible = net::content::text().get(
        keys::HOLIDAY_NOT_ELIGIBLE,
        &[("least", "3"), ("most", "6"), ("low", "41"), ("high", "50")],
    );
    assert!(
        trace.prompt_texts.iter().any(|t| t.contains(&not_eligible)),
        "{trace:?}"
    );

    // With them back, the three go into the hard arena, on its clock, and
    // the one of level 51 stays.
    let mut trace = Trace::default();
    let back = &mut team[2].connection;
    assert_eq!(warp(back, hard.lobby, "", &mut trace).await, hard.lobby);
    let mut left_behind = team.pop().unwrap();
    let mut seen = enter_holiday(&mut team, HARD).await;
    for (player, trace) in team.iter_mut().zip(seen.iter_mut()) {
        until(&mut player.connection, trace, |t| !t.countdowns.is_empty()).await;
        assert!((20..=25).contains(&trace.countdowns[0]), "{trace:?}");
    }
    let mut trace = Trace::default();
    read_for(&mut left_behind.connection, &mut trace, 200).await;
    assert!(!trace.warps.contains(&hard.entry), "{trace:?}");

    // A member gives up at the Snowman: the two left are too few, and land
    // on the exit map too.
    let trace = told(&mut team[2], talk(hard.entry, SNOWMAN)).await;
    assert_eq!(trace.prompts, [(SNOWMAN, 1)], "{trace:?}");
    assert!(said(&trace, keys::HOLIDAY_SNOWMAN_LEAVE), "{trace:?}");
    let trace = told(&mut team[2], answer(1, 1, None)).await;
    assert_eq!(trace.warps, [hard.exit], "{trace:?}");
    for player in team[..2].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&hard.exit)
        })
        .await;
    }

    // The exit's portal leads back to the lobby.
    take(&mut team[0], hard.exit, "st01").await;
    let mut trace = Trace::default();
    until(&mut team[0].connection, &mut trace, |t| {
        t.warps.contains(&hard.lobby)
    })
    .await;

    // The lobby's Snowman leads home to Happyville.
    let trace = told(&mut team[0], talk(hard.lobby, SNOWMAN)).await;
    assert!(said(&trace, keys::HOLIDAY_SNOWMAN_HOME), "{trace:?}");
    let trace = told(&mut team[0], answer(1, 1, None)).await;
    assert_eq!(trace.warps, [HAPPYVILLE], "{trace:?}");
}
