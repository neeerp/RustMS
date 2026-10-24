//! The Mu Lung Dojo, taken with a party. So Gong sends a party's leader
//! and the members with them in the hall to the party floors, if their
//! levels are within 30 of each other. Out of the floor before a rest
//! floor, once its boss is beaten, everyone on the section's floors
//! gathers on the rest floor, each with the points of the floors from
//! their own; from there the leader takes them on.

use game_data::server_content::keys;
use integration_harness::events::{dojo_challenger, maps, party, said, talk, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{dojo_boss, DOJO, DOJO_PARTY, DOJO_PARTY_FIRST, DOJO_UP_PORTAL};
use net::npc_scripts::SO_GONG;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const PINK: u8 = 5;
/// The floors' `dojang_up`, by the door, and `dojang_next`, behind it.
const UP: &str = "out00";
const NEXT: &str = "out001";

/// The leader, first, takes the party in; both land on the first floor.
async fn start(team: &mut [WorldSession]) {
    told(&mut team[0], talk(DOJO.lobby, SO_GONG)).await;
    send(&mut team[0].connection, answer(4, 1, Some(1))).await;
    for player in team {
        let mut seen = Trace::default();
        until(&mut player.connection, &mut seen, |t| {
            t.warps.contains(&DOJO_PARTY_FIRST)
        })
        .await;
    }
}

/// CHANGE_MAP_SPECIAL: walk to portal `name` on `map_id` and press Up.
async fn through(player: &mut WorldSession, map_id: i32, name: &str) {
    let portal = maps().field(map_id).unwrap().portal_by_name(name).unwrap();
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

fn points(earned: &str, total: &str) -> (u8, String) {
    let line = net::content::text().get(keys::DOJO_POINTS, &[("points", earned), ("total", total)]);
    (PINK, line)
}

#[tokio::test]
async fn a_party_gathers_on_the_rest_floor_and_its_leader_takes_it_on() {
    let mut team = vec![dojo_challenger(30).await, dojo_challenger(60).await];
    party(&mut team).await;
    start(&mut team).await;

    // The sixth floor stops the clock, so the floors after it are taken
    // at leisure: the leader on the 11th, the member two floors behind.
    for player in team.iter_mut() {
        let rest = DOJO_PARTY.stage_map(6);
        let mut seen = Trace::default();
        assert_eq!(
            warp(&mut player.connection, rest, "", &mut seen).await,
            rest
        );
        until(&mut player.connection, &mut seen, |t| t.clocks_stopped > 0).await;
    }
    let behind = DOJO_PARTY.stage_map(9);
    let mut member_seen = Trace::default();
    let c = &mut team[1].connection;
    assert_eq!(warp(c, behind, "", &mut member_seen).await, behind);
    let floor = DOJO_PARTY.stage_map(11);
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, floor, "", &mut seen).await, floor);
    until(c, &mut seen, |t| {
        t.field.mobs.iter().any(|m| Some(m.1) == dojo_boss(floor))
    })
    .await;
    let boss = seen
        .field
        .mobs
        .iter()
        .find(|m| Some(m.1) == dojo_boss(floor))
        .unwrap()
        .0;
    let clear = (3, "dojang/end/clear".to_string());
    for _ in 0..50 {
        if seen.field_effects.contains(&clear) {
            break;
        }
        let hits = seen.field.hp_bars.len() + seen.field.boss_bars.len();
        send(&mut team[0].connection, attack(boss, 30_000)).await;
        until(&mut team[0].connection, &mut seen, |t| {
            t.field.hp_bars.len() + t.field.boss_bars.len() > hits
                || t.field_effects.contains(&clear)
        })
        .await;
    }
    assert!(seen.field.killed.contains(&boss), "{seen:?}");

    // Behind the door, the way on gathers both on the 12th, each with the
    // points of the floors from their own.
    through(&mut team[0], floor, UP).await;
    until(&mut team[0].connection, &mut seen, |t| {
        t.dojo_warps.contains(&DOJO_UP_PORTAL)
    })
    .await;
    through(&mut team[0], floor, NEXT).await;
    let rest = DOJO_PARTY.stage_map(12);
    until(&mut team[0].connection, &mut seen, |t| {
        t.warps.contains(&rest) && t.broadcasts.contains(&points("2", "2"))
    })
    .await;
    until(&mut team[1].connection, &mut member_seen, |t| {
        t.warps.contains(&rest)
            && [points("2", "2"), points("2", "4"), points("2", "6")]
                .iter()
                .all(|line| t.broadcasts.contains(line))
    })
    .await;

    // The leader takes them both on to the 13th.
    told(&mut team[0], talk(rest, SO_GONG)).await;
    send(&mut team[0].connection, answer(4, 1, Some(0))).await;
    let on = DOJO_PARTY.stage_map(13);
    for player in team.iter_mut() {
        let mut seen = Trace::default();
        until(&mut player.connection, &mut seen, |t| t.warps.contains(&on)).await;
    }
}

#[tokio::test]
async fn so_gong_takes_a_party_only_from_its_leader_and_within_thirty_levels() {
    let mut team = vec![dojo_challenger(30).await, dojo_challenger(61).await];
    party(&mut team).await;
    told(&mut team[0], talk(DOJO.lobby, SO_GONG)).await;
    let broad = told(&mut team[0], answer(4, 1, Some(1))).await;
    assert!(said(&broad, keys::DOJO_SO_GONG_PARTY_SPREAD), "{broad:?}");
    send(&mut team[0].connection, answer(0, -1, None)).await;
    told(&mut team[1], talk(DOJO.lobby, SO_GONG)).await;
    let member = told(&mut team[1], answer(4, 1, Some(1))).await;
    assert!(said(&member, keys::DOJO_SO_GONG_PARTY_LEADER), "{member:?}");
}
