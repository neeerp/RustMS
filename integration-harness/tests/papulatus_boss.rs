//! Papulatus on the Origin of Clocktower: a party in the fight drops the
//! Piece of Cracked Dimension on the cracked dimension, which plays the
//! boss's music and brings out the Time Sphere; it dies into the Papulatus
//! Clock, and that into Papulatus, whose death the party sees cleared. The
//! win opens the gates on Deep Inside the Clocktower again.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_str, set_weapon_attack,
};
use integration_harness::events::{party, throw_down, waiting_in};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{CLOCKTOWER_DEPTHS, ORIGIN_OF_CLOCKTOWER};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PIECE_OF_CRACKED_DIMENSION: i32 = 4031179;
const GATES: [i32; 3] = [2208001, 2208002, 2208003];
const PINK: u8 = 5;
/// Under the most a level-50 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new level-50 character hitting very hard and holding the Piece of
/// Cracked Dimension, logged in on Deep Inside the Clocktower.
async fn hunter() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.unwrap();
    grant_stack(id, 4, 1, PIECE_OF_CRACKED_DIMENSION, 1)
        .await
        .unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, CLOCKTOWER_DEPTHS, "", &mut seen).await,
        CLOCKTOWER_DEPTHS
    );
    session
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `in00`.
fn enter() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("in00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// Hit `object_id` until `seen` sees it die, each hit once answered.
async fn beat(c: &mut MapleTestConnection, seen: &mut Trace, object_id: i32) {
    for _ in 0..40 {
        if seen.field.killed.contains(&object_id) {
            return;
        }
        let bars = seen.field.hp_bars.len();
        send(c, attack(object_id, HIT)).await;
        until(c, seen, |t| {
            t.field.hp_bars.len() > bars || t.field.killed.contains(&object_id)
        })
        .await;
    }
    panic!("{object_id} lives: {seen:?}");
}

/// Wait until `watcher` sees the three gates go to `state`.
async fn gates_go(watcher: &mut WorldSession, seen: &mut Trace, state: u8) {
    until(&mut watcher.connection, seen, |t| {
        GATES.iter().all(|&id| {
            t.field
                .reactors
                .iter()
                .filter(|r| r.reactor_id == id)
                .any(|r| t.field.reactor_hits.contains(&(r.object_id, state)))
        })
    })
    .await;
}

#[tokio::test]
async fn the_cracked_dimension_brings_out_papulatus_whose_death_wins_the_fight() {
    let mut watcher = waiting_in(CLOCKTOWER_DEPTHS, 50, &[]).await;
    let mut team = vec![hunter().await, waiting_in(CLOCKTOWER_DEPTHS, 50, &[]).await];
    party(&mut team).await;
    let mut seen = Trace::default();
    until(&mut watcher.connection, &mut seen, |t| {
        GATES
            .iter()
            .all(|&id| t.field.reactors.iter().any(|r| r.reactor_id == id))
    })
    .await;

    // The party goes in, and the gates shut behind it.
    send(&mut team[0].connection, enter()).await;
    let mut traces = Vec::new();
    for player in &mut team {
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == ORIGIN_OF_CLOCKTOWER)
        })
        .await;
        traces.push(t);
    }
    gates_go(&mut watcher, &mut seen, 1).await;
    let (mut ht, mut mt) = (traces.remove(0), traces.remove(0));
    let (hunter, member) = team.split_at_mut(1);
    let (h, m) = (&mut hunter[0].connection, &mut member[0].connection);

    // The Piece of Cracked Dimension, dropped on the ledge above the crack.
    send(h, walk_to(-124, -545)).await;
    send(h, throw_down(1, 1)).await;
    let summoned = net::content::text().get(keys::PAPULATUS_SUMMONED, &[]);
    until(m, &mut mt, |t| {
        t.broadcasts.contains(&(PINK, summoned.clone()))
            && !t.field.alive(net::mobs::TIME_SPHERE).is_empty()
    })
    .await;
    assert!(
        mt.field_effects
            .contains(&(6, "Bgm09/TimeAttack".to_string())),
        "{mt:?}"
    );

    // The sphere dies into the clock, and the clock into Papulatus.
    for mob_id in [net::mobs::TIME_SPHERE, 8500001, net::mobs::PAPULATUS_LAST] {
        until(h, &mut ht, |t| !t.field.alive(mob_id).is_empty()).await;
        let oid = ht.field.alive(mob_id)[0];
        beat(h, &mut ht, oid).await;
    }
    until(m, &mut mt, |t| {
        t.field_effects.contains(&(4, "Party1/Clear".to_string()))
    })
    .await;

    // Won, the fight opens the gates.
    gates_go(&mut watcher, &mut seen, 0).await;
}
