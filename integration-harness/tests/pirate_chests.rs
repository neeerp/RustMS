//! The Pirate Ship's bellflower rooms, off its decks: each opens with its
//! own Bellflowers and twenty more (Cosmic `PiratePQ`'s `setup`), and its
//! chest (2512001) stays hidden until the leader, with every one beaten,
//! asks Guon (Cosmic `2094002`'s `setReactorState`). Shown, it opens to the
//! Lord Pirate's key (4031437) dropped on it, into a spray of loot (Cosmic
//! `2512001`), and the room's way out leads back to its deck. The harness
//! world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so each
//! stage's 6 minutes are 6 seconds.

use game_data::server_content::keys;
use integration_harness::events::{
    armed_in, armed_with, enter_pirate, party, said, talk, throw_down, told,
};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::packets::build_change_map;
use integration_harness::trace::{send, until, warp, Trace};
use net::events::{PIRATE_LOBBY, PIRATE_STAGES};
use net::npc_scripts::GUON_ABOARD;
use net::reactors::PIRATE_CHEST;

/// The Lord Pirate's key, the one thing the chest takes.
const KEY: i32 = 4_031_437;
/// The room's two kinds of Bellflower, with how many of each it opens
/// with: its own (Map.nx, mobTime -1), and ten more.
const BELLFLOWERS: [(i32, usize); 2] = [(9300112, 33 + 10), (9300113, 15 + 10)];

#[tokio::test]
async fn a_bellflower_rooms_chest_shows_once_it_is_bare_and_opens_to_the_key() {
    let mut team = vec![
        armed_with(PIRATE_LOBBY, 70, &[(KEY, 1)]).await,
        armed_in(PIRATE_LOBBY, 70).await,
        armed_in(PIRATE_LOBBY, 70).await,
    ];
    party(&mut team).await;
    enter_pirate(&mut team).await;
    // A member on the deck starts the clock over at 6 minutes, for more
    // room; the leader goes on to the deck's bellflower room.
    let deck = PIRATE_STAGES[2];
    let c = &mut team[1].connection;
    assert_eq!(warp(c, deck, "", &mut Trace::default()).await, deck);
    let room = deck + 1;
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, room, "", &mut seen).await, room);
    until(c, &mut seen, |t| {
        BELLFLOWERS
            .iter()
            .all(|&(mob_id, n)| t.field.alive(mob_id).len() == n)
            && t.field
                .reactors
                .iter()
                .any(|r| r.reactor_id == PIRATE_CHEST)
    })
    .await;
    let chest = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == PIRATE_CHEST)
        .unwrap();
    assert_eq!(chest.state, 0, "hidden");

    // While any Bellflower is left, Guon only says what the room asks.
    let asked = told(&mut team[0], talk(room, GUON_ABOARD)).await;
    assert!(said(&asked, keys::PIRATE_GUON_BONUS), "{asked:?}");

    // The leader beats them all at once, as the clock is short.
    let c = &mut team[0].connection;
    let mobs: Vec<i32> = (BELLFLOWERS.iter())
        .flat_map(|&(mob_id, _)| seen.field.alive(mob_id))
        .collect();
    for &oid in &mobs {
        send(c, attack(oid, 30_000)).await;
    }
    until(c, &mut seen, |t| {
        mobs.iter().all(|oid| t.field.killed.contains(oid))
    })
    .await;
    assert!(!seen
        .field
        .reactor_hits
        .iter()
        .any(|&(oid, _)| oid == chest.object_id));

    // Another member reaching the next deck gives 6 more seconds; now
    // Guon shows the chest.
    let c = &mut team[2].connection;
    let next = PIRATE_STAGES[3];
    assert_eq!(warp(c, next, "", &mut Trace::default()).await, next);
    let shown = told(&mut team[0], talk(room, GUON_ABOARD)).await;
    assert!(said(&shown, keys::PIRATE_GUON_CHEST), "{shown:?}");
    let shows = (chest.object_id, 1);
    if !shown.field.reactor_hits.contains(&shows) {
        let c = &mut team[0].connection;
        until(c, &mut seen, |t| t.field.reactor_hits.contains(&shows)).await;
    }

    // And another, on the stage after, 6 more. The key on the chest opens
    // it into at least fifteen drops, one of them always the pack's quest
    // item (4031551), which only those on its quest see.
    let c = &mut team[1].connection;
    let next = PIRATE_STAGES[4];
    assert_eq!(warp(c, next, "", &mut Trace::default()).await, next);
    let c = &mut team[0].connection;
    send(c, walk_to(chest.x - 20, chest.y)).await;
    send(c, throw_down(1, 1)).await;
    until(c, &mut seen, |t| {
        let after_key = (t.field.drops.iter())
            .skip_while(|d| d.meso || d.value != KEY)
            .skip(1);
        (t.field.reactors_removed.iter()).any(|&(oid, _)| oid == chest.object_id)
            && after_key.count() >= 15 - 1
    })
    .await;
    assert!(seen.removed.contains(&(4, 1)), "{seen:?}");

    // The room's way out leads back to its deck.
    send(c, build_change_map(-1, "out00").unwrap()).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [deck], "{trace:?}");
}
