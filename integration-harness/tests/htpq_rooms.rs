//! Horntail's party quest, its rooms: each of the first four rooms' doors
//! is sealed until its seal takes the room's Crystal Key, which it sends on
//! to the first map; and the last room's seal lifts only for the leader
//! holding the Blue Crystal Key, who goes back to the first map, and lets
//! anyone through after. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts 25 seconds.

use game_data::server_content::keys;
use integration_harness::events::{cleared, enter_htpq, maps, party, throw_down, told, waiting_in};
use integration_harness::hunting::walk_to;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{BLUE_CRYSTAL_KEY, CRYSTAL_KEYS, HTPQ, HTPQ_ENTRY, HTPQ_KEY_AT};
use net::packet::op::RecvOpcode;
use net::reactors::HTPQ_PASS_KEY;
use net::scripts::HTPQ_SEAL;
use packet::io::write::PktWrite;
use packet::Packet;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// The name of `map`'s portal running `script`.
fn scripted(map: i32, script: &str) -> String {
    let field = maps().field(map).unwrap();
    let portal = field
        .portals()
        .find(|p| p.script.as_deref() == Some(script));
    portal.unwrap().name.clone()
}

/// `player` presses Up on the seal of the room they are in, `map`.
async fn at_seal(player: &mut WorldSession, map: i32) -> Trace {
    told(player, enter(&scripted(map, HTPQ_SEAL))).await
}

/// Whether `trace` shows the line `key`, pink or an item line.
fn heard(trace: &Trace, key: &str) -> bool {
    let line = net::content::text().get(key, &[]);
    trace.broadcasts.iter().any(|(_, text)| *text == line)
        || trace.item_lines.iter().any(|(text, _)| *text == line)
}

#[tokio::test]
async fn each_seal_takes_its_rooms_key_and_the_leader_lifts_the_last() {
    let [first, second] = [CRYSTAL_KEYS[1], CRYSTAL_KEYS[2]];
    let mut a = vec![waiting_in(HTPQ.lobby, 120, &[(BLUE_CRYSTAL_KEY, 1)]).await];
    a.push(waiting_in(HTPQ.lobby, 120, &[(first, 1), (second, 1)]).await);
    for _ in 0..4 {
        a.push(waiting_in(HTPQ.lobby, 120, &[]).await);
    }
    party(&mut a).await;
    enter_htpq(&mut a).await;

    // Two go into the first room, whose door is sealed.
    let room = HTPQ.stage_map(1);
    let mut seen = Trace::default();
    for player in a[1..3].iter_mut() {
        let mut t = Trace::default();
        send(&mut player.connection, enter("in00")).await;
        until(&mut player.connection, &mut t, |t| t.warps.contains(&room)).await;
    }
    let trace = at_seal(&mut a[2], room).await;
    assert!(heard(&trace, keys::HTPQ_SEALED), "{trace:?}");
    assert!(trace.warps.is_empty(), "{trace:?}");

    // Its seal leaves the second room's key on the ground, and takes its own.
    let c = &mut a[1].connection;
    until(c, &mut seen, |t| {
        t.field
            .reactors
            .iter()
            .any(|r| r.reactor_id == HTPQ_PASS_KEY)
    })
    .await;
    let seal = *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == HTPQ_PASS_KEY)
        .unwrap();
    send(c, walk_to(seal.x, seal.y - 10)).await;
    send(c, throw_down(2, 1)).await;
    send(c, throw_down(1, 1)).await;
    until(c, &mut seen, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == seal.object_id)
            && heard(t, keys::HTPQ_KEY_SENT)
            && cleared(t)
    })
    .await;
    let left = seen.field.item_drop(second).unwrap();
    assert!(seen.field.removals(left.object_id).is_empty(), "{seen:?}");

    // The key turns up on the first map, for anyone there.
    let mut there = Trace::default();
    until(&mut a[5].connection, &mut there, |t| {
        t.field.item_drop(first).is_some() && heard(t, keys::HTPQ_KEY_APPEARED)
    })
    .await;
    let key = there.field.item_drop(first).unwrap();
    assert_eq!(key.ownership, 2, "{there:?}");
    assert!((key.x - HTPQ_KEY_AT.0).abs() <= 100, "{there:?}");

    // The door opens on the next room.
    let trace = at_seal(&mut a[2], room).await;
    assert_eq!(
        trace.arrivals.first().map(|a| (a.0, a.1)),
        Some((HTPQ.stage_map(2), 0)),
        "{trace:?}"
    );

    // The last room's seal holds for anyone but the leader.
    let last = HTPQ.stage_map(5);
    for i in [3, 0] {
        let mut t = Trace::default();
        assert_eq!(warp(&mut a[i].connection, last, "", &mut t).await, last);
    }
    let trace = at_seal(&mut a[3], last).await;
    assert!(heard(&trace, keys::HTPQ_SEAL_LEADER_ONLY), "{trace:?}");
    assert!(trace.warps.is_empty(), "{trace:?}");

    // The leader's Blue Key lifts it and takes the leader back, keeping it.
    let start = maps()
        .field(HTPQ_ENTRY)
        .unwrap()
        .portal_by_name("st00")
        .unwrap()
        .id as u8;
    let trace = at_seal(&mut a[0], last).await;
    assert!(heard(&trace, keys::HTPQ_SEAL_LIFTED), "{trace:?}");
    assert_eq!(
        trace.arrivals.first().map(|a| (a.0, a.1)),
        Some((HTPQ_ENTRY, start)),
        "{trace:?}"
    );
    assert!(trace.removed.is_empty(), "{trace:?}");

    // Then anyone goes through.
    let trace = at_seal(&mut a[3], last).await;
    assert_eq!(
        trace.arrivals.first().map(|a| (a.0, a.1)),
        Some((HTPQ_ENTRY, start)),
        "{trace:?}"
    );
}
