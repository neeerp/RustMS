//! Henesys's party quest, its Moon Bunny: out once the six moonflowers
//! bloom, it makes a rice cake now and then, which it drops for the party;
//! players can't hurt it, but other mobs that run into it can, and its
//! guards hear of every sixth hit. Once it goes home the party is told it
//! failed and sent out. The quest lasts 10 seconds here
//! (`RUSTMS_EVENT_SCALE`), and the bunny makes a cake every third of a
//! second.

use game_data::server_content::keys;
use integration_harness::events::{enter_hpq, party, throw_down, waiting_in};
use integration_harness::hunting::{attack, move_mob, walk_to};
use integration_harness::trace::{read_for, send, until};
use net::events::HPQ;
use net::packet::op::RecvOpcode;
use net::reactors::MOON_BUNNY;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::{Duration, Instant};

const FLOWERS: [i32; 6] = [9108000, 9108001, 9108002, 9108003, 9108004, 9108005];
const SEEDS: [i32; 6] = [4001095, 4001096, 4001097, 4001098, 4001099, 4001100];
const RICE_CAKE: i32 = 4001101;
const MAX_HP: i32 = 5000;

/// MOB_DAMAGE_MOB_FRIENDLY as the v83 client sends it (0x667e40): mob
/// `attacker` ran into the friendly mob `victim` near `character_id`.
fn mob_damage_friendly(attacker: i32, character_id: i32, victim: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MobDamageMobFriendly as i16)
        .unwrap();
    p.write_int(attacker).unwrap();
    p.write_int(character_id).unwrap();
    p.write_int(victim).unwrap();
    p
}

#[tokio::test]
async fn mobs_hurt_the_moon_bunny_until_it_goes_and_the_party_fails() {
    let mut a = Vec::new();
    for pair in SEEDS.chunks(2) {
        a.push(waiting_in(HPQ.lobby, 10, &[(pair[0], 1), (pair[1], 1)]).await);
    }
    party(&mut a).await;
    let mut traces = enter_hpq(&mut a).await;
    until(&mut a[0].connection, &mut traces[0], |t| {
        FLOWERS
            .iter()
            .all(|id| t.field.reactors.iter().any(|r| r.reactor_id == *id))
    })
    .await;
    let flowers = FLOWERS.map(|reactor_id| {
        *traces[0]
            .field
            .reactors
            .iter()
            .find(|r| r.reactor_id == reactor_id)
            .unwrap()
    });
    for (i, player) in a.iter_mut().enumerate() {
        for (slot, flower) in [1, 2].into_iter().zip(&flowers[2 * i..]) {
            send(&mut player.connection, walk_to(flower.x, flower.y - 10)).await;
            send(&mut player.connection, throw_down(slot, 1)).await;
        }
    }
    for (player, trace) in a.iter_mut().zip(traces.iter_mut()) {
        until(&mut player.connection, trace, |t| {
            t.field.mobs.iter().any(|&(_, mob)| mob == MOON_BUNNY)
        })
        .await;
    }
    let &(bunny, _) = traces[0]
        .field
        .mobs
        .iter()
        .find(|m| m.1 == MOON_BUNNY)
        .unwrap();
    let at = traces[0].field.mob_positions[&bunny];

    // A player's blow passes through it (its first hit below finds it
    // whole).
    send(&mut a[0].connection, attack(bunny, 100)).await;

    // Its rice cakes fall for the party, each one counted aloud.
    until(&mut a[0].connection, &mut traces[0], |t| {
        t.field.item_drop(RICE_CAKE).is_some() && t.item_lines.len() >= 2
    })
    .await;
    let text = net::content::text();
    let cake = text.get(keys::HPQ_BUNNY_CAKE, &[("count", &"1")]);
    assert_eq!(traces[0].item_lines[1], (cake, 0));

    // A mob its controller walks into it hurts it, a hit at a time,
    // until it goes home, without dying.
    let (controller, attacker) = traces
        .iter()
        .enumerate()
        .find_map(|(i, t)| {
            let oid = t.field.controlled.iter().find(|&&oid| oid != bunny)?;
            Some((i, *oid))
        })
        .expect("a player controls a mob");
    let character_id = a[controller].character_id;
    let c = &mut a[controller].connection;
    send(c, move_mob(attacker, at, false, 0, 0)).await;
    let t = &mut traces[controller];
    let lines = t.item_lines.len();
    for _ in 0..1000 {
        if t.field.killed.contains(&bunny) {
            break;
        }
        send(c, mob_damage_friendly(attacker, character_id, bunny)).await;
        read_for(c, t, 20).await;
    }
    assert!(t.field.killed.contains(&bunny), "{t:?}");
    let gone = Instant::now();
    assert_eq!(t.field.leave_types[&bunny], 0);
    let hits: Vec<_> = t
        .field
        .mobs_hurt
        .iter()
        .filter(|hit| hit.0 == bunny)
        .collect();
    assert!(hits.len() >= 6, "{hits:?}");
    assert_eq!(hits[0].2, MAX_HP - hits[0].1);
    assert_eq!(hits.last().unwrap().2, 0);
    assert!(hits.windows(2).all(|w| w[1].2 == (w[0].2 - w[1].1).max(0)));
    // Every sixth hit, even one for nothing (which the trace keeps as a
    // heal), its guards hear of it; then of its going home.
    let missed = t.field.heals.iter().filter(|heal| **heal == (bunny, 0));
    let landed = hits.len() + missed.count();
    let sick = text.get(keys::HPQ_BUNNY_SICK, &[]);
    read_for(c, t, 20).await;
    let heard: Vec<_> = t.item_lines[lines..].iter().map(|line| &line.0).collect();
    let told = heard.iter().filter(|line| ***line == sick).count();
    assert_eq!(told, (landed - 1) / 6, "{landed} hits: {heard:?}");
    assert!(heard.contains(&&text.get(keys::HPQ_BUNNY_GONE, &[])));

    // Its going ends the quest for everyone at once, with a pink notice,
    // well before its time is up.
    let lost = (5, text.get(keys::HPQ_BUNNY_LOST, &[]));
    for (player, trace) in a.iter_mut().zip(traces.iter_mut()) {
        until(&mut player.connection, trace, |t| {
            t.warps.contains(&HPQ.exit)
        })
        .await;
        assert!(trace.broadcasts.contains(&lost), "{:?}", trace.broadcasts);
    }
    assert!(gone.elapsed() < Duration::from_secs(1));
}
