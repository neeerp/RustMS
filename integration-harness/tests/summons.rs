//! Summons on Snail Hunting Ground II (40001): a Ranger's Silver Hawk
//! and Puppet appear for the map, the Hawk's attacks reach the others
//! within its damage cap, a Puppet breaks when hurt past its HP, and the
//! Hawk follows its owner to the next map. A Dark Knight's Beholder heals
//! and buffs them, no sooner than its skills allow.

use integration_harness::database::{grant_skill, grant_stack, set_job};
use integration_harness::hunting::{
    attack, beholder, damage_summon, summon, summon_attack, walk_to,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const FOREST: i32 = 40_000;
const SNAIL: i32 = 100100;
const RANGER: i16 = 311;
const SILVER_HAWK: i32 = 3111005;
const PUPPET: i32 = 3111002;
const ETC: i16 = 4;
const SUMMONING_ROCK: i32 = 4006001;
/// Henesys Hunting Ground I, apart from the ranger's test.
const HENESYS_HUNTING_GROUND: i32 = 104_040_000;
const DARK_KNIGHT: i16 = 132;
const BEHOLDER: i32 = 1321007;
const AURA: i32 = 1320008;
const HEX: i32 = 1320009;

#[tokio::test]
async fn a_rangers_hawk_attacks_and_follows_and_their_puppet_breaks() {
    let config = load_harness_config_or_fail().await;
    let ranger_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(ranger_id, RANGER, 70, 2000)
        .await
        .expect("make a ranger");
    grant_skill(ranger_id, SILVER_HAWK, 1)
        .await
        .expect("teach Silver Hawk");
    // Level 1: 5 seconds, and 500 HP.
    grant_skill(ranger_id, PUPPET, 1)
        .await
        .expect("teach Puppet");
    // Each Hawk costs a Summoning Rock.
    grant_stack(ranger_id, ETC, 1, SUMMONING_ROCK, 5)
        .await
        .expect("give Summoning Rocks");

    let mut ranger = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (r, w) = (&mut ranger.connection, &mut watcher.connection);
    let (mut rt, mut wt) = (Trace::default(), Trace::default());
    assert_eq!(warp(w, HUNTING_GROUND, "", &mut wt).await, HUNTING_GROUND);
    assert_eq!(warp(r, HUNTING_GROUND, "", &mut rt).await, HUNTING_GROUND);
    until(r, &mut rt, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = rt.field.alive(SNAIL)[0];
    let (x, y) = rt.field.mob_positions[&snail];
    send(r, walk_to(x, y)).await;

    // The Hawk circles its owner and attacks; both see it come.
    send(r, summon(SILVER_HAWK, 1, x, y, true)).await;
    until(w, &mut wt, |t| !t.field.summons.is_empty()).await;
    until(r, &mut rt, |t| !t.field.summons.is_empty()).await;
    let hawk = wt.field.summons[0];
    assert_eq!(rt.field.summons[0], hawk);
    assert_eq!(
        (
            hawk.owner_id,
            hawk.skill_id,
            hawk.level,
            (hawk.x, hawk.y),
            hawk.moves,
            hawk.assist,
            hawk.appears
        ),
        (ranger_id, SILVER_HAWK, 1, (x, y), 3, 1, true)
    );

    // An attack past what the ranger could roll counts for nothing; one
    // within it reaches the watcher.
    send(r, summon_attack(hawk.object_id, snail, SNAIL, 100_000)).await;
    send(r, summon_attack(hawk.object_id, snail, SNAIL, 5)).await;
    until(w, &mut wt, |t| !t.field.summon_attacks.is_empty()).await;
    read_for(w, &mut wt, 300).await;
    assert_eq!(
        wt.field.summon_attacks,
        vec![(hawk.object_id, 0x84, vec![(snail, 5)])]
    );
    // Leave no hurt snail for the tests that hunt here next.
    send(r, attack(snail, 8)).await;

    // The Puppet stands where the ranger's client put it, 200 px ahead.
    send(r, summon(PUPPET, 1, x + 200, y, false)).await;
    until(w, &mut wt, |t| t.field.summons.len() == 2).await;
    let puppet = wt.field.summons[1];
    assert_eq!(
        (
            puppet.skill_id,
            (puppet.x, puppet.y),
            puppet.moves,
            puppet.assist
        ),
        (PUPPET, (x + 200, y), 0, 0)
    );

    // A snail hurts it, then breaks it: everyone sees it go, broken.
    send(r, damage_summon(puppet.object_id, 300, SNAIL)).await;
    read_for(w, &mut wt, 300).await;
    assert!(wt.field.summons_removed.is_empty());
    send(r, damage_summon(puppet.object_id, 200, SNAIL)).await;
    until(w, &mut wt, |t| !t.field.summons_removed.is_empty()).await;
    until(r, &mut rt, |t| !t.field.summons_removed.is_empty()).await;
    assert_eq!(wt.field.summons_removed, vec![(puppet.object_id, 2)]);
    assert_eq!(rt.field.summons_removed, vec![(puppet.object_id, 2)]);

    // The Hawk leaves with its owner, unannounced: the others drop it with
    // them. It comes along without its entrance.
    rt = Trace::default();
    assert_eq!(warp(r, FOREST, "", &mut rt).await, FOREST);
    until(r, &mut rt, |t| !t.field.summons.is_empty()).await;
    read_for(w, &mut wt, 300).await;
    assert_eq!(wt.field.summons_removed.len(), 1);
    let again = rt.field.summons[0];
    assert_eq!(
        (again.owner_id, again.skill_id, again.appears),
        (ranger_id, SILVER_HAWK, false)
    );
}

/// TAKE_DAMAGE: a snail's touch.
fn touched(damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(SNAIL).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

#[tokio::test]
async fn a_dark_knights_beholder_heals_and_hexes_them_in_its_own_time() {
    let config = load_harness_config_or_fail().await;
    let knight_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(knight_id, DARK_KNIGHT, 120, 2000)
        .await
        .expect("make a dark knight");
    // Level 1: Aura heals 40 every 10 s; Hex raises weapon defense by 20
    // for 20 s, every 48 s.
    for skill_id in [BEHOLDER, AURA, HEX] {
        grant_skill(knight_id, skill_id, 1)
            .await
            .expect("teach the Beholder");
    }

    let mut knight = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (k, w) = (&mut knight.connection, &mut watcher.connection);
    let (mut kt, mut wt) = (Trace::default(), Trace::default());
    let map = HENESYS_HUNTING_GROUND;
    assert_eq!(warp(w, map, "", &mut wt).await, map);
    assert_eq!(warp(k, map, "", &mut kt).await, map);
    send(k, summon(BEHOLDER, 1, 0, 0, false)).await;
    until(w, &mut wt, |t| !t.field.summons.is_empty()).await;
    let summoned = std::time::Instant::now();
    let eye = wt.field.summons[0];
    assert_eq!(
        (eye.owner_id, eye.skill_id, eye.moves, eye.assist),
        (knight_id, BEHOLDER, 1, 2)
    );
    send(k, touched(12)).await;
    until(k, &mut kt, |t| t.stat(stat::HP).is_some()).await;
    let hurt = kt.stat(stat::HP).unwrap();

    // Not in its first 10 s.
    send(k, beholder(eye.object_id, AURA, None)).await;
    read_for(w, &mut wt, 300).await;
    assert!(wt.field.summon_skills.is_empty());

    tokio::time::sleep(Duration::from_secs(10).saturating_sub(summoned.elapsed())).await;
    send(k, beholder(eye.object_id, AURA, None)).await;
    until(w, &mut wt, |t| !t.field.summon_skills.is_empty()).await;
    until(k, &mut kt, |t| t.stat(stat::HP) != Some(hurt)).await;
    assert_eq!(kt.stat(stat::HP), Some(hurt + 12));

    // Hex level 1 raises only weapon defense, its own time apart.
    send(k, beholder(eye.object_id, HEX, Some(4))).await;
    send(k, beholder(eye.object_id, HEX, Some(0))).await;
    until(k, &mut kt, |t| t.buffs.len() == 2).await;
    assert_eq!(kt.buffs[1], (HEX, 20, 20_000));

    // Neither again so soon.
    send(k, beholder(eye.object_id, AURA, None)).await;
    send(k, beholder(eye.object_id, HEX, Some(0))).await;
    read_for(w, &mut wt, 300).await;
    read_for(k, &mut kt, 300).await;
    assert_eq!(
        wt.field.summon_skills,
        vec![(eye.object_id, 6), (eye.object_id, 7)]
    );
    assert_eq!(kt.buffs.len(), 2);
}
