//! Pianus in the Cave of Pianus (230040420): two spawn points, one at
//! either end of the cave, each with its own boss bar. Neither ever turns
//! (`noFlip`), so the one on the left seals the players in front of him and
//! calls up his Bloody Booms there, whatever stance his controller reports.
//! The one on the right falls like any boss, his drops anyone's.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_str, set_weapon_attack};
use integration_harness::hunting::{attack, move_mob};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use packet::Packet;
use tokio::time::Duration;

const CAVE: i32 = 230_040_420;
/// The Pianus at the right end of the cave, and the one at the left.
const RIGHT: i32 = 8_510_000;
const LEFT: i32 = 8_520_000;
const BLOODY_BOOM: i32 = 8_510_100;
const PIANUS_CERTIFICATE: i32 = 4_001_085;
const SEAL: u8 = 120;
const SUMMON: u8 = 200;
/// The left Pianus's skills' animations (Mob.nx `info/skill/<n>/action`).
const ACTIONS: [(u8, u8); 6] = [
    (114, 3),
    (SUMMON, 1),
    (127, 2),
    (140, 4),
    (141, 4),
    (SEAL, 3),
];
/// Under the most a level-50 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new level-50 character hitting very hard, logged in on the cave.
async fn hunter() -> (WorldSession, Trace) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, CAVE, "", &mut seen).await,
        CAVE
    );
    (session, seen)
}

/// The movement casting the `skill` at `level` with the mob's `nth` skill
/// animation: the helper's stance faces right.
fn cast(object_id: i32, at: (i16, i16), nth: u8, skill: u8, level: u8) -> Packet {
    let data = i32::from(skill) | i32::from(level) << 8;
    move_mob(object_id, at, false, (20 + nth) << 1, data)
}

/// Have `pianus`, controlled, cast whatever skills he is granted until he
/// has both sealed his controller and called up his Booms.
async fn seal_and_booms(c: &mut MapleTestConnection, t: &mut Trace, pianus: i32) {
    let at = t.field.mob_positions[&pianus];
    let (mut sealed, mut summoned) = (false, false);
    for _ in 0..40 {
        if sealed && summoned {
            return;
        }
        t.field.grants.clear();
        send(c, move_mob(pianus, at, true, 0, 0)).await;
        read_for(c, t, 1_000).await;
        let Some(&(_, skill, level)) = t.field.grants.first() else {
            continue;
        };
        let nth = ACTIONS.iter().find(|a| a.0 == skill).unwrap().1;
        let booms = t.field.alive(BLOODY_BOOM).len();
        send(c, cast(pianus, at, nth, skill, level)).await;
        if skill == SEAL {
            let seal = i32::from(SEAL) | i32::from(level) << 16;
            until(c, t, |t| t.buffs.iter().any(|b| b.0 == seal)).await;
            sealed = true;
        }
        if skill == SUMMON {
            until(c, t, |t| t.field.alive(BLOODY_BOOM).len() >= booms + 5).await;
            summoned = true;
        }
    }
    panic!("no seal and summon: {t:?}");
}

#[tokio::test]
async fn pianus_never_turns_and_falls_like_a_boss() {
    let (mut session, mut t) = hunter().await;
    let c = &mut session.connection;
    until(c, &mut t, |t| {
        [RIGHT, LEFT]
            .iter()
            .all(|&id| !t.field.alive(id).is_empty())
    })
    .await;
    let (right, left) = (t.field.alive(RIGHT)[0], t.field.alive(LEFT)[0]);
    assert_eq!(t.field.mob_positions[&right].0, 568, "{t:?}");
    assert_eq!(t.field.mob_positions[&left].0, -459, "{t:?}");

    // Each has his own bar.
    send(c, attack(left, 1_000_000)).await;
    until(c, &mut t, |t| {
        t.field.controlled.contains(&left) && !t.field.boss_bars.is_empty()
    })
    .await;
    assert_eq!(t.field.boss_bars[0], (LEFT, 23_000_000, 24_000_000, (2, 5)));
    send(c, attack(right, 1_000_000)).await;
    until(c, &mut t, |t| t.field.boss_bars.len() > 1).await;
    assert_eq!(
        t.field.boss_bars[1],
        (RIGHT, 29_000_000, 30_000_000, (1, 5))
    );

    // Turned to the right by his controller, the left Pianus still seals
    // and calls up his Booms ahead of him, as he faces in the data: his
    // areas run right from him (x 100 to 1200 for the Booms). Mirrored,
    // they would run off the cave's left end, and every Boom would come
    // out at his own spot, as one does with no ground below its point.
    seal_and_booms(c, &mut t, left).await;
    let at = t.field.mob_positions[&left];
    let booms: Vec<_> = t.field.alive(BLOODY_BOOM);
    let spots: Vec<_> = booms.iter().map(|b| t.field.mob_positions[b]).collect();
    for &(x, y) in &spots {
        let ahead = (-359..=741).contains(&x) && (18..=240).contains(&y);
        assert!(ahead || (x, y) == at, "{spots:?}");
    }
    assert!(spots.iter().any(|&(x, _)| x > at.0), "{spots:?}");

    // The right one falls, his drops anyone's: his certificate and his
    // potions every time.
    for _ in 0..10 {
        if t.field.killed.contains(&right) {
            break;
        }
        let bars = t.field.boss_bars.len();
        send(c, attack(right, HIT)).await;
        until(c, &mut t, |t| {
            t.field.boss_bars.len() > bars || t.field.killed.contains(&right)
        })
        .await;
    }
    until(c, &mut t, |t| {
        [PIANUS_CERTIFICATE, 2_000_004, 2_000_005]
            .iter()
            .all(|&item| t.field.drops.iter().any(|d| !d.meso && d.value == item))
    })
    .await;
    assert_eq!(
        t.field.boss_bars.last().unwrap().1,
        0,
        "{:?}",
        t.field.boss_bars
    );
}
