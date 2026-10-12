//! Mob drops: rolling, ownership, pickup into the inventory or meso total,
//! persistence, and quest-only drops.

use integration_harness::connection::MapleTestConnection;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{send, warp, QuestTrace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration, Instant};

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const SMALL_FOREST: i32 = 40_000;
const SENTINEL: i32 = 9300018;

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

#[derive(Debug, Clone, Copy)]
struct SeenDrop {
    object_id: i32,
    meso: bool,
    value: i32,
    x: i16,
    y: i16,
}

#[derive(Debug, Default)]
struct Seen {
    mobs: Vec<(i32, i32, i16, i16)>,
    drops: Vec<SeenDrop>,
    removed: Vec<(i32, Option<i32>)>,
    meso: Option<i32>,
    items_added: Vec<i32>,
    killed: Vec<i32>,
}

fn observe(p: &Packet, seen: &mut Seen) {
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    let op = p.opcode();
    if op == SendOpcode::SpawnMonster as i16 {
        let oid = r.read_int().unwrap();
        r.read_byte().unwrap();
        let id = r.read_int().unwrap();
        r.read_bytes(16).unwrap();
        seen.mobs
            .push((oid, id, r.read_short().unwrap(), r.read_short().unwrap()));
    } else if op == SendOpcode::KillMonster as i16 {
        seen.killed.push(r.read_int().unwrap());
    } else if op == SendOpcode::DropItemFromMapObject as i16 {
        r.read_byte().unwrap();
        let object_id = r.read_int().unwrap();
        let meso = r.read_byte().unwrap() != 0;
        let value = r.read_int().unwrap();
        r.read_int().unwrap();
        r.read_byte().unwrap();
        let (x, y) = (r.read_short().unwrap(), r.read_short().unwrap());
        seen.drops.push(SeenDrop {
            object_id,
            meso,
            value,
            x,
            y,
        });
    } else if op == SendOpcode::RemoveItemFromMap as i16 {
        let mode = r.read_byte().unwrap();
        let oid = r.read_int().unwrap();
        seen.removed
            .push((oid, (mode >= 2).then(|| r.read_int().unwrap())));
    } else if op == SendOpcode::StatChange as i16 {
        r.read_byte().unwrap();
        if r.read_int().unwrap() == 0x40000 {
            seen.meso = Some(r.read_int().unwrap());
        }
    } else if op == SendOpcode::InventoryOperation as i16 {
        r.read_byte().unwrap();
        for _ in 0..r.read_byte().unwrap() {
            let mode = r.read_byte().unwrap();
            r.read_byte().unwrap();
            r.read_short().unwrap();
            if mode == 0 {
                r.read_byte().unwrap();
                seen.items_added.push(r.read_int().unwrap());
                return;
            }
            r.read_short().unwrap();
        }
    }
}

async fn read_for(c: &mut MapleTestConnection, seen: &mut Seen, ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(ms);
    while let Ok(Ok(envelope)) = timeout(
        deadline.saturating_duration_since(Instant::now()),
        c.read_packet("drops"),
    )
    .await
    {
        observe(&envelope.packet, seen);
    }
}

async fn until(c: &mut MapleTestConnection, seen: &mut Seen, done: impl Fn(&Seen) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(seen) {
        let envelope = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("drops"),
        )
        .await
        .unwrap_or_else(|_| panic!("timed out: {seen:?}"))
        .unwrap();
        observe(&envelope.packet, seen);
    }
}

#[tokio::test]
async fn snail_drops_go_to_the_killer_first_and_persist() {
    let hunter_config = load_harness_config_or_fail().await;
    let rival_config = load_harness_config_or_fail().await;
    let mut hunter = login_to_world_session(&hunter_config).await.unwrap();
    let mut rival = login_to_world_session(&rival_config).await.unwrap();
    let mut trace = QuestTrace::default();
    let (mut h, mut r) = (Seen::default(), Seen::default());
    assert_eq!(
        warp(&mut hunter.connection, FOREST, "", &mut trace).await,
        FOREST
    );
    until(&mut hunter.connection, &mut h, |s| s.mobs.len() >= 17).await;
    assert_eq!(
        warp(&mut rival.connection, FOREST, "", &mut trace).await,
        FOREST
    );
    read_for(&mut rival.connection, &mut r, 300).await;
    read_for(&mut hunter.connection, &mut h, 300).await;
    // Earlier tests may have left drops (and pending removals) on this map.
    let old_drops = h.drops.len();

    // Snails drop mesos 65% of the time; kill until something drops.
    let snails: Vec<_> = h.mobs.iter().filter(|m| m.1 == SNAIL).copied().collect();
    for &(oid, _, _, _) in &snails {
        send(&mut hunter.connection, attack(oid, 8)).await;
        until(&mut hunter.connection, &mut h, |s| s.killed.contains(&oid)).await;
        read_for(&mut hunter.connection, &mut h, 200).await;
        if h.drops.len() > old_drops {
            break;
        }
    }
    let loot = *h
        .drops
        .get(old_drops)
        .expect("seventeen snails dropped nothing");
    let ours = |s: &Seen| -> Vec<(i32, Option<i32>)> {
        s.removed
            .iter()
            .filter(|(oid, _)| *oid == loot.object_id)
            .copied()
            .collect()
    };
    until(&mut rival.connection, &mut r, |s| {
        s.drops.iter().any(|d| d.object_id == loot.object_id)
    })
    .await;

    // The rival can't take it yet; picking up from far away fails too.
    send(
        &mut rival.connection,
        pickup(loot.object_id, loot.x, loot.y),
    )
    .await;
    send(
        &mut hunter.connection,
        pickup(loot.object_id, loot.x + 2000, loot.y),
    )
    .await;
    read_for(&mut hunter.connection, &mut h, 400).await;
    assert!(ours(&h).is_empty());

    // The killer takes it; everyone sees it go.
    send(
        &mut hunter.connection,
        pickup(loot.object_id, loot.x, loot.y),
    )
    .await;
    until(&mut hunter.connection, &mut h, |s| !ours(s).is_empty()).await;
    assert_eq!(ours(&h), [(loot.object_id, Some(hunter.character_id))]);
    until(&mut rival.connection, &mut r, |s| !ours(s).is_empty()).await;
    if loot.meso {
        until(&mut hunter.connection, &mut h, |s| s.meso.is_some()).await;
        assert_eq!(h.meso, Some(loot.value));
    } else {
        until(&mut hunter.connection, &mut h, |s| {
            !s.items_added.is_empty()
        })
        .await;
        assert_eq!(h.items_added, [loot.value]);
    }

    // A second pickup of the same drop does nothing.
    send(
        &mut hunter.connection,
        pickup(loot.object_id, loot.x, loot.y),
    )
    .await;
    read_for(&mut hunter.connection, &mut h, 400).await;
    assert_eq!(ours(&h).len(), 1);
    drop(rival);
    drop(hunter);

    // What was picked up was saved with the character.
    let restored = relogin(&hunter_config).await;
    if loot.meso {
        assert_eq!(restored.meso, loot.value);
    }
}

#[tokio::test]
async fn the_shellpiece_only_drops_during_todds_quest() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut trace = QuestTrace::default();
    let mut seen = Seen::default();
    assert_eq!(
        warp(&mut session.connection, SMALL_FOREST, "", &mut trace).await,
        SMALL_FOREST
    );
    until(&mut session.connection, &mut seen, |s| {
        s.mobs.iter().filter(|m| m.1 == SENTINEL).count() >= 3
    })
    .await;
    let sentinel = seen.mobs.iter().find(|m| m.1 == SENTINEL).unwrap().0;
    send(&mut session.connection, attack(sentinel, 8)).await;
    until(&mut session.connection, &mut seen, |s| {
        s.killed.contains(&sentinel)
    })
    .await;
    read_for(&mut session.connection, &mut seen, 500).await;
    // Without quest 1035 started, the sentinel's only drop (the shellpiece
    // override) does not happen.
    assert!(seen.drops.is_empty(), "{:?}", seen.drops);
}
