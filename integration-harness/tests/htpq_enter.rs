//! Horntail's party quest, its first slice: the gatekeeper at the Cave
//! Entrance takes six of a party, all of level 120 or more, into their own
//! copy of the Trial Grounds. The way into the rooms keeps back the first
//! Crystal Key and the last one on the map; the leader hands the gatekeeper
//! the five Crystal Keys, which takes the whole team on to the lamp's room;
//! and one member's leaving by the Crystal of Roots ends it for all, with
//! the quest's keys taken. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts 25 seconds.

use game_data::server_content::keys;
use integration_harness::events::{
    enter_htpq, party, said, talk, told, waiting_in, CRYSTAL_OF_ROOTS, HTPQ_KEEPER,
};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{CRYSTAL_KEYS, HTPQ, HTPQ_ENTRY, HTPQ_LAMP, RED_CRYSTAL_KEY};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const FIRST_ROOM: i32 = HTPQ_ENTRY + 1;
const CAVE_OF_LIFE_ENTRANCE: i32 = 240_040_700;

/// CHANGE_MAP_SPECIAL: press Up on the first map's way into the rooms.
fn into_rooms() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("in00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// `player` presses Up on the way into the rooms and is told `key` in pink.
async fn kept_back(player: &mut WorldSession, key: &str) {
    let line = net::content::text().get(key, &[]);
    let mut t = Trace::default();
    send(&mut player.connection, into_rooms()).await;
    until(&mut player.connection, &mut t, |t| {
        t.broadcasts.contains(&(5, line.clone()))
    })
    .await;
    assert!(t.warps.is_empty(), "{t:?}");
}

/// Wait until `player` is moved to `map`.
async fn moved_to(player: &mut WorldSession, map: i32) -> Trace {
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| t.warps.contains(&map)).await;
    t
}

#[tokio::test]
async fn the_gatekeeper_takes_six_in_and_the_five_keys_take_them_on() {
    // The leader holds the five Crystal Keys; another a Red Crystal Key.
    let five: Vec<_> = CRYSTAL_KEYS.iter().map(|&key| (key, 1)).collect();
    let mut a = vec![waiting_in(HTPQ.lobby, 120, &five).await];
    a.push(waiting_in(HTPQ.lobby, 120, &[(RED_CRYSTAL_KEY, 1)]).await);
    for _ in 0..4 {
        a.push(waiting_in(HTPQ.lobby, 120, &[]).await);
    }
    party(&mut a).await;

    // Five there aren't enough.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut a[5].connection, 240_000_000, "", &mut trace).await,
        240_000_000
    );
    told(&mut a[0], talk(HTPQ.lobby, HTPQ_KEEPER)).await;
    let trace = told(&mut a[0], answer(4, 1, Some(0))).await;
    assert_eq!(trace.prompts, [(HTPQ_KEEPER, 0)]);
    assert!(trace.warps.is_empty(), "{trace:?}");
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut a[5].connection, HTPQ.lobby, "", &mut trace).await,
        HTPQ.lobby
    );

    // Six go in, on the clock.
    let traces = enter_htpq(&mut a).await;
    for (player, mut trace) in a.iter_mut().zip(traces) {
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        assert!((20..=25).contains(&trace.countdowns[0]), "{trace:?}");
    }

    // The gatekeeper hears only the leader.
    let trace = told(&mut a[1], talk(HTPQ_ENTRY, HTPQ_KEEPER)).await;
    assert_eq!(trace.prompts, [(HTPQ_KEEPER, 0)]);
    assert!(said(&trace, keys::HTPQ_KEEPER_LEADER_ONLY), "{trace:?}");

    // The leader can't take the first key into the rooms; the others go
    // in, until the leader is the last on the map.
    kept_back(&mut a[0], keys::HTPQ_ROOMS_HOLDING_KEY).await;
    for player in a[1..].iter_mut() {
        let mut t = Trace::default();
        send(&mut player.connection, into_rooms()).await;
        until(&mut player.connection, &mut t, |t| {
            t.warps.contains(&FIRST_ROOM)
        })
        .await;
    }
    kept_back(&mut a[0], keys::HTPQ_ROOMS_ALONE).await;

    // The leader's five keys take everyone on to the lamp.
    let mut seen = told(&mut a[0], talk(HTPQ_ENTRY, HTPQ_KEEPER)).await;
    until(&mut a[0].connection, &mut seen, |t| {
        t.warps.contains(&HTPQ_LAMP) && t.removed.len() == 5
    })
    .await;
    for player in a[1..].iter_mut() {
        moved_to(player, HTPQ_LAMP).await;
    }

    // One leaves by the Crystal of Roots, with their Red Key taken: the
    // quest ends, and the rest go back to the Cave Entrance.
    let trace = told(&mut a[1], talk(HTPQ_LAMP, CRYSTAL_OF_ROOTS)).await;
    assert_eq!(trace.prompts, [(CRYSTAL_OF_ROOTS, 1)]);
    let mut seen = told(&mut a[1], answer(1, 1, None)).await;
    until(&mut a[1].connection, &mut seen, |t| {
        t.warps.contains(&CAVE_OF_LIFE_ENTRANCE) && t.removed.contains(&(4, 1))
    })
    .await;
    for i in [0, 2, 3, 4, 5] {
        moved_to(&mut a[i], HTPQ.exit).await;
    }
}
