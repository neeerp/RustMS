//! Dollhouse (3230): Olson the Toy Soldier takes a player on the quest
//! alone into their own copy of Eos Tower's dollhouse room, one at a time on
//! a channel. A wrong dollhouse throws them back out; the odd one drops the
//! pendulum, which Mark the Toy Soldier takes to complete the quest before
//! sending them back.

use game_data::server_content::keys;
use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{said, talk, told};
use integration_harness::hunting::{hit_reactor, pickup, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::events::{DOLLHOUSE_PENDULUM, DOLLHOUSE_QUEST, DOLLHOUSE_ROOM, EOS_TOWER_100};
use net::reactors::DOLLHOUSE;
use std::time::Duration;

const OLSON: i32 = 2_040_002;
const OLSON_AT: (i16, i16) = (-99, 643);
const MARK: i32 = 2_040_028;
const MARK_AT: (i16, i16) = (-106, 358);
const ODD_DOLLHOUSE: i32 = 2_202_000;

/// A level 30 fighter on Dollhouse, by Olson.
async fn by_olson(config: &HarnessConfig) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 30, 50).await.expect("a level 30 fighter");
    start_quest(id, DOLLHOUSE_QUEST).await.expect("3230 taken");
    let mut session = relogin(config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, EOS_TOWER_100, "", &mut seen).await,
        EOS_TOWER_100
    );
    told(&mut session, walk_to(OLSON_AT.0, OLSON_AT.1)).await;
    session
}

/// `player` asks Olson to go in: the hint, then yes.
async fn ask_olson(player: &mut WorldSession) -> Trace {
    let t = told(player, talk(EOS_TOWER_100, OLSON)).await;
    assert!(said(&t, keys::DOLLHOUSE_HINT), "{t:?}");
    let t = told(player, answer(0, 1, None)).await;
    assert!(said(&t, keys::DOLLHOUSE_ENTER), "{t:?}");
    told(player, answer(1, 1, None)).await
}

/// The first reactor `reactor_id` on the player's copy of the room.
async fn find(player: &mut WorldSession, seen: &mut Trace, reactor_id: i32) -> (i32, i16, i16) {
    let c = &mut player.connection;
    until(c, seen, |t| t.warps.contains(&DOLLHOUSE_ROOM)).await;
    let on_map = |t: &Trace| {
        (t.field.reactors.iter())
            .find(|r| r.reactor_id == reactor_id)
            .map(|r| (r.object_id, r.x, r.y))
    };
    until(c, seen, |t| on_map(t).is_some()).await;
    on_map(seen).unwrap()
}

#[tokio::test]
async fn the_odd_dollhouse_holds_the_pendulum_mark_takes() {
    let (config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let mut player = by_olson(&config).await;
    let mut other = by_olson(&other_config).await;

    // In alone; anyone else waits while the room is taken.
    let mut seen = ask_olson(&mut player).await;
    let t = ask_olson(&mut other).await;
    assert!(said(&t, keys::DOLLHOUSE_BUSY), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");

    // A wrong dollhouse throws the player out by Olson, with a pink line.
    let (wrong, x, y) = find(&mut player, &mut seen, DOLLHOUSE).await;
    let c = &mut player.connection;
    send(c, walk_to(x, y)).await;
    send(c, hit_reactor(wrong)).await;
    let line = net::content::text().get(keys::DOLLHOUSE_WRONG, &[]);
    let mut out = Trace::default();
    until(c, &mut out, |t| {
        t.warps.contains(&EOS_TOWER_100) && t.broadcasts.contains(&(5, line.clone()))
    })
    .await;

    // Once the room is free again, the odd one drops the pendulum.
    tokio::time::sleep(Duration::from_secs(3)).await;
    told(&mut player, walk_to(OLSON_AT.0, OLSON_AT.1)).await;
    let mut seen = ask_olson(&mut player).await;
    let (odd, x, y) = find(&mut player, &mut seen, ODD_DOLLHOUSE).await;
    let c = &mut player.connection;
    send(c, walk_to(x, y)).await;
    // It takes a hit for each of its states before it breaks.
    for state in 1.. {
        send(c, hit_reactor(odd)).await;
        until(c, &mut seen, |t| {
            t.field.reactor_hits.contains(&(odd, state))
                || t.field.item_drop(DOLLHOUSE_PENDULUM).is_some()
        })
        .await;
        if seen.field.item_drop(DOLLHOUSE_PENDULUM).is_some() {
            break;
        }
    }
    let pendulum = seen.field.item_drop(DOLLHOUSE_PENDULUM).unwrap();
    send(c, pickup(pendulum.object_id, pendulum.x, pendulum.y)).await;
    until(c, &mut seen, |t| {
        t.added
            .iter()
            .any(|&(_, _, item)| item == DOLLHOUSE_PENDULUM)
    })
    .await;

    // Mark takes it, which completes the quest, and sends the player back.
    told(&mut player, walk_to(MARK_AT.0, MARK_AT.1)).await;
    let t = told(&mut player, talk(DOLLHOUSE_ROOM, MARK)).await;
    assert!(said(&t, keys::DOLLHOUSE_FOUND), "{t:?}");
    assert_eq!(t.progress, [(DOLLHOUSE_QUEST, 2)], "{t:?}");
    let t = told(&mut player, answer(0, 1, None)).await;
    assert!(said(&t, keys::DOLLHOUSE_BACK), "{t:?}");
    let mut back = Trace::default();
    send(&mut player.connection, answer(1, 1, None)).await;
    until(&mut player.connection, &mut back, |t| {
        t.warps.contains(&EOS_TOWER_100)
    })
    .await;
}
