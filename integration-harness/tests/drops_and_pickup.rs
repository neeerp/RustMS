//! Mob drops: rolling, ownership, pickup into the inventory or meso total,
//! persistence, and quest-only drops.

use integration_harness::hunting::{attack, pickup, walk_to, MapDrop};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, START};
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const EAST_ENTRANCE: i32 = 30_000;
const SENTINEL: i32 = 9300018;
const TODD: i32 = 2004;
const SHELLPIECE: i32 = 4031802;

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

    // Snails drop mesos 65% of the time; kill until something drops. The
    // harness makes every kill drop a Monster Book card too
    // (tests/monster_book.rs), so look past those.
    let not_card = |d: &&MapDrop| d.meso || d.value / 10_000 != 238;
    for oid in h.field.alive(SNAIL) {
        send(&mut hunter.connection, attack(oid, 8)).await;
        until(&mut hunter.connection, &mut h, |t| {
            t.field.killed.contains(&oid)
        })
        .await;
        read_for(&mut hunter.connection, &mut h, 200).await;
        if h.field.drops[old_drops..].iter().any(|d| not_card(&d)) {
            break;
        }
    }
    let loot = *h.field.drops[old_drops..]
        .iter()
        .find(not_card)
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
        warp(&mut session.connection, EAST_ENTRANCE, "", &mut seen).await,
        EAST_ENTRANCE
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

fn party_operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartyOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

#[tokio::test]
async fn a_party_members_quest_drop_shows_only_to_the_one_who_needs_it() {
    let hunter_config = load_harness_config_or_fail().await;
    let helper_config = load_harness_config_or_fail().await;
    let mut hunter = login_to_world_session(&hunter_config).await.unwrap();
    let mut helper = login_to_world_session(&helper_config).await.unwrap();
    let (mut h, mut p) = (Trace::default(), Trace::default());

    // The helper invites the hunter into a party.
    let mut invite = party_operation(4);
    invite
        .write_str_with_length(&hunter.character_name)
        .unwrap();
    send(&mut helper.connection, invite).await;
    until(&mut hunter.connection, &mut h, |t| {
        !t.party_invites.is_empty()
    })
    .await;
    let mut accept = party_operation(3);
    accept.write_int(h.party_invites[0].0).unwrap();
    send(&mut hunter.connection, accept).await;
    until(&mut helper.connection, &mut p, |t| {
        t.party_ops.contains(&0x0F)
    })
    .await;

    // Only the helper is on Todd's hunt.
    assert_eq!(
        warp(&mut helper.connection, EAST_ENTRANCE, "", &mut p).await,
        EAST_ENTRANCE
    );
    let marker = helper_config.character_name.clone();
    act(&mut helper.connection, &mut p, &marker, START, 1035, TODD).await;
    assert_eq!(p.states.get(&1035), Some(&1));
    assert_eq!(
        warp(&mut hunter.connection, EAST_ENTRANCE, "", &mut h).await,
        EAST_ENTRANCE
    );
    until(&mut hunter.connection, &mut h, |t| {
        !t.field.alive(SENTINEL).is_empty()
    })
    .await;

    // The hunter's kill drops the shellpiece for the party, but only the
    // helper, who needs it, sees it.
    let sentinel = h.field.alive(SENTINEL)[0];
    send(&mut hunter.connection, attack(sentinel, 8)).await;
    until(&mut helper.connection, &mut p, |t| {
        t.field.item_drop(SHELLPIECE).is_some()
    })
    .await;
    read_for(&mut hunter.connection, &mut h, 500).await;
    assert!(h.field.killed.contains(&sentinel));
    assert!(
        h.field.item_drop(SHELLPIECE).is_none(),
        "{:?}",
        h.field.drops
    );

    // The hunter can't take it; the helper can.
    let shellpiece = p.field.item_drop(SHELLPIECE).unwrap();
    send(&mut hunter.connection, walk_to(shellpiece.x, shellpiece.y)).await;
    send(
        &mut hunter.connection,
        pickup(shellpiece.object_id, shellpiece.x, shellpiece.y),
    )
    .await;
    read_for(&mut helper.connection, &mut p, 500).await;
    assert!(p.field.removals(shellpiece.object_id).is_empty());
    send(&mut helper.connection, walk_to(shellpiece.x, shellpiece.y)).await;
    send(
        &mut helper.connection,
        pickup(shellpiece.object_id, shellpiece.x, shellpiece.y),
    )
    .await;
    until(&mut helper.connection, &mut p, |t| {
        t.items.contains(&(SHELLPIECE, 1))
    })
    .await;
}
