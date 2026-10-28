//! The Prime Minister's refight: once The Seal of the Empire (2331) is
//! done, the Wedding Hall's door (`TD_MC_enterboss2`) opens the hidden
//! Blocked Entrance (1300013), whose menu takes the leader's party, or one
//! with no party alone, into their own copy of the hall (106021601) for
//! ten minutes, one team a channel. The Prime Minister comes out there at
//! once, and its death wins it; the team then walks out by `left00`. The harness world runs events 60 times as
//! fast (`RUSTMS_EVENT_SCALE`), so the ten minutes are ten seconds.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::events::{cleared, party, said};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    BLOCKED_ENTRANCE, PRIME_MINISTER, PRIME_MINISTER_AT, SEAL_OF_THE_EMPIRE, STORY_OF_BETRAYAL,
    WEDDING_HALL_COPY, WEDDING_HALL_DOOR, WHERES_VIOLETTA,
};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

/// The hall's door: the portal `right00`, whose script is `TD_MC_enterboss2`.
const DOOR: &str = "right00";
const DOOR_AT: (i16, i16) = (255, 140);
const MENU: u8 = 4;
/// The hall's way back to the door.
const LEFT_AT: (i16, i16) = (-641, 142);
/// Under the most a level-35 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new level-35 character hitting very hard, done with the Mushroom
/// Castle's quests up to The Seal of the Empire, at the hall's door.
async fn sealed() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 35, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    for quest in [WHERES_VIOLETTA, STORY_OF_BETRAYAL, SEAL_OF_THE_EMPIRE] {
        set_quest(id, quest, 2, "").await.unwrap();
    }
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    let door = warp(c, WEDDING_HALL_DOOR, "", &mut seen).await;
    assert_eq!(door, WEDDING_HALL_DOOR);
    step(c, &config.character_name, walk_to(DOOR_AT.0, DOOR_AT.1)).await;
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

/// Up at the door, which opens the Blocked Entrance's menu, and pick the
/// Prime Minister: what came of it.
async fn ask_in(player: &mut WorldSession) -> Trace {
    let name = player.character_name.clone();
    let c = &mut player.connection;
    let t = step(c, &name, touch(DOOR)).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.prompts, [(BLOCKED_ENTRANCE, MENU)], "{t:?}");
    step(c, &name, answer(MENU, 1, Some(0))).await
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
async fn the_blocked_entrance_takes_a_party_or_one_alone_to_the_prime_minister() {
    let mut team = vec![sealed().await, sealed().await];
    let mut alone = sealed().await;
    party(&mut team).await;

    // Only the leader may take the party in.
    let t = ask_in(&mut team[1]).await;
    assert!(said(&t, keys::PRIME_MINISTER_NOT_LEADER), "{t:?}");

    // The leader takes both into the copy, where the Prime Minister is out
    // at once.
    let mut lt = ask_in(&mut team[0]).await;
    let mut traces = Vec::new();
    for (n, player) in team.iter_mut().enumerate() {
        let mut t = if n == 0 {
            std::mem::take(&mut lt)
        } else {
            Trace::default()
        };
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == WEDDING_HALL_COPY)
                && !t.field.alive(PRIME_MINISTER).is_empty()
        })
        .await;
        traces.push(t);
    }
    let mut lt = traces.remove(0);
    let prime_minister = lt.field.alive(PRIME_MINISTER)[0];
    let at = lt.field.mob_positions[&prime_minister];
    assert_eq!(at.0, PRIME_MINISTER_AT.0, "{lt:?}");

    // One team at a time on a channel.
    let t = ask_in(&mut alone).await;
    assert!(said(&t, keys::PRIME_MINISTER_BUSY), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");
    send(&mut alone.connection, answer(0, -1, None)).await;

    // Its death wins it.
    let c = &mut team[0].connection;
    beat(c, &mut lt, prime_minister).await;
    until(c, &mut lt, cleared).await;

    // They walk back out to the door; then one with no party goes in
    // alone.
    for player in &mut team {
        let name = player.character_name.clone();
        let c = &mut player.connection;
        step(c, &name, walk_to(LEFT_AT.0, LEFT_AT.1)).await;
        send(c, build_change_map(-1, "left00").unwrap()).await;
        let mut t = Trace::default();
        until(c, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == WEDDING_HALL_DOOR)
        })
        .await;
    }
    let mut t = ask_in(&mut alone).await;
    until(&mut alone.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == WEDDING_HALL_COPY)
            && !t.field.alive(PRIME_MINISTER).is_empty()
    })
    .await;
}
