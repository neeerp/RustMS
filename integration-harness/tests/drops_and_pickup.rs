//! Mob drops: rolling, ownership, pickup into the inventory or meso total,
//! persistence, and quest-only drops.

use integration_harness::hunting::{attack, pickup};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const SMALL_FOREST: i32 = 40_000;
const SENTINEL: i32 = 9300018;

#[tokio::test]
async fn snail_drops_go_to_the_killer_first_and_persist() {
    let hunter_config = load_harness_config_or_fail().await;
    let rival_config = load_harness_config_or_fail().await;
    let mut hunter = login_to_world_session(&hunter_config).await.unwrap();
    let mut rival = login_to_world_session(&rival_config).await.unwrap();
    let (mut h, mut r) = (Trace::default(), Trace::default());
    assert_eq!(
        warp(&mut hunter.connection, FOREST, "", &mut h).await,
        FOREST
    );
    until(&mut hunter.connection, &mut h, |t| t.field.mobs.len() >= 17).await;
    assert_eq!(
        warp(&mut rival.connection, FOREST, "", &mut r).await,
        FOREST
    );
    read_for(&mut rival.connection, &mut r, 300).await;
    read_for(&mut hunter.connection, &mut h, 300).await;
    // Earlier tests may have left drops (and pending removals) on this map.
    let old_drops = h.field.drops.len();

    // Snails drop mesos 65% of the time; kill until something drops.
    for oid in h.field.alive(SNAIL) {
        send(&mut hunter.connection, attack(oid, 8)).await;
        until(&mut hunter.connection, &mut h, |t| {
            t.field.killed.contains(&oid)
        })
        .await;
        read_for(&mut hunter.connection, &mut h, 200).await;
        if h.field.drops.len() > old_drops {
            break;
        }
    }
    let loot = *h
        .field
        .drops
        .get(old_drops)
        .expect("seventeen snails dropped nothing");
    let ours = |t: &Trace| t.field.removals(loot.object_id);
    until(&mut rival.connection, &mut r, |t| {
        t.field.drops.iter().any(|d| d.object_id == loot.object_id)
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
    until(&mut hunter.connection, &mut h, |t| !ours(t).is_empty()).await;
    assert_eq!(ours(&h), [Some(hunter.character_id)]);
    until(&mut rival.connection, &mut r, |t| !ours(t).is_empty()).await;
    if loot.meso {
        until(&mut hunter.connection, &mut h, |t| {
            t.stat(stat::MESO).is_some()
        })
        .await;
        assert_eq!(h.stat(stat::MESO), Some(i64::from(loot.value)));
    } else {
        until(&mut hunter.connection, &mut h, |t| !t.added.is_empty()).await;
        assert_eq!(h.added[0].2, loot.value);
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
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, SMALL_FOREST, "", &mut seen).await,
        SMALL_FOREST
    );
    until(&mut session.connection, &mut seen, |t| {
        t.field.alive(SENTINEL).len() >= 3
    })
    .await;
    let sentinel = seen.field.alive(SENTINEL)[0];
    send(&mut session.connection, attack(sentinel, 8)).await;
    until(&mut session.connection, &mut seen, |t| {
        t.field.killed.contains(&sentinel)
    })
    .await;
    read_for(&mut session.connection, &mut seen, 500).await;
    // Without quest 1035 started, the sentinel's only drop (the shellpiece
    // override) does not happen.
    assert!(seen.field.drops.is_empty(), "{:?}", seen.field.drops);
}
