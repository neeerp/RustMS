//! The battle with The Boss in Showa's gang hideout: a leader of level 100
//! with the Lady Boss's Comb gathers an expedition at Konpei (9120201) in
//! the Armory and takes it into a copy of the arena. The comb dropped at
//! the arena's statue brings out the first bodyguard, who dies into the
//! second, and that into The Boss, whose death wins the battle. Konpei in
//! the arena hands each member a prize and takes them on to the clear map,
//! where the Slot Machine stands if no member died. The harness lets two
//! set out (`RUSTMS_EXPEDITION_MIN`).

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_str, set_weapon_attack,
};
use integration_harness::events::{cleared, clears, said, talk, throw_down, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    LADY_BOSS_COMB, SHOWA_ARENA, SHOWA_ARMORY, SHOWA_BODYGUARD, SHOWA_CLEAR, SHOWA_REWARDS,
    SHOWA_TOWN, SLOT_MACHINE, THE_BOSS,
};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const RECRUITER: i32 = 9120201;
const KONPEI: i32 = 9120202;
const KONPEI_CLEAR: i32 = 9120203;
const BODYGUARD_B: i32 = 9_400_113;
/// One of the arena's own mobs, whose touch hurts.
const GANGSTER: i32 = 9_400_103;
const PINK: u8 = 5;
const HP: i32 = 0x400;
/// Under the most a level-100 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new character of `level`, holding `combs` Lady Boss's Combs, logged in
/// in the Armory. One with combs hits very hard.
async fn adventurer(level: i16, combs: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.unwrap();
    if combs > 0 {
        grant_stack(id, 4, 1, LADY_BOSS_COMB, combs).await.unwrap();
        replace_item(id, -1, -11, 1302000).await.unwrap();
        set_weapon_attack(id, 1_999).await.unwrap();
        set_str(id, 32_000).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, SHOWA_ARMORY, "", &mut seen).await, SHOWA_ARMORY);
    session
}

fn recruiter() -> Packet {
    talk(SHOWA_ARMORY, RECRUITER)
}

/// The leader gathers a squad that `member` joins, and sets out: both
/// arrive in the arena.
async fn set_out(leader: &mut WorldSession, member: &mut WorldSession) -> (Trace, Trace) {
    let t = told(leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)], "{t:?}");
    let lt = told(leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::SQUAD_OPENED), "{lt:?}");
    told(leader, answer(0, 1, None)).await;
    let t = told(member, recruiter()).await;
    assert!(said(&t, keys::SHOWA_SQUAD_JOIN_ASK), "{t:?}");
    told(member, answer(1, 1, None)).await;
    told(member, answer(0, 1, None)).await;
    let t = told(leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 4)], "{t:?}");
    let mut lt = told(leader, answer(4, 1, Some(1))).await;
    let mut mt = Trace::default();
    for (player, t) in [(&mut *leader, &mut lt), (&mut *member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            t.arrivals.iter().any(|a| a.0 == SHOWA_ARENA)
        })
        .await;
    }
    (lt, mt)
}

/// Hit `object_id` until `seen` sees it die, five hits at a time.
async fn beat(c: &mut MapleTestConnection, name: &str, seen: &mut Trace, object_id: i32) {
    for _ in 0..60 {
        if seen.field.killed.contains(&object_id) {
            return;
        }
        for _ in 0..5 {
            send(c, attack(object_id, HIT)).await;
        }
        barrier(c, name, seen).await;
    }
    panic!("{object_id} lives: {seen:?}");
}

/// The leader drops a comb at the statue, and beats the bodyguards and
/// The Boss: the battle is won.
async fn win(leader: &mut WorldSession, lt: &mut Trace) {
    let name = leader.character_name.clone();
    let c = &mut leader.connection;
    send(c, walk_to(458, 124)).await;
    send(c, throw_down(1, 1)).await;
    for mob_id in [SHOWA_BODYGUARD, BODYGUARD_B, THE_BOSS] {
        until(c, lt, |t| !t.field.alive(mob_id).is_empty()).await;
        let oid = lt.field.alive(mob_id)[0];
        beat(c, &name, lt, oid).await;
    }
    until(c, lt, cleared).await;
    read_for(c, lt, 1_000).await;
    assert_eq!(clears(lt), 1, "{lt:?}");
}

/// `player` takes Konpei's prize after the win, and goes on to the clear
/// map, where it waits a little to see what is there.
async fn prize(player: &mut WorldSession) -> Trace {
    let t = told(player, talk(SHOWA_ARENA, KONPEI)).await;
    assert!(said(&t, keys::SHOWA_KONPEI_ARENA_WON), "{t:?}");
    let mut t = told(player, answer(0, 1, None)).await;
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == SHOWA_CLEAR)
    })
    .await;
    // A prize may stack onto one won before, so look at the gain's line.
    let got =
        (t.items.iter()).any(|&(item_id, n)| n > 0 && SHOWA_REWARDS.iter().any(|r| r.0 == item_id));
    assert!(got, "{t:?}");
    read_for(&mut player.connection, &mut t, 1500).await;
    t
}

/// `player`, on the clear map, goes back to town with Konpei.
async fn home(player: &mut WorldSession) {
    told(player, talk(SHOWA_CLEAR, KONPEI_CLEAR)).await;
    let mut t = told(player, answer(0, 1, None)).await;
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == SHOWA_TOWN)
    })
    .await;
}

/// A touch of `mob_id`'s for `damage` (TAKE_DAMAGE from its body).
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

#[tokio::test]
async fn the_comb_brings_out_the_boss_whose_death_wins_konpei_s_prizes() {
    let mut leader = adventurer(100, 2).await;
    let mut member = adventurer(100, 0).await;
    let no_casualties = net::content::text().get(keys::SHOWA_NO_CASUALTIES, &[]);

    // Without the comb, no one may lead one.
    let t = told(&mut member, recruiter()).await;
    assert!(said(&t, keys::SHOWA_SQUAD_NO_COMB), "{t:?}");
    told(&mut member, answer(0, 1, None)).await;

    // With it, the leader gathers the member and both go in. Before the
    // win, Konpei in the arena offers only the way out.
    let (mut lt, mut mt) = set_out(&mut leader, &mut member).await;
    let t = told(&mut member, talk(SHOWA_ARENA, KONPEI)).await;
    assert!(said(&t, keys::SHOWA_KONPEI_ARENA_LEAVE_ASK), "{t:?}");
    told(&mut member, answer(1, 0, None)).await;

    // The comb, the bodyguards, The Boss: won, with no one lost, so the
    // team hears of the Slot Machine, which stands on the clear map.
    win(&mut leader, &mut lt).await;
    until(&mut member.connection, &mut mt, |t| {
        cleared(t) && t.broadcasts.contains(&(PINK, no_casualties.clone()))
    })
    .await;
    let t = prize(&mut member).await;
    assert!(!t.field.alive(SLOT_MACHINE).is_empty(), "{t:?}");
    prize(&mut leader).await;
    home(&mut member).await;
    home(&mut leader).await;

    // Once both are out the channel may gather another. This time the
    // member dies in the arena first: no Slot Machine.
    for player in [&mut leader, &mut member] {
        let mut t = Trace::default();
        let c = &mut player.connection;
        assert_eq!(warp(c, SHOWA_ARMORY, "", &mut t).await, SHOWA_ARMORY);
    }
    let (mut lt, _) = set_out(&mut leader, &mut member).await;
    let mut dead = Trace::default();
    for _ in 0..40 {
        send(&mut member.connection, touched(20, GANGSTER)).await;
        read_for(&mut member.connection, &mut dead, 150).await;
        if dead.stat(HP) == Some(0) {
            break;
        }
    }
    assert_eq!(dead.stat(HP), Some(0), "{dead:?}");
    win(&mut leader, &mut lt).await;
    let t = prize(&mut leader).await;
    assert!(t.field.alive(SLOT_MACHINE).is_empty(), "{t:?}");
    let heard = (lt.broadcasts.iter()).chain(&t.broadcasts);
    assert!(!heard.into_iter().any(|b| b.1 == no_casualties), "{lt:?}");
}
