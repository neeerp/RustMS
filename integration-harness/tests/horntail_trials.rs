//! Horntail's two trials: an expedition's copy of each opens with its
//! head, the Left Head (8810000) on the first and the Right Head (8810001)
//! on the second. Each trial's door (`next00`, `hontale_BR`) is sealed
//! until its head dies, which those there see cleared; then it opens on to
//! the next, and the second's on to Horntail's Cave.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_str, set_weapon_attack};
use integration_harness::events::{talk, told};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ENTRANCE: i32 = 240050400;
const TRIAL_I: i32 = 240060000;
const TRIAL_II: i32 = 240060100;
const CAVE: i32 = 240060200;
const RECRUITER: i32 = 2083004;
const LEFT_HEAD: i32 = 8810000;
const RIGHT_HEAD: i32 = 8810001;
/// Under the most a level-100 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new level-100 character hitting very hard, logged in on the Entrance
/// to Horntail's Cave.
async fn hunter() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 100, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, ENTRANCE, "", &mut seen).await, ENTRANCE);
    session
}

/// CHANGE_MAP_SPECIAL: press Up on the trial's door.
fn door() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("next00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// Hit `object_id` until `seen` sees it die, each hit once answered.
async fn beat(c: &mut MapleTestConnection, seen: &mut Trace, object_id: i32) {
    for _ in 0..100 {
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

/// The object id of the `mob` that `seen` saw come out.
fn the(seen: &Trace, mob: i32) -> i32 {
    let found = seen.field.mobs.iter().find(|&&(_, id)| id == mob);
    found.unwrap_or_else(|| panic!("no {mob}: {seen:?}")).0
}

/// Whether `seen` saw the party quests' clear effect.
fn saw_cleared(seen: &Trace) -> bool {
    seen.field_effects
        .iter()
        .any(|(_, path)| path == "quest/party/clear")
}

/// `player` presses Up on their trial's door, which is sealed.
async fn sealed(player: &mut WorldSession) {
    let sealed = net::content::text().get(keys::HORNTAIL_TRIAL_SEALED, &[]);
    let mut t = Trace::default();
    send(&mut player.connection, door()).await;
    until(&mut player.connection, &mut t, |t| {
        t.item_lines.contains(&(sealed.clone(), 0))
    })
    .await;
    assert!(t.arrivals.is_empty(), "{t:?}");
}

/// `player` goes through their trial's door, and arrives on `map`.
async fn through(player: &mut WorldSession, map: i32) -> Trace {
    let mut t = Trace::default();
    send(&mut player.connection, door()).await;
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == map)
    })
    .await;
    t
}

#[tokio::test]
async fn horntails_trials_open_their_doors_as_their_heads_fall() {
    let mut leader = hunter().await;
    let mut member = hunter().await;
    let recruiter = || talk(ENTRANCE, RECRUITER);

    // An expedition sets out and lands on the first trial, with its head.
    told(&mut leader, recruiter()).await;
    told(&mut leader, answer(1, 1, None)).await;
    told(&mut leader, answer(0, 1, None)).await;
    told(&mut member, recruiter()).await;
    told(&mut member, answer(1, 1, None)).await;
    told(&mut member, answer(0, 1, None)).await;
    told(&mut leader, recruiter()).await;
    let mut lt = told(&mut leader, answer(4, 1, Some(1))).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.arrivals.iter().any(|a| a.0 == TRIAL_I) && !t.field.mobs.is_empty()
    })
    .await;
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.arrivals.iter().any(|a| a.0 == TRIAL_I)
    })
    .await;
    let left = the(&lt, LEFT_HEAD);
    assert_eq!(lt.field.mobs.len(), 1, "{lt:?}");

    // Its door is sealed while the head lives.
    sealed(&mut member).await;

    // The leader brings it down; both see the trial cleared, and the door
    // opens.
    beat(&mut leader.connection, &mut lt, left).await;
    until(&mut leader.connection, &mut lt, saw_cleared).await;
    until(&mut member.connection, &mut mt, saw_cleared).await;
    let mut mt = through(&mut member, TRIAL_II).await;
    until(&mut member.connection, &mut mt, |t| {
        !t.field.mobs.is_empty()
    })
    .await;
    let right = the(&mt, RIGHT_HEAD);

    // The second's door is sealed until its head falls too, then opens on
    // to Horntail's Cave.
    sealed(&mut member).await;
    beat(&mut member.connection, &mut mt, right).await;
    until(&mut member.connection, &mut mt, saw_cleared).await;
    through(&mut member, CAVE).await;
    // The first trial's door stays open.
    through(&mut leader, TRIAL_II).await;
}
