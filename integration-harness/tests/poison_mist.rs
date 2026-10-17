//! Poison Mist on Snail Hunting Ground II (40001): the mage's cloud covers
//! the box around them, the map sees it come and go, and while it lasts it
//! poisons the snails in it.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::{attack, magic_attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::{Duration, Instant};

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const FP_MAGE: i16 = 211;
const POISON_MIST: i32 = 2111003;
const POISON: u128 = 1 << 9;

#[tokio::test]
async fn poison_mist_poisons_the_snails_in_it_until_it_lifts() {
    let config = load_harness_config_or_fail().await;
    let mage_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(mage_id, FP_MAGE, 70, 2000)
        .await
        .expect("make a mage");
    // Level 5: 8 seconds, a 240 x 180 box, and a 45% chance each reach.
    grant_skill(mage_id, POISON_MIST, 5)
        .await
        .expect("teach Poison Mist");

    let mut mage = relogin(&config).await;
    let c = &mut mage.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut t).await, HUNTING_GROUND);
    until(c, &mut t, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = t.field.alive(SNAIL)[0];
    let (x, y) = t.field.mob_positions[&snail];
    let (x, y) = (i32::from(x), i32::from(y));
    send(c, walk_to(x as i16, y as i16)).await;

    // Reaching every 2.5 seconds from its start, it may miss a snail
    // three times; cast again until one is poisoned.
    for _ in 0..4 {
        let cast = Instant::now();
        let mists = t.field.mists.len();
        send(c, magic_attack(POISON_MIST, snail, 0)).await;
        until(c, &mut t, |t| t.field.mists.len() > mists).await;
        let mist = t.field.mists[mists];
        assert_eq!(
            (
                mist.kind,
                mist.owner_id,
                mist.skill_id,
                mist.level,
                mist.delay
            ),
            (1, mage_id, POISON_MIST, 5, 8)
        );
        // Facing left, the box as the data has it.
        assert_eq!(mist.area, (x - 120, y - 90, x + 120, y + 90));

        until(c, &mut t, |t| {
            t.field.mists_removed.contains(&mist.object_id)
        })
        .await;
        let lasted = cast.elapsed();
        assert!(
            lasted >= Duration::from_millis(8_600) && lasted < Duration::from_secs(10),
            "8 seconds from its start, 800 ms after the cast: {lasted:?}"
        );
        if !t.field.stat_sets.is_empty() {
            break;
        }
    }

    assert!(!t.field.stat_sets.is_empty(), "the mist poisoned a snail");
    for &(oid, stats, value, skill_id) in &t.field.stat_sets {
        // Level 5 poison: max HP 8 over 65, rounded up.
        assert_eq!((stats, value, skill_id), (POISON, 1, POISON_MIST));
        let (sx, sy) = t.field.mob_positions[&oid];
        let (sx, sy) = (i32::from(sx), i32::from(sy));
        assert!((x - 120..=x + 120).contains(&sx) && (y - 90..=y + 90).contains(&sy));
    }

    // Leave no hurt snails for the tests that hunt here next, with plain
    // swings, which leave no mist.
    let poisoned: Vec<i32> = t.field.stat_sets.iter().map(|s| s.0).collect();
    for oid in poisoned {
        if !t.field.killed.contains(&oid) {
            send(c, attack(oid, 8)).await;
        }
    }
    let mists = t.field.mists.len();
    read_for(c, &mut t, 500).await;
    assert_eq!(t.field.mists.len(), mists);
}
