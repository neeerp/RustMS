//! Quests with item and mob objectives and item rewards along the Maple
//! Island route: 1034 (Nina's candy for Sen), 1035 (Todd's hunt), and
//! 1037/1038 (Sam's snails, Maria, and Lucas).

use integration_harness::connection::MapleTestConnection;
use integration_harness::packets::build_player_move;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, barrier, send, warp, QuestTrace, COMPLETE, START};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration, Instant};

const NINA: i32 = 2102;
const SEN: i32 = 2001;
const TODD: i32 = 2004;
const PETER: i32 = 2002;
const SAM: i32 = 2005;
const MARIA: i32 = 2103;
const LUCAS: i32 = 12000;
const SENTINEL: i32 = 9300018;
const SNAIL: i32 = 100100;
const CANDY: i32 = 4031792;
const SHELLPIECE: i32 = 4031802;
const LETTER: i32 = 4031800;
const BEGINNER_HATS: [i32; 5] = [1002053, 1002008, 1002014, 1002068, 1002069];

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

/// One absolute movement to (x, y), standing.
fn walk_to(x: i16, y: i16) -> Packet {
    let mut movement = vec![1u8, 0];
    movement.extend(x.to_le_bytes());
    movement.extend(y.to_le_bytes());
    movement.extend([0; 6]);
    movement.push(4);
    movement.extend(100i16.to_le_bytes());
    build_player_move(&movement).unwrap()
}

fn pickup(object_id: i32, x: i16, y: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemPickup as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_int(object_id).unwrap();
    p
}

/// Mobs, kills, and drops seen on the field, next to the quest trace.
#[derive(Debug, Default)]
struct Field {
    mobs: Vec<(i32, i32)>,
    killed: Vec<i32>,
    /// (object id, item id, x, y)
    drops: Vec<(i32, i32, i16, i16)>,
    picked_up: Vec<i32>,
}

impl Field {
    fn observe(&mut self, p: &Packet) {
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        let op = p.opcode();
        if op == SendOpcode::SpawnMonster as i16 {
            let oid = r.read_int().unwrap();
            r.read_byte().unwrap();
            self.mobs.push((oid, r.read_int().unwrap()));
        } else if op == SendOpcode::KillMonster as i16 {
            self.killed.push(r.read_int().unwrap());
        } else if op == SendOpcode::DropItemFromMapObject as i16 {
            r.read_byte().unwrap();
            let oid = r.read_int().unwrap();
            if r.read_byte().unwrap() == 0 {
                let item = r.read_int().unwrap();
                r.read_int().unwrap();
                r.read_byte().unwrap();
                self.drops
                    .push((oid, item, r.read_short().unwrap(), r.read_short().unwrap()));
            }
        } else if op == SendOpcode::RemoveItemFromMap as i16 {
            r.read_byte().unwrap();
            self.picked_up.push(r.read_int().unwrap());
        }
    }

    fn alive(&self, mob_id: i32) -> Vec<i32> {
        self.mobs
            .iter()
            .filter(|(oid, id)| *id == mob_id && !self.killed.contains(oid))
            .map(|(oid, _)| *oid)
            .collect()
    }
}

/// Read packets into both records until `done`.
async fn until(
    c: &mut MapleTestConnection,
    field: &mut Field,
    trace: &mut QuestTrace,
    done: impl Fn(&Field, &QuestTrace) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !done(field, trace) {
        let envelope = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("quest objectives"),
        )
        .await
        .unwrap_or_else(|_| panic!("timed out: {field:?} {trace:?}"))
        .unwrap();
        field.observe(&envelope.packet);
        trace.observe(&envelope.packet);
    }
}

/// Warp somewhere and wait for `count` of `mob_id` to show up.
async fn enter_hunting_ground(
    c: &mut MapleTestConnection,
    trace: &mut QuestTrace,
    map: i32,
    mob_id: i32,
    count: usize,
) -> Field {
    assert_eq!(warp(c, map, "", trace).await, map);
    let mut field = Field::default();
    until(c, &mut field, trace, |f, _| f.alive(mob_id).len() >= count).await;
    field
}

/// Kill one `mob_id` and wait for its kill to be counted.
async fn kill_one(
    c: &mut MapleTestConnection,
    field: &mut Field,
    trace: &mut QuestTrace,
    mob_id: i32,
) {
    let target = field.alive(mob_id)[0];
    send(c, attack(target, 8)).await;
    until(c, field, trace, |f, _| f.killed.contains(&target)).await;
}

#[tokio::test]
async fn nina_gives_candy_that_sen_shares_with_beginners() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = QuestTrace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;

    // 1034 needs Sen's own errand (1033) done first.
    assert_eq!(warp(c, 30_000, "", &mut trace).await, 30_000);
    act(c, &mut trace, &m, START, 1032, NINA).await;
    assert_eq!(warp(c, 30_001, "", &mut trace).await, 30_001);
    act(c, &mut trace, &m, COMPLETE, 1032, SEN).await;
    act(c, &mut trace, &m, START, 1033, SEN).await;
    assert_eq!(warp(c, 30_000, "", &mut trace).await, 30_000);
    act(c, &mut trace, &m, COMPLETE, 1033, NINA).await;
    // Act/1033/1/nextQuest continues straight into Nina's candy errand.
    assert_eq!(trace.results.last(), Some(&(1033, NINA, 1034)));

    // Accepting hands over the ten candies (Act/1034/0/item).
    act(c, &mut trace, &m, START, 1034, NINA).await;
    assert_eq!(trace.states.get(&1034), Some(&1));
    assert_eq!(trace.items, [(CANDY, 10)]);

    assert_eq!(warp(c, 30_001, "", &mut trace).await, 30_001);
    act(c, &mut trace, &m, COMPLETE, 1034, SEN).await;
    assert_eq!(trace.states.get(&1034), Some(&2));
    // Sen's candies go; two edible ones (job-filtered: beginners) come.
    assert_eq!(&trace.items[1..], [(CANDY, -10), (2022253, 2)]);
    assert_eq!(trace.gains.last(), Some(&20));
    // 1032, 1033, and 1034 each played the "Quest Clear" effect.
    assert_eq!(trace.cleared, 3);
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&1034), Some(&2));
    assert_eq!(restored.items.get(&CANDY), None);
    assert_eq!(restored.items.get(&2022253), Some(&2));
}

#[tokio::test]
async fn todds_hunt_needs_the_kill_and_the_shellpiece() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = QuestTrace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut field = enter_hunting_ground(c, &mut trace, 40_000, SENTINEL, 1).await;

    act(c, &mut trace, &m, START, 1035, TODD).await;
    assert_eq!(trace.states.get(&1035), Some(&1));
    // One Jr. Sentinel to hunt, none yet.
    assert_eq!(trace.kills.get(&1035).map(String::as_str), Some("000"));

    // Peter won't finish it before the hunt.
    act(c, &mut trace, &m, COMPLETE, 1035, PETER).await;
    assert_eq!(trace.states.get(&1035), Some(&1));
    assert!(trace.results.iter().all(|r| r.0 != 1035 || r.1 != PETER));

    kill_one(c, &mut field, &mut trace, SENTINEL).await;
    until(c, &mut field, &mut trace, |f, t| {
        t.kills.get(&1035).map(String::as_str) == Some("001")
            && f.drops.iter().any(|d| d.1 == SHELLPIECE)
    })
    .await;
    // The kill alone isn't enough: the shellpiece must be picked up.
    act(c, &mut trace, &m, COMPLETE, 1035, PETER).await;
    assert_eq!(trace.states.get(&1035), Some(&1));

    let (oid, _, x, y) = *field.drops.iter().find(|d| d.1 == SHELLPIECE).unwrap();
    send(c, pickup(oid, x, y)).await;
    until(c, &mut field, &mut trace, |f, _| f.picked_up.contains(&oid)).await;
    act(c, &mut trace, &m, COMPLETE, 1035, PETER).await;
    assert_eq!(trace.states.get(&1035), Some(&2));
    assert_eq!(trace.gains.last(), Some(&30));
    // One random beginner hat (prop), and the shellpiece is taken.
    let hats: Vec<_> = trace
        .items
        .iter()
        .filter(|(id, count)| BEGINNER_HATS.contains(id) && *count == 1)
        .collect();
    assert_eq!(hats.len(), 1, "{:?}", trace.items);
    assert!(trace.items.contains(&(SHELLPIECE, -1)));
    assert_eq!(trace.cleared, 1);
    let hat = hats[0].0;
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&1035), Some(&2));
    assert_eq!(restored.items.get(&hat), Some(&1));
    assert_eq!(restored.items.get(&SHELLPIECE), None);
}

#[tokio::test]
async fn snail_kills_persist_and_lead_on_to_lucas() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = QuestTrace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut field = enter_hunting_ground(c, &mut trace, 50_000, SNAIL, 17).await;
    act(c, &mut trace, &m, START, 1037, SAM).await;
    assert_eq!(trace.kills.get(&1037).map(String::as_str), Some("000"));
    for count in 1..=4 {
        kill_one(c, &mut field, &mut trace, SNAIL).await;
        let expected = format!("{count:03}");
        until(c, &mut field, &mut trace, |_, t| {
            t.kills.get(&1037) == Some(&expected)
        })
        .await;
    }
    drop(session);

    // The kill count survives a relog.
    let mut session = relogin(&config).await;
    assert_eq!(session.quests.get(&1037), Some(&1));
    assert_eq!(
        session.quest_progress.get(&1037).map(String::as_str),
        Some("004")
    );
    let c = &mut session.connection;
    assert_eq!(warp(c, 40_000, "", &mut trace).await, 40_000);
    let mut field = enter_hunting_ground(c, &mut trace, 50_000, SNAIL, 6).await;
    for _ in 5..=11 {
        if field.alive(SNAIL).is_empty() {
            until(c, &mut field, &mut trace, |f, _| !f.alive(SNAIL).is_empty()).await;
        }
        kill_one(c, &mut field, &mut trace, SNAIL).await;
    }
    barrier(c, &m, &mut trace).await;
    // The eleventh kill isn't counted past the ten needed.
    assert_eq!(trace.kills.get(&1037).map(String::as_str), Some("010"));

    assert_eq!(warp(c, 1_000_000, "", &mut trace).await, 1_000_000);
    act(c, &mut trace, &m, COMPLETE, 1037, MARIA).await;
    assert_eq!(trace.states.get(&1037), Some(&2));
    assert_eq!(trace.gains.last(), Some(&60));
    assert_eq!(trace.results.last(), Some(&(1037, MARIA, 1038)));

    // Maria's letter for Lucas, then potions for delivering it.
    act(c, &mut trace, &m, START, 1038, MARIA).await;
    assert_eq!(trace.items.last(), Some(&(LETTER, 1)));
    // Lucas stands across town, out of reach from the spawn point.
    act(c, &mut trace, &m, COMPLETE, 1038, LUCAS).await;
    assert_eq!(trace.states.get(&1038), Some(&1));
    send(c, walk_to(1500, 154)).await;
    act(c, &mut trace, &m, COMPLETE, 1038, LUCAS).await;
    assert_eq!(trace.states.get(&1038), Some(&2));
    assert!(trace
        .items
        .ends_with(&[(2000000, 5), (2000003, 5), (LETTER, -1)]));
    assert_eq!(trace.gains.last(), Some(&40));
}
