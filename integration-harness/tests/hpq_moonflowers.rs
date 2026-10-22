//! Henesys's party quest, its moonflowers: each of Primrose Hill's six
//! takes its own seed put on it, a few seconds after it lands, and stays in
//! bloom; the full moon waxes a phase a bloom, and with the sixth the Moon
//! Bunny comes out below it as the party is told to guard it. The quest
//! lasts 10 seconds here (`RUSTMS_EVENT_SCALE`), so the party plants at once.

use integration_harness::events::{enter_hpq, party, waiting_in};
use integration_harness::hunting::walk_to;
use integration_harness::trace::{send, until};
use net::events::HPQ;
use net::packet::op::RecvOpcode;
use net::reactors::MOON_BUNNY;
use packet::io::write::PktWrite;
use packet::Packet;

const ETC: u8 = 4;
const FLOWERS: [i32; 6] = [9108000, 9108001, 9108002, 9108003, 9108004, 9108005];
const SEEDS: [i32; 6] = [4001095, 4001096, 4001097, 4001098, 4001099, 4001100];
const FULL_MOON: i32 = 9101000;
const WEATHER: i32 = 5120016;

/// Drop one of what lies in the Etc tab's `slot`.
fn throw(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(ETC).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(1).unwrap();
    p
}

#[tokio::test]
async fn six_seeds_bloom_the_moonflowers_and_bring_out_the_moon_bunny() {
    // Each plants two flowers; the first also has a seed of the wrong
    // colour, which the first flower leaves lying.
    let mut a = vec![
        waiting_in(HPQ.lobby, 10, &[(SEEDS[1], 2), (SEEDS[0], 1)]).await,
        waiting_in(HPQ.lobby, 10, &[(SEEDS[2], 1), (SEEDS[3], 1)]).await,
        waiting_in(HPQ.lobby, 10, &[(SEEDS[4], 1), (SEEDS[5], 1)]).await,
    ];
    party(&mut a).await;
    let mut traces = enter_hpq(&mut a).await;
    let (player, trace) = (&mut a[0], &mut traces[0]);
    until(&mut player.connection, trace, |t| {
        [FULL_MOON]
            .iter()
            .chain(&FLOWERS)
            .all(|id| t.field.reactors.iter().any(|r| r.reactor_id == *id))
    })
    .await;
    let reactor = |reactor_id| {
        *traces[0]
            .field
            .reactors
            .iter()
            .find(|r| r.reactor_id == reactor_id)
            .unwrap()
    };
    let flowers = FLOWERS.map(reactor);
    let moon = reactor(FULL_MOON);
    assert!(flowers.iter().chain([&moon]).all(|r| r.state == 0));

    let plantings: [&[(usize, i16)]; 3] = [
        &[(0, 1), (0, 2), (1, 1)],
        &[(2, 1), (3, 2)],
        &[(4, 1), (5, 2)],
    ];
    for (player, planting) in a.iter_mut().zip(plantings) {
        for &(flower, slot) in planting {
            let flower = flowers[flower];
            send(&mut player.connection, walk_to(flower.x, flower.y - 10)).await;
            send(&mut player.connection, throw(slot)).await;
        }
    }

    let (player, trace) = (&mut a[0], &mut traces[0]);
    until(&mut player.connection, trace, |t| {
        t.field.mobs.iter().any(|&(_, mob)| mob == MOON_BUNNY)
    })
    .await;
    // It comes out on the ground below the moon.
    let &(bunny, _) = trace.field.mobs.iter().find(|m| m.1 == MOON_BUNNY).unwrap();
    let (x, y) = trace.field.mob_positions[&bunny];
    assert_eq!(x, moon.x);
    assert!((moon.y..moon.y + 300).contains(&y), "{:?}", (x, y, moon.y));
    for flower in flowers {
        assert!(
            trace.field.reactor_hits.contains(&(flower.object_id, 1)),
            "{trace:?}"
        );
    }
    let phases: Vec<u8> = trace
        .field
        .reactor_hits
        .iter()
        .filter(|&&(object_id, _)| object_id == moon.object_id)
        .map(|&(_, state)| state)
        .collect();
    assert_eq!(phases, [1, 2, 3, 4, 5, 6]);
    // The flowers stay in bloom, and the wrong seed lies where it fell.
    assert!(trace.field.reactors_removed.is_empty(), "{trace:?}");
    let lying: Vec<usize> = trace
        .field
        .drops
        .iter()
        .filter(|d| !d.meso && d.value == SEEDS[1])
        .map(|d| trace.field.removals(d.object_id).len())
        .collect();
    assert_eq!(lying, [0, 1], "{trace:?}");
    until(&mut player.connection, trace, |t| {
        !t.item_lines.is_empty() && !t.weathers.is_empty()
    })
    .await;
    assert_eq!(trace.weathers[0].0, WEATHER);
    assert_eq!(trace.item_lines[0].1, 0);
}
