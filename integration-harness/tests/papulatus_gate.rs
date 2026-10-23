//! The way to Papulatus: at Deep Inside the Clocktower (220080000) a
//! party's leader takes the party through the `in00` portal to the Origin
//! of Clocktower, one party a channel at a time. While it is in there the
//! three gates before the portal are shut and the next party is turned
//! away; the fight lasts until its last member leaves by the Machine
//! Apparatus, or its 45 minutes are up, when the gates open again. The
//! harness world runs events 60 times as fast.

use game_data::server_content::keys;
use integration_harness::events::{leave_party, party, talk, told, waiting_in};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use integration_harness::WorldSession;
use net::events::{CLOCKTOWER_DEPTHS, ORIGIN_OF_CLOCKTOWER};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MACHINE_APPARATUS: i32 = 2041025;
const GATES: [i32; 3] = [2208001, 2208002, 2208003];
const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `in00`.
fn enter() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("in00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// `player` tries the portal and is told the line `key`, staying put.
async fn turned_away(player: &mut WorldSession, key: &str) {
    let line = net::content::text().get(key, &[]);
    let t = told(player, enter()).await;
    assert!(
        t.broadcasts.contains(&(PINK, line)) && t.arrivals.is_empty(),
        "{t:?}"
    );
}

/// Every player in `team` arrives at the Origin of Clocktower, on its
/// 45-minute clock.
async fn all_arrive(team: &mut [WorldSession]) {
    for player in team {
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == ORIGIN_OF_CLOCKTOWER) && !t.countdowns.is_empty()
        })
        .await;
        assert!((40..=45).contains(&t.countdowns[0]), "{t:?}");
    }
}

/// `player` asks the Machine Apparatus to take them back.
async fn leave(player: &mut WorldSession) {
    let t = told(player, talk(ORIGIN_OF_CLOCKTOWER, MACHINE_APPARATUS)).await;
    assert_eq!(t.prompts, [(MACHINE_APPARATUS, 1)], "{t:?}");
    send(&mut player.connection, answer(1, 1, None)).await;
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == CLOCKTOWER_DEPTHS)
    })
    .await;
}

/// Wait until `watcher` sees the three gates go to `state`.
async fn gates_go(watcher: &mut WorldSession, seen: &mut Trace, gates: &[i32], state: u8) {
    until(&mut watcher.connection, seen, |t| {
        gates
            .iter()
            .all(|&gate| t.field.reactor_hits.contains(&(gate, state)))
    })
    .await;
}

#[tokio::test]
async fn one_party_at_a_time_goes_to_papulatus_until_it_is_empty_or_out_of_time() {
    let mut watcher = waiting_in(CLOCKTOWER_DEPTHS, 50, &[]).await;
    let mut first = Vec::new();
    let mut second = Vec::new();
    for _ in 0..2 {
        first.push(waiting_in(CLOCKTOWER_DEPTHS, 50, &[]).await);
        second.push(waiting_in(CLOCKTOWER_DEPTHS, 50, &[]).await);
    }
    party(&mut first).await;
    party(&mut second).await;

    // The watcher sees the gates open, and may not pass without a party.
    let mut seen = Trace::default();
    until(&mut watcher.connection, &mut seen, |t| {
        GATES
            .iter()
            .all(|&id| t.field.reactors.iter().any(|r| r.reactor_id == id))
    })
    .await;
    let gates: Vec<i32> = GATES
        .iter()
        .map(|&id| {
            let gate = seen.field.reactors.iter().find(|r| r.reactor_id == id);
            let gate = gate.unwrap();
            assert_eq!(gate.state, 0, "{:?}", seen.field.reactors);
            gate.object_id
        })
        .collect();
    turned_away(&mut watcher, keys::PAPULATUS_NO_PARTY).await;

    // Only the leader takes the party in, and the gates shut behind it.
    turned_away(&mut first[1], keys::PAPULATUS_NOT_LEADER).await;
    send(&mut first[0].connection, enter()).await;
    all_arrive(&mut first).await;
    gates_go(&mut watcher, &mut seen, &gates, 1).await;
    turned_away(&mut second[0], keys::PAPULATUS_BUSY).await;

    // Leaving the party inside, or the leader leaving, does not end the
    // fight; its last member leaving does, and the gates open for the next
    // party.
    send(&mut first[1].connection, leave_party()).await;
    turned_away(&mut second[0], keys::PAPULATUS_BUSY).await;
    leave(&mut first[0]).await;
    turned_away(&mut second[0], keys::PAPULATUS_BUSY).await;
    leave(&mut first[1]).await;
    gates_go(&mut watcher, &mut seen, &gates, 0).await;
    let mut seen = Trace::default();
    send(&mut second[0].connection, enter()).await;
    all_arrive(&mut second).await;
    gates_go(&mut watcher, &mut seen, &gates, 1).await;

    // Its time up, the next party is sent back, and the gates open.
    tokio::time::sleep(Duration::from_secs(38)).await;
    for player in &mut second {
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == CLOCKTOWER_DEPTHS)
        })
        .await;
    }
    gates_go(&mut watcher, &mut seen, &gates, 0).await;
}
