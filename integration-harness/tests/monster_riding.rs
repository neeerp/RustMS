//! Monster Riding (6002): Kenta in the Aquarium's zoo sends a third-job
//! player on the quest alone into the Watch Hog's room for five minutes
//! (5 seconds here, `RUSTMS_EVENT_SCALE`). The Watch Hog drops Pheromones
//! and the Watch Hog's room's boxes Kenta's Reports, for those on the quest;
//! Kenta's double in the room lets a player out, and once the time is up
//! the room puts them out, taking both. The way out of the room's exit
//! leads back to the zoo.

use game_data::server_content::keys;
use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{said, talk, told};
use integration_harness::hunting::{hit_reactor, pickup, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    AQUARIUM_ZOO, HOG_ROOM, HOG_ROOM_EXIT, KENTAS_REPORT, MONSTER_RIDING_QUEST, PHEROMONE,
};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const KENTA: i32 = 2_060_005;
const KENTA_AT: (i16, i16) = (0, 22);
/// Kenta's double in the Watch Hog's room.
const KENTA_INSIDE: i32 = 9_060_000;
const KENTA_INSIDE_AT: (i16, i16) = (224, -54);
const HOG_BOX: i32 = 2_302_005;
/// The room's exit's way out (`out00`, script `tamepig_out2`).
const EXIT_OUT: (i16, i16) = (173, 675);
const ETC: u8 = 4;

/// Enter the portal `name`'s script where the player stands.
fn enter(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    p
}

/// A level 70 Crusader on Monster Riding, sent in by Kenta.
async fn in_the_hogs_room() -> (WorldSession, Trace) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 111, 70, 50).await.expect("a level 70 Crusader");
    start_quest(id, MONSTER_RIDING_QUEST)
        .await
        .expect("6002 taken");
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, AQUARIUM_ZOO, "", &mut seen).await,
        AQUARIUM_ZOO
    );
    told(&mut session, walk_to(KENTA_AT.0, KENTA_AT.1)).await;
    let mut seen = Trace::default();
    send(&mut session.connection, talk(AQUARIUM_ZOO, KENTA)).await;
    until(&mut session.connection, &mut seen, |t| {
        t.warps.contains(&HOG_ROOM)
    })
    .await;
    (session, seen)
}

/// The way out of the room's exit leads back to the zoo.
async fn back_to_the_zoo(player: &mut WorldSession) {
    told(player, walk_to(EXIT_OUT.0, EXIT_OUT.1)).await;
    let mut back = Trace::default();
    send(&mut player.connection, enter("out00")).await;
    until(&mut player.connection, &mut back, |t| {
        t.warps.contains(&AQUARIUM_ZOO)
    })
    .await;
}

#[tokio::test]
async fn the_watch_hog_and_its_boxes_drop_for_6002_until_the_time_is_up() {
    let (mut player, mut seen) = in_the_hogs_room().await;
    let c = &mut player.connection;
    let lying = |t: &Trace, item: i32| {
        (t.field.drops.iter())
            .find(|d| !d.meso && d.value == item && t.field.removals(d.object_id).is_empty())
            .copied()
    };

    // The Watch Hog drops a Pheromone.
    until(c, &mut seen, |t| lying(t, PHEROMONE).is_some()).await;
    let drop = lying(&seen, PHEROMONE).unwrap();
    send(c, walk_to(drop.x, drop.y)).await;
    send(c, pickup(drop.object_id, drop.x, drop.y)).await;
    until(c, &mut seen, |t| t.added.iter().any(|a| a.2 == PHEROMONE)).await;

    // A box breaks into one of Kenta's Reports.
    until(c, &mut seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == HOG_BOX)
    })
    .await;
    let hog_box = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == HOG_BOX)
        .unwrap();
    send(c, walk_to(hog_box.x, hog_box.y)).await;
    for state in 1.. {
        send(c, hit_reactor(hog_box.object_id)).await;
        until(c, &mut seen, |t| {
            t.field.reactor_hits.contains(&(hog_box.object_id, state))
                || lying(t, KENTAS_REPORT).is_some()
        })
        .await;
        if lying(&seen, KENTAS_REPORT).is_some() {
            break;
        }
    }
    let drop = lying(&seen, KENTAS_REPORT).unwrap();
    send(c, pickup(drop.object_id, drop.x, drop.y)).await;
    until(c, &mut seen, |t| {
        t.added.iter().any(|a| a.2 == KENTAS_REPORT)
    })
    .await;

    // The time runs out with too few: the room puts the player out to its
    // exit and takes both.
    until(c, &mut seen, |t| {
        t.warps.contains(&HOG_ROOM_EXIT) && t.removed.iter().filter(|r| r.0 == ETC).count() == 2
    })
    .await;
    back_to_the_zoo(&mut player).await;
}

#[tokio::test]
async fn kentas_double_lets_a_player_out_of_the_watch_hogs_room() {
    let (mut player, _) = in_the_hogs_room().await;
    told(&mut player, walk_to(KENTA_INSIDE_AT.0, KENTA_INSIDE_AT.1)).await;
    let t = told(&mut player, talk(HOG_ROOM, KENTA_INSIDE)).await;
    assert!(said(&t, keys::KENTA_HOG_LEAVE), "{t:?}");
    let t = told(&mut player, answer(1, 1, None)).await;
    assert!(said(&t, keys::KENTA_HOG_BACK), "{t:?}");
    let mut out = Trace::default();
    send(&mut player.connection, answer(0, 1, None)).await;
    until(&mut player.connection, &mut out, |t| {
        t.warps.contains(&HOG_ROOM_EXIT)
    })
    .await;
    back_to_the_zoo(&mut player).await;
}
