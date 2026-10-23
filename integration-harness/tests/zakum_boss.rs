//! Zakum on its altar (280030000): the Eye of Fire dropped on the altar
//! plays Zakum's music and brings out its body, trembling in and not yet
//! real, then its eight arms. While an arm is out the body takes no harm
//! and no one moves it; the last arm's death makes it real, and from there
//! it dies into its second body, and that into its third.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_str, set_weapon_attack};
use integration_harness::events::{go_to, throw_down};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const ALTAR_MAP: i32 = 280030000;
const ALTAR: i32 = 2111001;
const EYE_OF_FIRE: i32 = 4001017;
const BODY: i32 = 8800000;
const ARMS: std::ops::RangeInclusive<i32> = 8800003..=8800010;
const PINK: u8 = 5;
/// Under the most a level-1 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// Hit `object_id` until `seen` sees it die, each hit once answered.
async fn beat(c: &mut MapleTestConnection, seen: &mut Trace, object_id: i32) {
    for _ in 0..40 {
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

#[tokio::test]
async fn the_eye_of_fire_brings_out_zakum_whose_arms_shield_its_body() {
    let hunter_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&hunter_config)
        .await
        .unwrap()
        .character_id;
    drop(relogin(&hunter_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_str(id, 32_000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    grant_stack(id, 4, 1, EYE_OF_FIRE, 1).await.unwrap();
    let mut hunter = relogin(&hunter_config).await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let mut wt = Trace::default();
    assert_eq!(
        warp(&mut watcher.connection, ALTAR_MAP, "", &mut wt).await,
        ALTAR_MAP
    );
    let (mut ht, altar) = go_to(&mut hunter, ALTAR_MAP, ALTAR).await;
    let (h, w) = (&mut hunter.connection, &mut watcher.connection);

    // The Eye of Fire, dropped on the altar's ledge.
    send(h, walk_to(altar.x - 30, altar.y - 36)).await;
    send(h, throw_down(1, 1)).await;
    let summoned = net::content::text().get(keys::ZAKUM_ALTAR_SUMMONED, &[]);
    until(w, &mut wt, |t| {
        t.broadcasts.contains(&(PINK, summoned.clone()))
    })
    .await;
    assert!(
        wt.field_effects
            .contains(&(6, net::reactors::ZAKUM_MUSIC.to_string())),
        "{wt:?}"
    );
    // The body first, trembling in, then the arms.
    let ids: Vec<i32> = wt.field.mobs.iter().map(|&(_, id)| id).collect();
    assert_eq!(ids, [BODY].into_iter().chain(ARMS).collect::<Vec<_>>());
    let body = wt.field.mobs[0].0;
    assert_eq!(wt.field.appearances[&body], -4);
    let arms: Vec<i32> = wt.field.mobs[1..].iter().map(|&(oid, _)| oid).collect();
    until(h, &mut ht, |t| t.field.mobs.len() == 9).await;
    read_for(h, &mut ht, 300).await;
    read_for(w, &mut wt, 300).await;
    let controlled: Vec<i32> = ht
        .field
        .controlled
        .union(&wt.field.controlled)
        .copied()
        .collect();
    assert_eq!(controlled, arms);

    // Its arms shield it: the hunter's hits do nothing to it.
    send(h, attack(body, HIT)).await;
    read_for(h, &mut ht, 500).await;
    assert!(
        !ht.field.hp_bars.iter().any(|&(oid, _)| oid == body) && ht.field.boss_bars.is_empty(),
        "{:?}",
        ht.field
    );

    // The last arm's death makes it real: it is just there, and moved.
    for &arm in &arms {
        assert!(!ht.field.controlled.contains(&body), "{:?}", ht.field);
        beat(h, &mut ht, arm).await;
    }
    assert_eq!(wt.field.appearances[&body], -4);
    until(w, &mut wt, |t| t.field.appearances[&body] == -1).await;
    until(h, &mut ht, |t| t.field.controlled.contains(&body)).await;

    // It takes harm now, and dies into its second body, and that into its
    // third.
    for (mob_id, next) in [
        (BODY, Some(8800001)),
        (8800001, Some(8800002)),
        (8800002, None),
    ] {
        let oid = *ht.field.alive(mob_id).first().unwrap();
        beat(h, &mut ht, oid).await;
        until(w, &mut wt, |t| {
            t.field.boss_bars.iter().any(|b| b.0 == mob_id)
        })
        .await;
        if let Some(next) = next {
            until(h, &mut ht, |t| !t.field.alive(next).is_empty()).await;
        }
    }
}
