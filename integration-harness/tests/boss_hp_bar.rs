//! A boss's HP bar across the top of the screen (FIELD_EFFECT 5): hitting
//! the Neo City boss 7220003 in 240070203 shows everyone on the field its
//! bar, by its template id and in its colours, and killing it takes the bar
//! down.

use integration_harness::database::{set_str, set_weapon_attack};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const MAP: i32 = 240_070_203;
const BOSS: i32 = 7_220_003;
const MAX_HP: i32 = 40_000;
/// Green over yellow-green (`hpTagColor`, `hpTagBgcolor`).
const COLORS: (u8, u8) = (4, 3);

#[tokio::test]
async fn a_boss_shows_the_field_its_hp_bar() {
    let hunter_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&hunter_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&hunter_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Strong enough to kill it in two hits.
    set_str(id, 30_000).await.expect("hit hard");
    set_weapon_attack(id, 1_000).await.expect("weapon attack");
    let mut hunter = relogin(&hunter_config).await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (h, w) = (&mut hunter.connection, &mut watcher.connection);
    let (mut ht, mut wt) = (Trace::default(), Trace::default());
    assert_eq!(warp(h, MAP, "", &mut ht).await, MAP);
    assert_eq!(warp(w, MAP, "", &mut wt).await, MAP);
    until(h, &mut ht, |t| !t.field.alive(BOSS).is_empty()).await;
    let boss = ht.field.alive(BOSS)[0];
    // Nothing until it is hurt.
    read_for(w, &mut wt, 300).await;
    assert!(wt.field.boss_bars.is_empty(), "{:?}", wt.field.boss_bars);

    send(h, attack(boss, 1_000)).await;
    until(w, &mut wt, |t| !t.field.boss_bars.is_empty()).await;
    assert_eq!(wt.field.boss_bars, [(BOSS, 39_000, MAX_HP, COLORS)]);
    until(h, &mut ht, |t| !t.field.boss_bars.is_empty()).await;

    send(h, attack(boss, 39_000)).await;
    until(w, &mut wt, |t| t.field.killed.contains(&boss)).await;
    until(w, &mut wt, |t| t.field.boss_bars.len() == 2).await;
    assert_eq!(wt.field.boss_bars[1], (BOSS, 0, MAX_HP, COLORS));
}
