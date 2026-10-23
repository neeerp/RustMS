//! Horntail himself: in an expedition's copy of Horntail's Cave, six hits
//! break the stone (2401000), which plays his music, brings out his
//! summoning (8810026) and from it the body (8810018) and its eight parts,
//! and restarts the expedition's clock. The parts share the body's HP: a
//! hit on one lowers the body's bar, and a part's death lets out its dead
//! part but leaves the body alive. Once all eight dead parts are out, he
//! dies, then they do; the expedition wins, each player there takes his
//! Victory buff (2022108), and the world reads of it.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_str, set_weapon_attack};
use integration_harness::events::{talk, told};
use integration_harness::hunting::{attack, hit_reactor, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::mobs::{DEAD_HORNTAIL_PARTS, HORNTAIL, HORNTAIL_INTRO, HORNTAIL_PARTS};
use net::packet::op::RecvOpcode;
use net::reactors::{HORNTAIL_MUSIC, HORNTAIL_STONE};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ENTRANCE: i32 = 240050400;
const TRIAL_I: i32 = 240060000;
const TRIAL_II: i32 = 240060100;
const CAVE: i32 = 240060200;
const RECRUITER: i32 = 2083004;
const LEFT_HEAD: i32 = 8810000;
const RIGHT_HEAD: i32 = 8810001;
/// Horntail's tail, the part with the least HP (80,000,000), and the dead
/// tail it leaves.
const TAIL: i32 = 8810009;
const DEAD_TAIL: i32 = 8810017;
/// Under the most a level-100 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;
/// Horntail Squad : Victory, what Horntail leaves those who beat him.
const VICTORY: i32 = 2022108;

/// A new level-100 character hitting very hard, logged in on the Entrance
/// to Horntail's Cave.
async fn hunter() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 100, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, ENTRANCE, "", &mut seen).await, ENTRANCE);
    session
}

/// CHANGE_MAP_SPECIAL: press Up on the trial's door.
fn door() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("next00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// Hit `object_id` until `seen` sees it die, each hit once answered.
async fn beat(c: &mut MapleTestConnection, seen: &mut Trace, object_id: i32) {
    // His left head, the toughest part, takes 98.
    for _ in 0..120 {
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

/// The object id of the `mob` that `seen` saw come out.
fn the(seen: &Trace, mob: i32) -> i32 {
    let found = seen.field.mobs.iter().find(|&&(_, id)| id == mob);
    found.unwrap_or_else(|| panic!("no {mob}: {seen:?}")).0
}

/// Whether `seen` saw the party quests' clear effect.
fn saw_cleared(seen: &Trace) -> bool {
    seen.field_effects
        .iter()
        .any(|(_, path)| path == "quest/party/clear")
}

/// `player` goes through their trial's door, and arrives on `map`.
async fn through(player: &mut WorldSession, map: i32) -> Trace {
    let mut t = Trace::default();
    send(&mut player.connection, door()).await;
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == map)
    })
    .await;
    t
}

/// `player`, on a trial with its head, beats the head and goes through.
async fn clear(player: &mut WorldSession, seen: &mut Trace, head: i32, next: i32) -> Trace {
    let c = &mut player.connection;
    until(c, seen, |t| t.field.mobs.iter().any(|m| m.1 == head)).await;
    let head = the(seen, head);
    beat(c, seen, head).await;
    until(c, seen, saw_cleared).await;
    through(player, next).await
}

/// An expedition of `leader` and `member` beats both heads, and `leader`
/// reaches Horntail's Cave, breaks the stone and brings him out.
async fn summon(leader: &mut WorldSession, member: &mut WorldSession) -> Trace {
    let recruiter = || talk(ENTRANCE, RECRUITER);

    // An expedition sets out, beats both heads and reaches Horntail's Cave.
    told(leader, recruiter()).await;
    told(leader, answer(1, 1, None)).await;
    told(leader, answer(0, 1, None)).await;
    told(member, recruiter()).await;
    told(member, answer(1, 1, None)).await;
    told(member, answer(0, 1, None)).await;
    told(leader, recruiter()).await;
    let mut lt = told(leader, answer(4, 1, Some(1))).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.arrivals.iter().any(|a| a.0 == TRIAL_I)
    })
    .await;
    let mut lt = clear(leader, &mut lt, LEFT_HEAD, TRIAL_II).await;
    let mut lt = clear(leader, &mut lt, RIGHT_HEAD, CAVE).await;
    // A line says the cave closes unless he is summoned in the
    // expedition's time (120 minutes, two in the harness).
    until(&mut leader.connection, &mut lt, |t| {
        !t.cave_closings.is_empty()
    })
    .await;
    let (summoned, minutes) = lt.cave_closings[0];
    assert!(!summoned && (1..=2).contains(&minutes), "{lt:?}");
    until(&mut leader.connection, &mut lt, |t| {
        t.field
            .reactors
            .iter()
            .any(|r| r.reactor_id == HORNTAIL_STONE)
    })
    .await;
    let stone = *lt
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == HORNTAIL_STONE)
        .unwrap();
    assert!(lt.field.mobs.is_empty(), "{lt:?}");

    // The leader walks up to the stone and breaks it with six hits.
    let c = &mut leader.connection;
    send(c, walk_to(stone.x, stone.y)).await;
    read_for(c, &mut lt, 300).await;
    for _ in 0..6 {
        send(c, hit_reactor(stone.object_id)).await;
        read_for(c, &mut lt, 150).await;
    }
    let summoned = net::content::text().get(keys::HORNTAIL_SUMMONED, &[]);
    until(c, &mut lt, |t| {
        t.item_lines.contains(&(summoned.clone(), 0)) && t.field.mobs.len() >= 10
    })
    .await;
    lt
}

// One test, as a channel holds one expedition at a time.
#[tokio::test]
async fn horntails_stone_brings_him_out_and_beating_his_parts_brings_him_down() {
    let mut leader = hunter().await;
    let mut member = hunter().await;
    let mut lt = summon(&mut leader, &mut member).await;
    let c = &mut leader.connection;
    assert!(
        (lt.field_effects.iter()).any(|(mode, name)| *mode == 6 && name == HORNTAIL_MUSIC),
        "{lt:?}"
    );

    // The summoning comes out first, then the body and its eight parts,
    // each named as out of the summoning.
    let intro = the(&lt, HORNTAIL_INTRO);
    let body = the(&lt, HORNTAIL);
    assert_eq!(lt.field.appearances.get(&body), Some(&-3), "{lt:?}");
    for part in HORNTAIL_PARTS {
        let part = the(&lt, part);
        assert!(part > intro, "{lt:?}");
        assert_eq!(lt.field.appearances.get(&part), Some(&-3), "{lt:?}");
    }

    // The expedition's clock starts over, at 60 minutes (one minute in the
    // harness, where event time runs sixty times faster), and the cave now
    // closes in that.
    until(c, &mut lt, |t| {
        t.countdowns.last().is_some_and(|s| (55..=60).contains(s))
            && t.cave_closings.last() == Some(&(true, 1))
    })
    .await;

    // His bar shows full as he comes out, and a hit on the tail lowers it.
    let full = *lt.field.boss_bars.iter().find(|b| b.0 == HORNTAIL).unwrap();
    assert_eq!(full.1, full.2, "{full:?}");
    let tail = the(&lt, TAIL);
    send(c, attack(tail, HIT)).await;
    until(c, &mut lt, |t| {
        t.field
            .boss_bars
            .iter()
            .any(|b| b.0 == HORNTAIL && b.1 < b.2)
    })
    .await;
    let bar = *lt
        .field
        .boss_bars
        .iter()
        .rev()
        .find(|b| b.0 == HORNTAIL)
        .unwrap();

    // Beaten, the tail leaves its dead tail, and the body lives on.
    beat(c, &mut lt, tail).await;
    until(c, &mut lt, |t| {
        t.field.mobs.iter().any(|m| m.1 == DEAD_TAIL)
    })
    .await;
    read_for(c, &mut lt, 300).await;
    assert!(!lt.field.killed.contains(&body), "{lt:?}");
    let last = *lt
        .field
        .boss_bars
        .iter()
        .rev()
        .find(|b| b.0 == HORNTAIL)
        .unwrap();
    assert!(0 < last.1 && last.1 < bar.1, "{last:?} after {bar:?}");

    // The leader beats the other seven parts, and each leaves its dead
    // part; the last of those brings him down, and then them.
    for part in HORNTAIL_PARTS.rev().skip(1) {
        let part = the(&lt, part);
        beat(c, &mut lt, part).await;
    }
    read_for(c, &mut lt, 300).await;
    assert!(!lt.field.killed.contains(&body), "{lt:?}");
    let last_part = the(&lt, *HORNTAIL_PARTS.start());
    beat(c, &mut lt, last_part).await;
    until(c, &mut lt, |t| t.field.killed.contains(&body)).await;
    until(c, &mut lt, |t| {
        let dead = t
            .field
            .mobs
            .iter()
            .filter(|m| DEAD_HORNTAIL_PARTS.contains(&m.1));
        dead.clone().count() == 8 && dead.clone().all(|m| t.field.killed.contains(&m.0))
    })
    .await;
    // He dies before them, and nothing of him is left.
    let killed = |id: &i32| lt.field.killed.iter().position(|k| k == id);
    for dead in lt
        .field
        .mobs
        .iter()
        .filter(|m| DEAD_HORNTAIL_PARTS.contains(&m.1))
    {
        assert!(killed(&body) < killed(&dead.0), "{lt:?}");
    }
    read_for(c, &mut lt, 300).await;
    let left = lt
        .field
        .mobs
        .iter()
        .filter(|m| !lt.field.killed.contains(&m.0));
    assert_eq!(left.count(), 0, "{lt:?}");

    // His expedition wins: the clear shows, the clock stops, and the
    // leader, still alive, takes his Victory buff for an hour.
    until(c, &mut lt, |t| saw_cleared(t) && t.clocks_stopped > 0).await;
    until(c, &mut lt, |t| t.buffs.iter().any(|b| b.0 == -VICTORY)).await;
    let buff = lt.buffs.iter().find(|b| b.0 == -VICTORY).unwrap();
    assert_eq!(buff.2, 3_600_000, "{buff:?}");

    // The member, back on the first trial, reads of it with the world.
    let victory = net::content::text().get(keys::HORNTAIL_VICTORY, &[]);
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.item_lines.contains(&(victory.clone(), 0))
    })
    .await;
    assert!(!mt.buffs.iter().any(|b| b.0 == -VICTORY), "{mt:?}");
}
