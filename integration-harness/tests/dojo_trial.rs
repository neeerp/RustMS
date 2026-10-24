//! A first-time challenger at the Mu Lung Dojo is tried by So Gong
//! himself. In the hall he offers his trial; its room holds him as a
//! monster, and its way out along the railing stays shut while he lives.
//! Beating him lets the challenger out to the hall, and from then on,
//! even after logging out, he takes them into the Dojo.

use game_data::server_content::keys;
use integration_harness::database::{replace_item, set_job, set_str, set_weapon_attack};
use integration_harness::events::{maps, said, talk, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{DOJO_FIRST, DOJO_HALL, DOJO_TAUNT_WEATHER, DOJO_TRIAL_MAP, DOJO_TRIAL_MOB};
use net::npc_scripts::SO_GONG;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PINK: u8 = 5;
/// The trial room's `dojang_tuto`, at the top of its ladder.
const OUT: &str = "out000";

/// CHANGE_MAP_SPECIAL: walk to the trial room's way out and press Up.
async fn leave(player: &mut WorldSession) {
    let portal = maps()
        .field(DOJO_TRIAL_MAP)
        .unwrap()
        .portal_by_name(OUT)
        .unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(OUT).unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

#[tokio::test]
async fn so_gong_tries_a_first_timer_before_letting_them_into_the_dojo() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    let mut player = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, DOJO_HALL, "", &mut seen).await, DOJO_HALL);

    // Going alone, he first offers his own trial.
    told(&mut player, talk(DOJO_HALL, SO_GONG)).await;
    let asked = told(&mut player, answer(4, 1, Some(0))).await;
    assert!(said(&asked, keys::DOJO_SO_GONG_TRIAL_ASK), "{asked:?}");
    let mut seen = Trace::default();
    send(&mut player.connection, answer(1, 1, None)).await;
    let taunt = net::content::text().get(keys::DOJO_TRIAL_TAUNT, &[]);
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_TRIAL_MAP)
            && t.weathers.contains(&(DOJO_TAUNT_WEATHER, taunt.clone()))
            && t.field.mobs.iter().any(|m| m.1 == DOJO_TRIAL_MOB)
    })
    .await;
    assert!(seen.countdowns.is_empty(), "{seen:?}");
    let gong = seen
        .field
        .mobs
        .iter()
        .find(|m| m.1 == DOJO_TRIAL_MOB)
        .unwrap()
        .0;

    // The way out stays shut while he stands.
    leave(&mut player).await;
    let stay = (PINK, net::content::text().get(keys::DOJO_TRIAL_STAY, &[]));
    until(&mut player.connection, &mut seen, |t| {
        t.broadcasts.contains(&stay)
    })
    .await;
    assert_eq!(seen.warps, [DOJO_TRIAL_MAP], "{seen:?}");

    // Beaten, he runs off crying, with the room's notice and the Dojo's
    // clear.
    let (_, ran_off) = maps()
        .field(DOJO_TRIAL_MAP)
        .unwrap()
        .time_mob
        .clone()
        .unwrap();
    let clear = (3, "dojang/end/clear".to_string());
    for _ in 0..50 {
        if seen.field_effects.contains(&clear) {
            break;
        }
        let hits = seen.field.hp_bars.len() + seen.field.boss_bars.len();
        send(&mut player.connection, attack(gong, 30_000)).await;
        until(&mut player.connection, &mut seen, |t| {
            t.field.hp_bars.len() + t.field.boss_bars.len() > hits
                || t.field_effects.contains(&clear)
        })
        .await;
    }
    until(&mut player.connection, &mut seen, |t| {
        t.field_effects.contains(&clear) && t.item_lines.contains(&(ran_off.clone(), 0))
    })
    .await;
    assert!(seen.field.killed.contains(&gong), "{seen:?}");

    // Now the railing leads back to the hall.
    leave(&mut player).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_HALL)
    })
    .await;
    drop(player);

    // And after logging out, So Gong takes them to the Dojo's first floor.
    let mut player = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, DOJO_HALL, "", &mut seen).await, DOJO_HALL);
    told(&mut player, talk(DOJO_HALL, SO_GONG)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_FIRST)
    })
    .await;
}

#[tokio::test]
async fn a_first_timer_may_turn_so_gongs_trial_down() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    let mut player = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, DOJO_HALL, "", &mut seen).await, DOJO_HALL);
    told(&mut player, talk(DOJO_HALL, SO_GONG)).await;
    told(&mut player, answer(4, 1, Some(0))).await;
    let refused = told(&mut player, answer(1, 0, None)).await;
    assert!(said(&refused, keys::DOJO_SO_GONG_TRIAL_NO), "{refused:?}");
}
