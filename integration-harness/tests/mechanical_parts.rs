//! The Missing Mechanical Parts (3239): Cheng the Assistant takes a player
//! on the quest alone into their own copy of the Toy Factory's Sector 4, one
//! at a time on a channel. Its ten parts boxes each drop a Machine Part;
//! Cheng, at the far end, takes ten for a scroll and 2,700 EXP, which
//! completes the quest, and lets the player out to the Secret Passage,
//! whose way out leads back to the factory.

use game_data::server_content::keys;
use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{said, talk, told};
use integration_harness::hunting::{hit_reactor, pickup, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::events::{
    APPARATUS_ROOM, MACHINE_PART, PARTS_PASSAGE, PARTS_QUEST, PARTS_ROOM, TOY_FACTORY,
};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const CHENG: i32 = 2_040_003;
const CHENG_OUTSIDE: (i16, i16) = (-265, -1098);
const CHENG_INSIDE: (i16, i16) = (5281, 162);
const PARTS_BOX: i32 = 2_202_001;
const SCROLLS: [i32; 4] = [2040704, 2040705, 2040707, 2040708];
/// The Secret Passage's way out (`out00`, script `ludi021`).
const PASSAGE_OUT: (i16, i16) = (397, -156);

/// Enter the portal `name`'s script where the player stands.
fn enter(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    p
}

/// A level 30 fighter on The Missing Mechanical Parts, by Cheng.
async fn by_cheng(config: &HarnessConfig) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 30, 50).await.expect("a level 30 fighter");
    start_quest(id, PARTS_QUEST).await.expect("3239 taken");
    let mut session = relogin(config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, TOY_FACTORY, "", &mut seen).await,
        TOY_FACTORY
    );
    told(&mut session, walk_to(CHENG_OUTSIDE.0, CHENG_OUTSIDE.1)).await;
    session
}

/// `player` asks Cheng to go in and says yes.
async fn ask_cheng(player: &mut WorldSession) -> Trace {
    let t = told(player, talk(TOY_FACTORY, CHENG)).await;
    assert!(said(&t, keys::MECHANICAL_PARTS_ENTER), "{t:?}");
    told(player, answer(1, 1, None)).await
}

#[tokio::test]
async fn ten_parts_from_sector_4_for_cheng() {
    let (config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let mut player = by_cheng(&config).await;
    let mut other = by_cheng(&other_config).await;

    // In alone; anyone else waits while Sector 4 is taken.
    let mut seen = ask_cheng(&mut player).await;
    let t = ask_cheng(&mut other).await;
    assert!(said(&t, keys::MECHANICAL_PARTS_BUSY), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");

    // Each of the ten boxes breaks in four hits and drops a part.
    let c = &mut player.connection;
    until(c, &mut seen, |t| t.warps.contains(&PARTS_ROOM)).await;
    let boxes = |t: &Trace| {
        (t.field.reactors.iter())
            .filter(|r| r.reactor_id == PARTS_BOX)
            .map(|r| (r.object_id, r.x, r.y))
            .collect::<Vec<_>>()
    };
    until(c, &mut seen, |t| boxes(t).len() == 10).await;
    let part = |t: &Trace| {
        (t.field.drops.iter())
            .find(|d| {
                d.value == MACHINE_PART && !t.field.removed.iter().any(|r| r.0 == d.object_id)
            })
            .copied()
    };
    for (object_id, x, y) in boxes(&seen) {
        send(c, walk_to(x, y)).await;
        // It takes a hit for each of its states before it breaks.
        for state in 1.. {
            send(c, hit_reactor(object_id)).await;
            until(c, &mut seen, |t| {
                t.field.reactor_hits.contains(&(object_id, state)) || part(t).is_some()
            })
            .await;
            if part(&seen).is_some() {
                break;
            }
        }
        let drop = part(&seen).unwrap();
        send(c, pickup(drop.object_id, drop.x, drop.y)).await;
        until(c, &mut seen, |t| {
            t.field.removed.iter().any(|r| r.0 == drop.object_id)
        })
        .await;
    }

    // The tenth is in the bag before Cheng is asked.
    until(c, &mut seen, |t| {
        t.quantities.last().is_some_and(|q| q.2 == 10)
    })
    .await;

    // Cheng takes the ten for a scroll and 2,700 EXP, then lets them out.
    told(&mut player, walk_to(CHENG_INSIDE.0, CHENG_INSIDE.1)).await;
    let t = told(&mut player, talk(PARTS_ROOM, CHENG)).await;
    assert!(said(&t, keys::MECHANICAL_PARTS_DONE), "{t:?}");
    assert_eq!(t.progress, [(PARTS_QUEST, 2)], "{t:?}");
    assert_eq!(t.gains, [2700], "{t:?}");
    assert!(
        (t.added.iter()).any(|&(_, _, item)| SCROLLS.contains(&item)),
        "{t:?}"
    );
    let t = told(&mut player, answer(0, 1, None)).await;
    assert!(said(&t, keys::MECHANICAL_PARTS_LEAVE), "{t:?}");
    let mut out = Trace::default();
    send(&mut player.connection, answer(1, 1, None)).await;
    until(&mut player.connection, &mut out, |t| {
        t.warps.contains(&PARTS_PASSAGE)
    })
    .await;

    // The Secret Passage's way out, a script portal, leads back to the
    // factory.
    told(&mut player, walk_to(PASSAGE_OUT.0, PASSAGE_OUT.1)).await;
    let mut back = Trace::default();
    send(&mut player.connection, enter("out00")).await;
    until(&mut player.connection, &mut back, |t| {
        t.warps.contains(&APPARATUS_ROOM)
    })
    .await;
}
