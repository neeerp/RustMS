//! The way to Pink Bean in the Temple of Time: a leader of level 120 or
//! more gathers an expedition at the temple's last gate (2141001) and takes
//! it into a copy of the Twilight of Gods. The dead come back in the copy's
//! Forgotten Twilight, whose portal leads back to the fight; the Temple
//! Keeper (2141002) takes anyone out to the gate, and the expedition lasts
//! until its last member leaves. Between Twilight and Daybreak's portal
//! leads to the gate too. The third and fourth of the expedition to die
//! hear a warning, and the fifth ends it.
//! The harness lets two set out (`RUSTMS_EXPEDITION_MIN`).

use game_data::server_content::keys;
use integration_harness::database::{set_hp, set_job};
use integration_harness::events::{said, talk, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{PINK_BEAN_ARENA, PINK_BEAN_GATE, PINK_BEAN_OUT, PINK_BEAN_REVIVE};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const RECRUITER: i32 = 2141001;
const KEEPER: i32 = 2141002;
/// A mob whose touch the server takes from anyone.
const JR_GARGOYLE: i32 = 9_300_033;
const HP: i32 = 0x400;
const PINK: u8 = 5;

/// A channel holds one expedition at a time: the tests take turns.
static EXPEDITION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A new character of level 120 with little HP, logged in at the temple's
/// last gate.
async fn adventurer() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 120, 50).await.unwrap();
    set_hp(id, 50, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, PINK_BEAN_GATE, "", &mut seen).await, PINK_BEAN_GATE);
    session
}

fn recruiter() -> Packet {
    talk(PINK_BEAN_GATE, RECRUITER)
}

/// TAKE_DAMAGE: touched by a mob for `damage`.
fn touched(damage: i32, mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// `player` is touched till they die.
async fn fall(player: &mut WorldSession) -> Trace {
    let mut seen = Trace::default();
    for _ in 0..40 {
        send(&mut player.connection, touched(20, JR_GARGOYLE)).await;
        read_for(&mut player.connection, &mut seen, 150).await;
        if seen.stat(HP) == Some(0) {
            break;
        }
    }
    assert_eq!(seen.stat(HP), Some(0), "{seen:?}");
    seen
}

/// `player` is touched till they die, and answers the death dialog: where
/// they revive.
async fn die(player: &mut WorldSession) -> i32 {
    let mut seen = fall(player).await;
    warp(&mut player.connection, PINK_BEAN_ARENA, "", &mut seen).await
}

/// Wait until `player` arrives on `map_id`.
async fn arrives(player: &mut WorldSession, mut t: Trace, map_id: i32) {
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == map_id)
    })
    .await;
}

/// The leader gathers a squad that `member` joins, and sets out: both
/// arrive in the Twilight of Gods.
async fn set_out(leader: &mut WorldSession, member: &mut WorldSession) -> Trace {
    let t = told(leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)], "{t:?}");
    let lt = told(leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::SQUAD_OPENED), "{lt:?}");
    told(leader, answer(0, 1, None)).await;
    let t = told(member, recruiter()).await;
    assert!(said(&t, keys::PINK_BEAN_SQUAD_JOIN_ASK), "{t:?}");
    told(member, answer(1, 1, None)).await;
    told(member, answer(0, 1, None)).await;
    let t = told(leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 4)], "{t:?}");
    let mut lt = told(leader, answer(4, 1, Some(1))).await;
    let mut mt = Trace::default();
    for (player, t) in [(&mut *leader, &mut lt), (&mut *member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            t.arrivals.iter().any(|a| a.0 == PINK_BEAN_ARENA)
        })
        .await;
    }
    lt
}

#[tokio::test]
async fn the_dead_come_back_to_pink_bean_and_the_keeper_takes_them_out() {
    let _turn = EXPEDITION.lock().await;
    let mut leader = adventurer().await;
    let mut member = adventurer().await;
    let mut lt = set_out(&mut leader, &mut member).await;

    // The keeper asks before taking anyone out.
    let t = told(&mut member, talk(PINK_BEAN_ARENA, KEEPER)).await;
    assert!(said(&t, keys::PINK_BEAN_KEEPER_LEAVE_ASK), "{t:?}");
    told(&mut member, answer(1, 0, None)).await;

    // Dead in the fight, the member comes back in the copy's Forgotten
    // Twilight, and its portal leads back to the leader.
    let id = member.character_id;
    let sees_member = |t: &Trace| t.players.iter().any(|p| p.character_id == id);
    until(&mut leader.connection, &mut lt, sees_member).await;
    assert_eq!(die(&mut member).await, PINK_BEAN_REVIVE);
    lt.players.clear();
    touch_script_portal(&mut member, PINK_BEAN_REVIVE, "in00").await;
    arrives(&mut member, Trace::default(), PINK_BEAN_ARENA).await;
    until(&mut leader.connection, &mut lt, sees_member).await;

    // Out at the gate, the member may go back in while the leader fights.
    let t = told(&mut member, talk(PINK_BEAN_ARENA, KEEPER)).await;
    assert!(said(&t, keys::PINK_BEAN_KEEPER_LEAVE_ASK), "{t:?}");
    let t = told(&mut member, answer(1, 1, None)).await;
    arrives(&mut member, t, PINK_BEAN_GATE).await;
    let t = told(&mut member, recruiter()).await;
    assert!(said(&t, keys::PINK_BEAN_SQUAD_REJOIN_ASK), "{t:?}");
    told(&mut member, answer(1, 0, None)).await;

    // The last out ends it: the gate's gatherer offers a new one.
    told(&mut leader, talk(PINK_BEAN_ARENA, KEEPER)).await;
    let t = told(&mut leader, answer(1, 1, None)).await;
    arrives(&mut leader, t, PINK_BEAN_GATE).await;
    // The event tells the gate it is over a moment after.
    let mut t = Trace::default();
    for _ in 0..20 {
        t = told(&mut member, recruiter()).await;
        if !said(&t, keys::PINK_BEAN_SQUAD_REJOIN_ASK) {
            break;
        }
        told(&mut member, answer(1, 0, None)).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(!said(&t, keys::PINK_BEAN_SQUAD_REJOIN_ASK), "{t:?}");
    assert_eq!(t.prompts, [(RECRUITER, 1)], "{t:?}");

    // Between Twilight and Daybreak's portal leads to the gate.
    let mut seen = Trace::default();
    let c = &mut leader.connection;
    assert_eq!(warp(c, PINK_BEAN_OUT, "", &mut seen).await, PINK_BEAN_OUT);
    touch_script_portal(&mut leader, PINK_BEAN_OUT, "in00").await;
    arrives(&mut leader, Trace::default(), PINK_BEAN_GATE).await;
}

#[tokio::test]
async fn the_third_and_fourth_to_fall_are_warned_and_the_fifth_ends_the_expedition() {
    let _turn = EXPEDITION.lock().await;
    let mut leader = adventurer().await;
    let mut member = adventurer().await;
    let mut lt = set_out(&mut leader, &mut member).await;
    let line = |key| (PINK, net::content::text().get(key, &[]));
    let (careful, last_stand, over) = (
        line(keys::PINK_BEAN_FALLEN_CAREFUL),
        line(keys::PINK_BEAN_FALLEN_LAST_STAND),
        line(keys::PINK_BEAN_FALLEN_OVER),
    );

    // Deaths in the Forgotten Twilight count too.
    for fallen in 1..=4 {
        assert_eq!(die(&mut member).await, PINK_BEAN_REVIVE);
        read_for(&mut leader.connection, &mut lt, 300).await;
        let heard = |b| lt.broadcasts.contains(b);
        assert_eq!(
            (heard(&careful), heard(&last_stand)),
            (fallen >= 3, fallen >= 4),
            "{fallen}: {lt:?}"
        );
    }
    assert!(!lt.broadcasts.contains(&over), "{lt:?}");

    // The fifth: everyone hears it is over and goes out.
    fall(&mut member).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.broadcasts.contains(&over) && t.arrivals.iter().any(|a| a.0 == PINK_BEAN_OUT)
    })
    .await;
    // The gate takes a new expedition once the copy closes, 2 s on.
    tokio::time::sleep(Duration::from_secs(3)).await;
}
