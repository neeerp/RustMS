//! The Pirate Ship's fifth stage, its doors: four seals (2519000 to
//! 2519003) each take an Old Metal Key (4001117) dropped on them and stay
//! set, and the spawn points around each spawn no more (Cosmic `2519000`,
//! `setAllowSpawnPointInBox`). With all four set, the last monster's death
//! clears the stage (`PiratePQ.monsterKilled`), and the way on
//! (`davy_next4`) leads to the last stage, where the first member through
//! finds the Lord Pirate: with no chest opened, 9300119. The harness world
//! runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so a stage's 6
//! minutes are 6 seconds.

use game_data::server_content::keys;
use integration_harness::events::{
    armed_in, armed_with, cleared, enter_pirate, maps, party, throw_down,
};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{PIRATE_LOBBY, PIRATE_STAGES};
use net::packet::op::RecvOpcode;
use net::reactors::{PIRATE_LAST_SEAL, PIRATE_SEAL};
use packet::io::write::PktWrite;
use packet::Packet;

/// The Old Metal Keys the seals take.
const KEY: i32 = 4_001_117;
/// The Lord Pirate the doors lead to with no chest opened.
const LORD_PIRATE: i32 = 9_300_119;
const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: walk to `map`'s portal `name` and press Up.
async fn take(player: &mut WorldSession, map: i32, name: &str) {
    let field = maps().field(map).unwrap();
    let portal = field.portal_by_name(name).unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

#[tokio::test]
async fn the_doors_seal_with_their_keys_and_lead_to_the_lord_pirate() {
    let mut team = vec![
        armed_with(PIRATE_LOBBY, 70, &[(KEY, 4)]).await,
        armed_in(PIRATE_LOBBY, 70).await,
        armed_in(PIRATE_LOBBY, 70).await,
    ];
    party(&mut team).await;
    enter_pirate(&mut team).await;
    // A member on the doors starts the clock over at 6 minutes, and waits
    // there.
    let doors = PIRATE_STAGES[4];
    let c = &mut team[1].connection;
    assert_eq!(warp(c, doors, "", &mut Trace::default()).await, doors);
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, doors, "", &mut seen).await, doors);
    until(c, &mut seen, |t| {
        (PIRATE_SEAL..=PIRATE_LAST_SEAL)
            .all(|seal| t.field.reactors.iter().any(|r| r.reactor_id == seal))
            && !t.field.mobs.is_empty()
    })
    .await;
    let seals: Vec<_> = (PIRATE_SEAL..=PIRATE_LAST_SEAL)
        .map(|seal| {
            *(seen.field.reactors.iter())
                .find(|r| r.reactor_id == seal)
                .unwrap()
        })
        .collect();
    assert!(seals.iter().all(|seal| seal.state == 0), "{seals:?}");

    // The leader drops a key on each seal; each takes it 5 seconds later.
    for seal in &seals {
        send(c, walk_to(seal.x, seal.y)).await;
        send(c, throw_down(1, 1)).await;
    }

    // Unsealed till then, the way on says so in pink and stays shut.
    let shut = net::content::text().get(keys::PIRATE_PORTAL_SHUT, &[]);
    let warps = seen.warps.len();
    take(&mut team[0], doors, "next00").await;
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        t.broadcasts.contains(&(PINK, shut.clone()))
    })
    .await;
    read_for(c, &mut seen, 300).await;
    assert_eq!(seen.warps.len(), warps, "{seen:?}");
    assert!(seen.field.reactor_hits.is_empty(), "{seen:?}");

    // A member on the last stage starts the clock over at 8 minutes, and
    // waits there.
    let boss_map = PIRATE_STAGES[5];
    let mut theirs = Trace::default();
    let c = &mut team[2].connection;
    assert_eq!(warp(c, boss_map, "", &mut theirs).await, boss_map);
    assert!(theirs.field.alive(LORD_PIRATE).is_empty());

    // Each seal sets, and stays.
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        (seals.iter()).all(|seal| t.field.reactor_hits.contains(&(seal.object_id, 1)))
    })
    .await;
    assert!(seen.removed.contains(&(4, 1)), "{seen:?}");
    assert!(
        (seals.iter()).all(
            |seal| !(seen.field.reactors_removed.iter()).any(|&(oid, _)| oid == seal.object_id)
        ),
        "{seen:?}"
    );
    assert!(!cleared(&seen), "monsters are left");

    // With the seals set, nothing comes back: the last monster clears the
    // stage.
    let mobs: Vec<i32> = seen.field.mobs.iter().map(|&(oid, _)| oid).collect();
    let mobs: Vec<i32> = mobs
        .into_iter()
        .filter(|oid| !seen.field.killed.contains(oid))
        .collect();
    for &oid in &mobs {
        send(c, attack(oid, 30_000)).await;
    }
    until(c, &mut seen, |t| {
        mobs.iter().all(|oid| t.field.killed.contains(oid)) && cleared(t)
    })
    .await;
    read_for(c, &mut seen, 300).await;
    let left: Vec<_> = (seen.field.mobs.iter())
        .filter(|(oid, _)| !seen.field.killed.contains(oid))
        .collect();
    assert!(left.is_empty(), "{left:?}");

    // The way on leads to the last stage, where the Lord Pirate comes out
    // for the first member through only.
    take(&mut team[0], doors, "next00").await;
    let mut trace = Trace::default();
    let c = &mut team[0].connection;
    until(c, &mut trace, |t| {
        t.warps == [boss_map] && t.field.alive(LORD_PIRATE).len() == 1
    })
    .await;
    let c = &mut team[2].connection;
    until(c, &mut theirs, |t| t.field.alive(LORD_PIRATE).len() == 1).await;
    take(&mut team[1], doors, "next00").await;
    let mut second = Trace::default();
    let c = &mut team[1].connection;
    until(c, &mut second, |t| t.warps == [boss_map]).await;
    // What it saw on the doors, unread till now, is no record of this map.
    second.field = Default::default();
    read_for(c, &mut second, 500).await;
    assert_eq!(second.field.alive(LORD_PIRATE).len(), 1, "{second:?}");
    read_for(&mut team[2].connection, &mut theirs, 100).await;
    assert_eq!(theirs.field.alive(LORD_PIRATE).len(), 1, "{theirs:?}");
}
