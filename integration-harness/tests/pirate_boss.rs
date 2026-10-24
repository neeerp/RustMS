//! The Pirate Ship's last stage and its win: the Lord Pirate's death frees
//! Wu Yang (Cosmic `PiratePQ.monsterKilled`), who sends a member to the
//! leader and, to the leader, wins the ship (`clearPQ`): everyone earns
//! the EXP the chests opened set and goes on to the clear room, where Wu
//! Yang gives a feather and the way out, or a hat for twenty feathers. The
//! harness world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so a
//! stage's 6 minutes are 6 seconds.

use std::time::Duration;

use game_data::server_content::keys;
use integration_harness::events::{
    armed_with, cleared, enter_pirate, maps, party, said, talk, talk_to, throw_down, told,
};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    pirate_win_exp, HPQ_REWARD, PIRATE_CLEAR, PIRATE_HATS, PIRATE_HAT_FEATHERS, PIRATE_LOBBY,
    PIRATE_STAGES,
};
use net::npc_scripts::WU_YANG;
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
async fn the_lord_pirates_fall_frees_wu_yang_who_wins_the_ship() {
    let feathers = (HPQ_REWARD, PIRATE_HAT_FEATHERS);
    let mut team = vec![
        armed_with(PIRATE_LOBBY, 70, &[(KEY, 4)]).await,
        armed_with(PIRATE_LOBBY, 70, &[feathers]).await,
        armed_with(PIRATE_LOBBY, 70, &[]).await,
    ];
    party(&mut team).await;
    enter_pirate(&mut team).await;

    // The leader sets the doors' four seals.
    let doors = PIRATE_STAGES[4];
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, doors, "", &mut seen).await, doors);
    let arrived = tokio::time::Instant::now();
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
    for seal in &seals {
        send(c, walk_to(seal.x, seal.y)).await;
        send(c, throw_down(1, 1)).await;
    }

    // A member on the last stage starts the clock over at 8 minutes, late
    // enough to see the fight and the win through, and waits there.
    tokio::time::sleep_until(arrived + Duration::from_millis(3_000)).await;
    let boss_map = PIRATE_STAGES[5];
    let mut theirs = Trace::default();
    let c = &mut team[2].connection;
    assert_eq!(warp(c, boss_map, "", &mut theirs).await, boss_map);

    // With the seals set, the last monster clears the doors.
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        (seals.iter()).all(|seal| t.field.reactor_hits.contains(&(seal.object_id, 1)))
    })
    .await;
    let mobs: Vec<i32> = (seen.field.mobs.iter())
        .map(|&(oid, _)| oid)
        .filter(|oid| !seen.field.killed.contains(oid))
        .collect();
    for &oid in &mobs {
        send(c, attack(oid, 30_000)).await;
    }
    until(c, &mut seen, |t| {
        mobs.iter().all(|oid| t.field.killed.contains(oid)) && cleared(t)
    })
    .await;

    // On the last stage, the Lord Pirate falls, and Wu Yang comes out with a
    // pink line.
    take(&mut team[0], doors, "next00").await;
    let mut trace = Trace::default();
    let c = &mut team[0].connection;
    until(c, &mut trace, |t| {
        t.warps == [boss_map] && t.field.alive(LORD_PIRATE).len() == 1
    })
    .await;
    let boss = trace.field.alive(LORD_PIRATE)[0];
    for _ in 0..20 {
        send(c, attack(boss, 30_000)).await;
    }
    let freed = net::content::text().get(keys::PIRATE_WU_FREED, &[]);
    let wu_yang = |t: &Trace| {
        (t.field.npcs.iter())
            .find(|&&(_, npc)| npc == WU_YANG)
            .map(|&(oid, _)| oid)
    };
    until(c, &mut trace, |t| {
        t.field.killed.contains(&boss)
            && t.broadcasts.contains(&(PINK, freed.clone()))
            && wu_yang(t).is_some()
    })
    .await;
    let wu = wu_yang(&trace).unwrap();
    let c = &mut team[2].connection;
    until(c, &mut theirs, |t| wu_yang(t) == Some(wu)).await;

    // A member is sent to the leader.
    let t = told(&mut team[2], talk_to(wu)).await;
    assert!(said(&t, keys::PIRATE_WU_WAIT), "{t:?}");
    assert!(t.warps.is_empty(), "{t:?}");

    // The leader's word wins the ship: everyone, on the doors or here, earns
    // the EXP for no chest opened and goes on to the clear room, without the
    // clear's effect.
    let t = told(&mut team[0], talk_to(wu)).await;
    assert_eq!(t.prompts, [(WU_YANG, 0)], "{t:?}");
    assert!(said(&t, keys::PIRATE_WU_THANKS), "{t:?}");
    let mut won = told(&mut team[0], answer(0, 1, None)).await;
    let exp = pirate_win_exp(0);
    for (i, player) in team.iter_mut().enumerate() {
        let mut seen = match i {
            0 => std::mem::take(&mut won),
            2 => std::mem::take(&mut theirs),
            _ => Trace::default(),
        };
        let c = &mut player.connection;
        until(c, &mut seen, |t| {
            t.gains.contains(&exp) && t.warps.contains(&PIRATE_CLEAR)
        })
        .await;
        read_for(c, &mut seen, 200).await;
        assert!(!cleared(&seen), "{seen:?}");
    }

    // In the clear room, Wu Yang makes the first hat for twenty feathers.
    let t = told(&mut team[1], talk(PIRATE_CLEAR, WU_YANG)).await;
    assert_eq!(t.prompts, [(WU_YANG, 4)], "{t:?}");
    let t = told(&mut team[1], answer(4, 1, Some(1))).await;
    assert!(said(&t, keys::PIRATE_WU_HAT_GIVEN), "{t:?}");
    assert!(t.items.contains(&(PIRATE_HATS[0], 1)), "{t:?}");
    assert!(
        t.items
            .contains(&(HPQ_REWARD, -i32::from(PIRATE_HAT_FEATHERS))),
        "{t:?}"
    );

    // Each who asks gets a feather and goes back to Herb Town.
    for player in team.iter_mut() {
        let t = told(player, talk(PIRATE_CLEAR, WU_YANG)).await;
        assert_eq!(t.prompts, [(WU_YANG, 4)], "{t:?}");
        let t = told(player, answer(4, 1, Some(0))).await;
        assert_eq!(t.warps, [PIRATE_LOBBY], "{t:?}");
        assert!(t.items.contains(&(HPQ_REWARD, 1)), "{t:?}");
    }
}
