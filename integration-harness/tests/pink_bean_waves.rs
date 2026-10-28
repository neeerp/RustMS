//! The goddess's guardians in the Twilight of Gods: a while after the
//! first comes into the expedition's copy, the first wave wakes Solomon the
//! Wise out of its statue; each wave after wakes one more, until the fifth
//! wakes all five. Pink Bean's statue takes no harm while they stand. Once
//! the fifth wave is down, the item that calls the lightning falls on the
//! statue's reactor, which takes it a few seconds later: the statue dies,
//! and Pink Bean comes out where it stood. Its fall wins the fight: every
//! mob there goes, the world hears which channel's expedition won, and
//! what it leaves is its own.
//! The harness world runs events
//! 60 times as fast, and lets two set out (`RUSTMS_EXPEDITION_MIN`).

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_str, set_weapon_attack};
use integration_harness::events::{said, talk, told};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{PINK_BEAN_ARENA, PINK_BEAN_GATE, PINK_BEAN_GUARDIANS, PINK_BEAN_LIGHTNING_ITEM};
use net::mobs::{PINK_BEAN, PINK_BEAN_STATUE};
use tokio::time::Duration;

const RECRUITER: i32 = 2141001;
const KEEPER: i32 = 2141002;
const PINK: u8 = 5;
/// Under the most a level-120 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new character of level 120 hitting very hard, logged in at the
/// temple's last gate.
async fn hunter() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 120, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, PINK_BEAN_GATE, "", &mut seen).await, PINK_BEAN_GATE);
    session
}

fn recruiter() -> packet::Packet {
    talk(PINK_BEAN_GATE, RECRUITER)
}

/// The leader gathers a squad that `member` joins, and sets out: both
/// arrive in the Twilight of Gods. Their traces since.
async fn set_out(leader: &mut WorldSession, member: &mut WorldSession) -> (Trace, Trace) {
    told(leader, recruiter()).await;
    let lt = told(leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::SQUAD_OPENED), "{lt:?}");
    told(leader, answer(0, 1, None)).await;
    told(member, recruiter()).await;
    told(member, answer(1, 1, None)).await;
    told(member, answer(0, 1, None)).await;
    told(leader, recruiter()).await;
    let mut lt = told(leader, answer(4, 1, Some(1))).await;
    let mut mt = Trace::default();
    for (player, t) in [(&mut *leader, &mut lt), (&mut *member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            t.arrivals.iter().any(|a| a.0 == PINK_BEAN_ARENA)
        })
        .await;
    }
    (lt, mt)
}

/// Hit `object_id` until `seen` sees it die, five hits at a time.
async fn beat(c: &mut MapleTestConnection, name: &str, seen: &mut Trace, object_id: i32) {
    // Pink Bean, the toughest, takes 420.
    for _ in 0..100 {
        if seen.field.killed.contains(&object_id) {
            return;
        }
        for _ in 0..5 {
            send(c, attack(object_id, HIT)).await;
        }
        barrier(c, name, seen).await;
    }
    panic!("{object_id} lives: {seen:?}");
}

fn line(key: &str) -> (u8, String) {
    (PINK, net::content::text().get(key, &[]))
}

#[tokio::test]
async fn the_guardians_wake_in_waves_and_their_fall_lets_the_lightning_strike_the_statue() {
    let mut leader = hunter().await;
    let mut member = hunter().await;
    let (mut lt, mut mt) = set_out(&mut leader, &mut member).await;
    let name = leader.character_name.clone();

    // Whoever came in first hears the first wave is coming, just after.
    read_for(&mut leader.connection, &mut lt, 500).await;
    read_for(&mut member.connection, &mut mt, 500).await;
    let c = &mut leader.connection;
    let first = line(keys::PINK_BEAN_WAVE_FIRST);
    assert!(
        lt.broadcasts.contains(&first) || mt.broadcasts.contains(&first),
        "{lt:?} {mt:?}"
    );
    let statue = lt.field.alive(PINK_BEAN_STATUE);
    assert_eq!(statue.len(), 1, "{lt:?}");
    let statue = statue[0];

    let next = line(keys::PINK_BEAN_WAVE_NEXT);
    for wave in 1..=PINK_BEAN_GUARDIANS.len() {
        // The wave wakes as many guardians as its number, in order, each
        // out of one of its statues: the newest out of the map's.
        let (woken, asleep) = PINK_BEAN_GUARDIANS.split_at(wave);
        until(c, &mut lt, |t| {
            woken
                .iter()
                .all(|&(_, guardian)| !t.field.alive(guardian).is_empty())
        })
        .await;
        for &(_, guardian) in woken {
            assert_eq!(lt.field.alive(guardian).len(), 1, "{wave}: {lt:?}");
        }
        let newest = woken[wave - 1].0[0];
        assert!(lt.field.alive(newest).is_empty(), "{wave}: {lt:?}");
        for &(statues, guardian) in asleep {
            assert!(lt.field.alive(guardian).is_empty(), "{wave}: {lt:?}");
            assert_eq!(lt.field.alive(statues[0]).len(), 1, "{wave}: {lt:?}");
        }
        // The statue takes no harm while they stand.
        if wave == 1 {
            send(c, attack(statue, HIT)).await;
            barrier(c, &name, &mut lt).await;
            assert!(!lt.field.killed.contains(&statue), "{lt:?}");
        }
        for &(_, guardian) in woken {
            let awake = lt.field.alive(guardian)[0];
            beat(c, &name, &mut lt, awake).await;
        }
        // With none left, the next wave stirs.
        if wave < PINK_BEAN_GUARDIANS.len() {
            until(c, &mut lt, |t| {
                t.broadcasts.iter().filter(|&b| *b == next).count() == wave
            })
            .await;
        }
    }

    // After the last, the item that calls the lightning falls on the
    // statue's reactor, which takes it; the statue dies, and Pink Bean
    // comes out where it stood.
    let last = net::content::text().get(keys::PINK_BEAN_WAVE_LAST, &[]);
    until(c, &mut lt, |t| {
        t.item_lines.iter().any(|(text, _)| *text == last)
            && t.field
                .drops
                .iter()
                .any(|d| d.value == PINK_BEAN_LIGHTNING_ITEM)
    })
    .await;
    // Anyone's, and the reactor, not a player, takes it.
    let drop = *lt
        .field
        .drops
        .iter()
        .find(|d| d.value == PINK_BEAN_LIGHTNING_ITEM)
        .unwrap();
    assert_eq!(drop.ownership, 2, "{lt:?}");
    until(c, &mut lt, |t| !t.field.alive(PINK_BEAN).is_empty()).await;
    assert_eq!(lt.field.removals(drop.object_id), [None], "{lt:?}");
    assert!(lt.field.killed.contains(&statue), "{lt:?}");
    let pink_bean = lt.field.alive(PINK_BEAN)[0];
    let at = |object_id| lt.field.mob_positions.get(&object_id);
    assert_eq!(at(pink_bean), at(statue), "{lt:?}");

    // Pink Bean's fall wins: both see the clear, every mob goes, the world
    // hears the first channel's expedition won, and the keeper knows it.
    let before = lt.field.drops.len();
    beat(c, &name, &mut lt, pink_bean).await;
    let cleared = |t: &Trace| {
        t.field_effects
            .iter()
            .any(|(_, path)| path == "quest/party/clear")
    };
    let victory = net::content::text().get(keys::PINK_BEAN_VICTORY, &[("channel", "1")]);
    until(c, &mut lt, cleared).await;
    until(&mut member.connection, &mut mt, |t| {
        cleared(t) && t.item_lines.contains(&(victory.clone(), 0))
    })
    .await;
    barrier(&mut leader.connection, &name, &mut lt).await;
    let left: Vec<_> = lt
        .field
        .mobs
        .iter()
        .filter(|(oid, _)| !lt.field.killed.contains(oid))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    // What it leaves comes from its own table (or is the harness's card),
    // as its statue and guardians leave nothing.
    let table = game_data::server_content::RewardTable::load(None).unwrap();
    let ours = table.drops(PINK_BEAN);
    for drop in &lt.field.drops[before..] {
        // Its pile of about 45,000 mesos, which ours vary a fifth around.
        let known = if drop.meso {
            (36_000..=54_000).contains(&drop.value)
        } else {
            drop.value / 10_000 == 238 || ours.iter().any(|d| d.item == Some(drop.value))
        };
        assert!(known, "{drop:?}");
    }
    let t = told(&mut leader, talk(PINK_BEAN_ARENA, KEEPER)).await;
    assert!(said(&t, keys::PINK_BEAN_KEEPER_WON), "{t:?}");
}
