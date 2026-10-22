//! Magic Guard takes its share of a mob's touch from MP instead of HP, and
//! Power Guard sends some of it back at the mob. The others on the map see
//! what the player's client said of the hit: the share sent back, Power
//! Stance, and a dodge.

use integration_harness::database::{grant_skill, set_dex, set_hp, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::build::world::mob::{Hurt, FROM_TOUCH};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MAGICIAN: i16 = 200;
const MAGIC_GUARD: i32 = 2001002;
const FIGHTER: i16 = 110;
const POWER_GUARD: i32 = 1101007;
const STUMPY: i32 = 3_220_000;
const NIGHT_LORD: i16 = 412;
const SHADOW_SHIFTER: i32 = 4120002;
/// Henesys Hunting Ground I.
const HUNTING_GROUND: i32 = 104_040_000;

/// The 14 bytes after the Power Guard percent and the Guardian byte:
/// Power Guard, the mob's object id (1234) and hit action (6), its
/// position (100, -50), and the player's (7, 8). The others see the first
/// ten.
const GUARD_BLOCK: [u8; 14] = [1, 0xD2, 0x04, 0, 0, 6, 100, 0, 0xCE, 0xFF, 7, 0, 8, 0];

/// TAKE_DAMAGE for `mob_id`'s touch from the `left` side, then `tail`.
fn touch(damage: i32, mob_id: i32, left: u8, tail: &[u8]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(left).unwrap();
    p.write_bytes(tail).unwrap();
    p
}

/// TAKE_DAMAGE for `mob_id`'s touch, with no guard or stance after it.
fn touched(damage: i32, mob_id: i32) -> Packet {
    touch(damage, mob_id, 0, &[])
}

/// What follows a touch Power Guard sent `percent` of back, and whether
/// Power Stance kept the player in place.
fn guarded(percent: u8, stance: bool) -> Vec<u8> {
    [&[percent, 0][..], &GUARD_BLOCK, &[u8::from(stance)]].concat()
}

/// `touched`, with the Power Guard block sending `percent` back.
fn touched_back(damage: i32, mob_id: i32, percent: u8) -> Packet {
    touch(damage, mob_id, 0, &guarded(percent, false))
}

/// The hit the others on the map saw `id` take.
fn hurt_of(t: &Trace, id: i32) -> Option<&Hurt> {
    t.field
        .hurts
        .iter()
        .find(|(who, _)| *who == id)
        .map(|(_, hurt)| hurt)
}

#[tokio::test]
async fn magic_guard_pays_most_of_a_touch_in_mp() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, MAGICIAN, 30, 500)
        .await
        .expect("make a magician");
    set_hp(id, 500, 500).await.expect("full HP");
    grant_skill(id, MAGIC_GUARD, 20)
        .await
        .expect("teach Magic Guard");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // Without the guard a touch is all HP.
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [480]);
    assert!(t.stat_values(stat::MP).is_empty());

    let t = step(c, &m, use_skill(MAGIC_GUARD, 20)).await;
    let mp = *t.stat_values(stat::MP).last().expect("the cast costs MP");

    // At level 20 it takes 80%: 16 of 20 from MP, 4 from HP.
    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [476]);
    assert_eq!(t.stat_values(stat::MP), [mp - 16]);
}

#[tokio::test]
async fn power_guard_sends_some_of_a_touch_back() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, FIGHTER, 30, 500).await.expect("make a fighter");
    set_hp(id, 500, 500).await.expect("full HP");
    // Enough accuracy that Stumpy, five levels up, cannot dodge it.
    set_dex(id, 100).await.expect("raise DEX");
    grant_skill(id, POWER_GUARD, 30)
        .await
        .expect("teach Power Guard");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    step(c, &m, use_skill(POWER_GUARD, 30)).await;

    // 40% of 40 is 16, halved for a boss: 8 of the 40 never lands.
    let t = step(c, &m, touched_back(40, STUMPY, 40)).await;
    assert_eq!(t.stat_values(stat::HP), [468]);

    // Without the block the client saw no Power Guard, so none applies.
    let t = step(c, &m, touched(40, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [428]);
}

#[tokio::test(flavor = "multi_thread")]
async fn others_see_the_share_sent_back_and_the_stance() {
    let config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, FIGHTER, 30, 500).await.expect("make a fighter");
    set_hp(id, 500, 500).await.expect("full HP");
    set_dex(id, 100).await.expect("raise DEX");
    grant_skill(id, POWER_GUARD, 30)
        .await
        .expect("teach Power Guard");

    let mut session = relogin(&config).await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (c, w) = (&mut session.connection, &mut watcher.connection);
    let (mut t, mut wt) = (Trace::default(), Trace::default());
    assert_eq!(warp(w, HUNTING_GROUND, "", &mut wt).await, HUNTING_GROUND);
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut t).await, HUNTING_GROUND);
    step(c, &m, use_skill(POWER_GUARD, 30)).await;

    // From the right, Power Guard sending 40% back and Stance holding.
    send(c, touch(40, STUMPY, 1, &guarded(40, true))).await;
    until(w, &mut wt, |t| hurt_of(t, id).is_some()).await;
    let block: [u8; 10] = GUARD_BLOCK[..10].try_into().unwrap();
    // It shows the 32 that landed, as in power_guard_sends_some_of_a_touch_back.
    assert_eq!(
        *hurt_of(&wt, id).unwrap(),
        Hurt {
            from: FROM_TOUCH,
            damage: 40,
            mob_id: STUMPY,
            left: 1,
            reflect: Some((40, block)),
            stance: true,
            shown: 32,
            dodged_by: None,
        }
    );

    // The player's own client shows it already.
    barrier(c, "own hit", &mut t).await;
    assert!(hurt_of(&t, id).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn others_see_shadow_shifter_dodge() {
    let config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, NIGHT_LORD, 120, 500)
        .await
        .expect("make a night lord");
    set_hp(id, 500, 500).await.expect("full HP");
    grant_skill(id, SHADOW_SHIFTER, 30)
        .await
        .expect("teach Shadow Shifter");

    let mut session = relogin(&config).await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (c, w) = (&mut session.connection, &mut watcher.connection);
    let (mut t, mut wt) = (Trace::default(), Trace::default());
    assert_eq!(warp(w, HUNTING_GROUND, "", &mut wt).await, HUNTING_GROUND);
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut t).await, HUNTING_GROUND);

    // A dodge shows no number but the skill that dodged.
    send(c, touched(-1, STUMPY)).await;
    until(w, &mut wt, |t| hurt_of(t, id).is_some()).await;
    assert_eq!(
        *hurt_of(&wt, id).unwrap(),
        Hurt {
            from: FROM_TOUCH,
            damage: -1,
            mob_id: STUMPY,
            shown: -1,
            dodged_by: Some(SHADOW_SHIFTER),
            ..Hurt::default()
        }
    );
}
