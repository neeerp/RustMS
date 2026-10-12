//! Mobs on Dangerous Forest (50000): spawning, control, movement, melee
//! attacks, kills and EXP, touch damage, respawning, and levelling up.

use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{send, warp, QuestTrace};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::collections::BTreeSet;
use std::io::Cursor;
use tokio::time::{timeout, Duration, Instant};

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;

fn attack(object_id: i32, damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CloseRangeAttack as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0x11).unwrap();
    p.write_int(0).unwrap();
    p.write_bytes(&[0; 8]).unwrap();
    for b in [0u8, 0x80, 5, 0, 4] {
        p.write_byte(b).unwrap();
    }
    p.write_bytes(&[0; 4]).unwrap();
    p.write_int(object_id).unwrap();
    p.write_bytes(&[0; 14]).unwrap();
    p.write_int(damage).unwrap();
    p.write_bytes(&[0; 4]).unwrap();
    p
}

fn move_mob(object_id: i32, x: i16, y: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MoveLife as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_short(1).unwrap();
    p.write_bytes(&[0; 19]).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    // One absolute movement to (x + 10, y).
    p.write_byte(1).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(x + 10).unwrap();
    p.write_short(y).unwrap();
    p.write_bytes(&[0; 6]).unwrap();
    p.write_byte(2).unwrap();
    p.write_short(100).unwrap();
    p
}

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

#[derive(Debug, Default)]
struct Seen {
    spawned: Vec<(i32, i32)>,
    controlled: BTreeSet<i32>,
    moved: Vec<i32>,
    move_responses: Vec<i32>,
    hp_bars: Vec<(i32, u8)>,
    killed: Vec<i32>,
    attacks_by: Vec<i32>,
    gains: Vec<i32>,
    /// HP from HP-only stat updates.
    hp: Vec<i16>,
    /// (level, max HP) from level-up stat updates.
    level_ups: Vec<(u8, i16)>,
    level_up_effects: Vec<i32>,
}

fn observe(p: &Packet, seen: &mut Seen) {
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    let op = p.opcode();
    if op == SendOpcode::SpawnMonster as i16 {
        let oid = r.read_int().unwrap();
        r.read_byte().unwrap();
        seen.spawned.push((oid, r.read_int().unwrap()));
    } else if op == SendOpcode::SpawnMonsterControl as i16 {
        if r.read_byte().unwrap() == 1 {
            seen.controlled.insert(r.read_int().unwrap());
        }
    } else if op == SendOpcode::MoveMonster as i16 {
        seen.moved.push(r.read_int().unwrap());
    } else if op == SendOpcode::MoveMonsterResponse as i16 {
        seen.move_responses.push(r.read_int().unwrap());
    } else if op == SendOpcode::ShowMonsterHp as i16 {
        seen.hp_bars
            .push((r.read_int().unwrap(), r.read_byte().unwrap()));
    } else if op == SendOpcode::KillMonster as i16 {
        seen.killed.push(r.read_int().unwrap());
    } else if op == SendOpcode::CloseRangeAttack as i16 {
        seen.attacks_by.push(r.read_int().unwrap());
    } else if op == SendOpcode::ShowStatusInfo as i16 {
        if r.read_byte().unwrap() == 3 {
            r.read_byte().unwrap();
            seen.gains.push(r.read_int().unwrap());
        }
    } else if op == SendOpcode::StatChange as i16 {
        r.read_byte().unwrap();
        let mask = r.read_int().unwrap();
        if mask == 0x400 {
            seen.hp.push(r.read_short().unwrap());
        } else if mask & 0x10 != 0 && mask & 0x800 != 0 {
            // LEVEL comes first in mask order, MAXHP after HP.
            let level = r.read_byte().unwrap();
            r.read_short().unwrap(); // HP
            seen.level_ups.push((level, r.read_short().unwrap()));
        }
    } else if op == SendOpcode::ShowForeignEffect as i16 {
        let cid = r.read_int().unwrap();
        if r.read_byte().unwrap() == 0 {
            seen.level_up_effects.push(cid);
        }
    }
}

/// Read until `done` holds or the deadline passes.
async fn until(
    c: &mut MapleTestConnection,
    seen: &mut Seen,
    secs: u64,
    done: impl Fn(&Seen) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while !done(seen) {
        let left = deadline.saturating_duration_since(Instant::now());
        match timeout(left, c.read_packet("mob combat")).await {
            Ok(Ok(envelope)) => observe(&envelope.packet, seen),
            _ => panic!("timed out; seen so far: {seen:?}"),
        }
    }
}

/// Read whatever arrives within `ms`.
async fn drain(c: &mut MapleTestConnection, seen: &mut Seen, ms: u64) {
    while let Ok(Ok(envelope)) = timeout(Duration::from_millis(ms), c.read_packet("drain")).await {
        observe(&envelope.packet, seen);
    }
}

#[tokio::test]
async fn snails_spawn_move_fight_back_die_and_respawn() {
    let hunter_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut hunter = login_to_world_session(&hunter_config).await.unwrap();
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let mut trace = QuestTrace::default();

    // The first player in gets every snail to control.
    let mut h = Seen::default();
    assert_eq!(
        warp(&mut hunter.connection, FOREST, "", &mut trace).await,
        FOREST
    );
    until(&mut hunter.connection, &mut h, 5, |s| {
        s.controlled.len() >= 17
    })
    .await;
    let snails: Vec<i32> = h
        .spawned
        .iter()
        .filter(|(_, id)| *id == SNAIL)
        .map(|(oid, _)| *oid)
        .collect();
    assert_eq!(snails.len(), 17);

    // The second sees them all but controls none.
    let mut w = Seen::default();
    assert_eq!(
        warp(&mut watcher.connection, FOREST, "", &mut trace).await,
        FOREST
    );
    until(&mut watcher.connection, &mut w, 5, |s| {
        s.spawned.len() >= 17
    })
    .await;
    drain(&mut watcher.connection, &mut w, 300).await;
    assert!(w.controlled.is_empty());

    // Only the controller's movement is relayed.
    let snail = snails[0];
    send(&mut watcher.connection, move_mob(snail, 147, -25)).await;
    send(&mut hunter.connection, move_mob(snail, 147, -25)).await;
    until(&mut hunter.connection, &mut h, 5, |s| {
        s.move_responses.contains(&snail)
    })
    .await;
    until(&mut watcher.connection, &mut w, 5, |s| {
        s.moved.contains(&snail)
    })
    .await;
    assert_eq!(w.moved, [snail]);

    // Two hits of 4 kill an 8 HP snail; the watcher sees the swing and the death.
    send(&mut hunter.connection, attack(snail, 4)).await;
    until(&mut hunter.connection, &mut h, 5, |s| !s.hp_bars.is_empty()).await;
    assert_eq!(h.hp_bars, [(snail, 50)]);
    send(&mut hunter.connection, attack(snail, 4)).await;
    until(&mut hunter.connection, &mut h, 5, |s| {
        s.killed.contains(&snail) && !s.gains.is_empty()
    })
    .await;
    assert_eq!(h.gains, [3]);
    until(&mut watcher.connection, &mut w, 5, |s| {
        s.killed.contains(&snail) && s.attacks_by.len() == 2
    })
    .await;
    assert_eq!(w.attacks_by, [hunter.character_id; 2]);

    // Hitting harder than the character can is refused.
    send(&mut hunter.connection, attack(snails[1], 500)).await;
    drain(&mut hunter.connection, &mut h, 400).await;
    assert!(!h.hp_bars.iter().any(|(oid, _)| *oid == snails[1]));

    // Touching a snail hurts up to its attack (12), no more.
    send(&mut hunter.connection, touched(12, SNAIL)).await;
    until(&mut hunter.connection, &mut h, 5, |s| !s.hp.is_empty()).await;
    let after_touch = h.hp[0];
    send(&mut hunter.connection, touched(13, SNAIL)).await;
    drain(&mut hunter.connection, &mut h, 400).await;
    assert_eq!(h.hp, [after_touch]);

    // Four more snails (15 EXP total) reach level 2, which others see.
    for &snail in &snails[1..5] {
        send(&mut hunter.connection, attack(snail, 8)).await;
    }
    until(&mut hunter.connection, &mut h, 5, |s| {
        !s.level_ups.is_empty()
    })
    .await;
    let (level, max_hp) = h.level_ups[0];
    assert_eq!(level, 2);
    assert!(max_hp > 50);
    until(&mut watcher.connection, &mut w, 5, |s| {
        s.level_up_effects.contains(&hunter.character_id)
    })
    .await;

    // The first snail comes back after the respawn delay.
    let before = h.spawned.len();
    until(&mut hunter.connection, &mut h, 12, |s| {
        s.spawned.len() > before
    })
    .await;
}
