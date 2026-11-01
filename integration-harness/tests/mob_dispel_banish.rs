//! A mob's Dispel and Banish reach the players in their areas. Aqua
//! Road's 8150100 (on 230040400) casts Dispel (127 level 3, sure to
//! land), which ends a beginner's Nimble Feet. A Kerning City subway
//! Shade (5090000) casts Banish (129 level 5), which tells the player why
//! in pink and sends them to the ticketing booth (103000100, `sp`).
//! Delli's serpents (9300160, `banType` 1) banish by touch: the client's
//! MOB_BANISH_PLAYER sends the player back to Looking for Delli 1
//! (925010000); the same packet for the Shade, which banishes only by its
//! skill, does nothing.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::{move_mob, use_skill, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::{io::write::PktWrite, Packet};
use tokio::time::Duration;

const DISPEL_MAP: i32 = 230_040_400;
const DISPELLER: i32 = 8_150_100;
const DISPEL: u8 = 127;
const SUBWAY: i32 = 103_000_105;
const SHADE: i32 = 5_090_000;
const BANISH: u8 = 129;
const TICKETING_BOOTH: i32 = 103_000_100;
const NIMBLE_FEET: i32 = 1002;
const LOOKING_FOR_DELLI_2: i32 = 925_010_100;
const LOOKING_FOR_DELLI_1: i32 = 925_010_000;
const SERPENT: i32 = 9_300_160;
/// Speed's CANCEL_BUFF bit (second mask).
const SPEED_MASK: i64 = 0x80_0000_0000;

/// The first mob of `mob_id` the player controls, and where it is.
async fn controlled(c: &mut MapleTestConnection, t: &mut Trace, mob_id: i32) -> (i32, (i16, i16)) {
    let find = |t: &Trace| {
        t.field
            .controlled
            .iter()
            .copied()
            .find(|oid| t.field.mobs.contains(&(*oid, mob_id)))
    };
    until(c, t, |t| find(t).is_some()).await;
    let mob = find(t).unwrap();
    (mob, t.field.mob_positions[&mob])
}

/// Have the mob cast whatever it is granted from its first skill
/// animation, until it is granted and casts `skill`.
async fn cast(c: &mut MapleTestConnection, t: &mut Trace, mob: i32, at: (i16, i16), skill: u8) {
    for _ in 0..3 {
        let granted = t.field.grants.len();
        send(c, move_mob(mob, at, true, 0, 0)).await;
        until(c, t, |t| t.field.grants.len() > granted).await;
        let (_, grant, level) = *t.field.grants.last().unwrap();
        let data = i32::from(grant) | i32::from(level) << 8;
        send(c, move_mob(mob, at, false, 21 << 1, data)).await;
        if grant == skill {
            return;
        }
        read_for(c, t, 300).await;
    }
    panic!("{skill} never granted: {:?}", t.field.grants);
}

#[tokio::test]
async fn a_mobs_dispel_ends_the_players_buffs() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 100).await.expect("level up");
    grant_skill(id, NIMBLE_FEET, 1)
        .await
        .expect("learn Nimble Feet");
    let mut player = relogin(&config).await;
    let c = &mut player.connection;

    let mut t = Trace::default();
    assert_eq!(warp(c, DISPEL_MAP, "", &mut t).await, DISPEL_MAP);
    let (mob, at) = controlled(c, &mut t, DISPELLER).await;
    send(c, use_skill(NIMBLE_FEET, 1)).await;
    until(c, &mut t, |t| !t.buffs.is_empty()).await;

    // Standing on it, its Dispel (its Magic Up may come first) ends the
    // buff.
    send(c, walk_to(at.0, at.1)).await;
    cast(c, &mut t, mob, at, DISPEL).await;
    until(c, &mut t, |t| !t.cancelled_buffs.is_empty()).await;
    assert_eq!(t.cancelled_buffs, [SPEED_MASK]);
}

#[tokio::test]
async fn a_mobs_banish_sends_the_player_away() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let c = &mut player.connection;

    let mut t = Trace::default();
    assert_eq!(warp(c, SUBWAY, "", &mut t).await, SUBWAY);
    let (mob, at) = controlled(c, &mut t, SHADE).await;
    send(c, walk_to(at.0, at.1)).await;
    cast(c, &mut t, mob, at, BANISH).await;
    until(c, &mut t, |t| t.warps.len() > 1).await;
    assert_eq!(t.warps.last(), Some(&TICKETING_BOOTH));
    assert_eq!(t.broadcasts.len(), 1);
    let (kind, message) = &t.broadcasts[0];
    assert_eq!(*kind, 5);
    assert!(message.ends_with("Subway Ticketing Booth."), "{message}");
}

fn touched(mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MobBanishPlayer as i16).unwrap();
    p.write_int(mob_id).unwrap();
    p
}

#[tokio::test]
async fn a_serpents_touch_banishes_but_a_shades_does_not() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let c = &mut player.connection;

    let mut t = Trace::default();
    assert_eq!(warp(c, SUBWAY, "", &mut t).await, SUBWAY);
    until(c, &mut t, |t| !t.field.alive(SHADE).is_empty()).await;
    send(c, touched(SHADE)).await;
    read_for(c, &mut t, 500).await;
    assert_eq!(t.warps, [SUBWAY], "{t:?}");

    let mut t = Trace::default();
    let delli = LOOKING_FOR_DELLI_2;
    assert_eq!(warp(c, delli, "", &mut t).await, delli);
    until(c, &mut t, |t| !t.field.alive(SERPENT).is_empty()).await;
    send(c, touched(SERPENT)).await;
    until(c, &mut t, |t| t.warps.len() > 1).await;
    assert_eq!(t.warps.last(), Some(&LOOKING_FOR_DELLI_1));
    assert!(t.broadcasts.iter().any(|(kind, _)| *kind == 5), "{t:?}");
}
