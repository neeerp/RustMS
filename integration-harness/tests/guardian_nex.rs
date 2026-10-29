//! Guardian Nex: the Tera Forest's `in00` (`TD_neo_inTree`) takes one on
//! an era's Nex the Time Guard quest alone into that era's room for
//! fifteen minutes, one at a time an era on a channel. The Nex there's
//! death wins it with the clear effect, once; the player walks out by
//! `out00`, and the tree then turns them away until the quest is ended.
use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    replace_item, set_hp, set_job, set_quest, set_str, set_weapon_attack,
};
use integration_harness::events::cleared;
use integration_harness::hunting::{attack, walk_to};
use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{nex_room, NEX_BOSSES, NEX_QUESTS, TERA_FOREST};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

/// The tree's way in, and the room's way back out.
const TREE_AT: (i16, i16) = (-39, 46);
const OUT_AT: (i16, i16) = (490, -397);
/// Andy the Time Traveler, before the first era's Nex.
const TIME_GATE_QUEST: i16 = 3718;
/// Under the most a level-70 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;
/// The HP stat's bit.
const HP: i32 = 0x400;

/// A new level-70 character hitting very hard, on the first era's Nex
/// quest, by the Tera Forest's tree.
async fn by_the_tree() -> WorldSession {
    on_era(0, None).await
}

/// `by_the_tree`, on era `era`'s Nex quest, with `hp` HP if given.
async fn on_era(era: usize, hp: Option<i16>) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 70, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    set_quest(id, TIME_GATE_QUEST, 2, "").await.unwrap();
    set_quest(id, NEX_QUESTS[era], 1, "").await.unwrap();
    if let Some(hp) = hp {
        set_hp(id, hp, hp).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, TERA_FOREST, "", &mut seen).await, TERA_FOREST);
    step(c, &config.character_name, walk_to(TREE_AT.0, TREE_AT.1)).await;
    session
}

/// Going through a scripted portal.
fn touch(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Whether `trace` shows the pink line `key`.
fn pinked(trace: &Trace, key: &str) -> bool {
    let text = net::content::text().get(key, &[]);
    (trace.broadcasts.iter()).any(|(kind, line)| *kind == 5 && *line == text)
}

/// Hit `object_id` until `seen` sees it die.
async fn beat(c: &mut MapleTestConnection, seen: &mut Trace, object_id: i32) {
    for _ in 0..20 {
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

#[tokio::test]
async fn the_tera_forests_tree_takes_one_alone_to_the_eras_nex() {
    let mut first = by_the_tree().await;
    let mut second = by_the_tree().await;
    let nex = NEX_BOSSES[0];
    let room = nex_room(0);

    // In alone, where the era's Nex waits.
    let mut seen = Trace::default();
    send(&mut first.connection, touch("in00")).await;
    until(&mut first.connection, &mut seen, |t| {
        t.arrivals.iter().any(|a| a.0 == room) && !t.field.alive(nex).is_empty()
    })
    .await;

    // One at a time an era.
    let name = second.character_name.clone();
    let t = step(&mut second.connection, &name, touch("in00")).await;
    assert!(pinked(&t, keys::GUARDIAN_NEX_BUSY), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");

    // Its death wins it, with one clear effect.
    let c = &mut first.connection;
    let object_id = seen.field.alive(nex)[0];
    beat(c, &mut seen, object_id).await;
    until(c, &mut seen, cleared).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let name = first.character_name.clone();
    let c = &mut first.connection;
    let t = step(c, &name, walk_to(OUT_AT.0, OUT_AT.1)).await;
    seen.field_effects.extend(t.field_effects);
    let clears = (seen.field_effects.iter())
        .filter(|(_, e)| e == "quest/party/clear")
        .count();
    assert_eq!(clears, 1, "{seen:?}");

    // Out by `out00` to the forest, where the tree turns them away now.
    send(c, build_change_map(-1, "out00").unwrap()).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| t.arrivals.iter().any(|a| a.0 == TERA_FOREST)).await;
    step(c, &name, walk_to(TREE_AT.0, TREE_AT.1)).await;
    let t = step(c, &name, touch("in00")).await;
    assert!(pinked(&t, keys::GUARDIAN_NEX_FACED), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");

    // The era is free again.
    let mut t = Trace::default();
    send(&mut second.connection, touch("in00")).await;
    until(&mut second.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == room) && !t.field.alive(nex).is_empty()
    })
    .await;
}

#[tokio::test]
async fn the_tree_turns_away_one_on_no_nex_quest() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, TERA_FOREST, "", &mut seen).await, TERA_FOREST);
    step(c, &config.character_name, walk_to(TREE_AT.0, TREE_AT.1)).await;
    let t = step(c, &config.character_name, touch("in00")).await;
    assert!(pinked(&t, keys::GUARDIAN_NEX_NO_WAY), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");
}

/// TAKE_DAMAGE: touched by `mob_id` for `damage`.
fn touched(damage: i32, mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

#[tokio::test]
async fn one_dead_in_the_nexs_room_wakes_in_the_tera_forest() {
    // The second era's, so as not to take the first's from the test above.
    let mut player = on_era(1, Some(50)).await;
    let c = &mut player.connection;
    let mut seen = Trace::default();
    send(c, touch("in00")).await;
    until(c, &mut seen, |t| {
        t.arrivals.iter().any(|a| a.0 == nex_room(1))
    })
    .await;
    for _ in 0..40 {
        send(c, touched(20, NEX_BOSSES[1])).await;
        read_for(c, &mut seen, 150).await;
        if seen.stat(HP) == Some(0) {
            break;
        }
    }
    assert_eq!(seen.stat(HP), Some(0), "{seen:?}");
    assert_eq!(warp(c, nex_room(1), "", &mut seen).await, TERA_FOREST);
}
