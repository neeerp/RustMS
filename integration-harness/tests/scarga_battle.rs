//! The battle with Targa and Scarlion in Malaysia's Spooky World: a leader
//! of level 100 with a Spirit of Fantasy Theme Park gathers an expedition
//! at Aldol (9270047) at its entrance and takes it into a copy of the
//! Spooky World. A spirit dropped on each of its two reactors brings out
//! Targa or Scarlion, each of whom comes back twice; the deaths of both
//! last forms win the battle. Aldol in the Spooky World hands each member
//! a prize and takes them back out. The harness lets two set out
//! (`RUSTMS_EXPEDITION_MIN`).

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
    SCARGA_BOSSES, SCARGA_REWARDS, SCARLION, SPIRIT_OF_THEME_PARK, SPOOKY_ENTRANCE, SPOOKY_WORLD,
    TARGA,
};
use packet::Packet;
use tokio::time::Duration;

const RECRUITER: i32 = 9270047;
const ALDOL: i32 = 9201134;
const PINK: u8 = 5;
/// Under the most a level-100 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new character of `level`, holding `spirits` Spirits of Fantasy Theme
/// Park, logged in at the Spooky World's entrance. One with spirits hits
/// very hard.
async fn adventurer(level: i16, spirits: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.unwrap();
    // The spirit stacks one to a slot.
    for slot in 1..=spirits {
        grant_stack(id, 4, slot, SPIRIT_OF_THEME_PARK, 1)
            .await
            .unwrap();
    }
    if spirits > 0 {
        replace_item(id, -1, -11, 1302000).await.unwrap();
        set_weapon_attack(id, 1_999).await.unwrap();
        set_str(id, 32_000).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, SPOOKY_ENTRANCE, "", &mut seen).await,
        SPOOKY_ENTRANCE
    );
    session
}

fn recruiter() -> Packet {
    talk(SPOOKY_ENTRANCE, RECRUITER)
}

/// The leader gathers a squad that `member` joins, and sets out: both
/// arrive in the Spooky World.
async fn set_out(leader: &mut WorldSession, member: &mut WorldSession) -> (Trace, Trace) {
    let t = told(leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)], "{t:?}");
    let lt = told(leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::SQUAD_OPENED), "{lt:?}");
    told(leader, answer(0, 1, None)).await;
    let t = told(member, recruiter()).await;
    assert!(said(&t, keys::SCARGA_SQUAD_JOIN_ASK), "{t:?}");
    told(member, answer(1, 1, None)).await;
    told(member, answer(0, 1, None)).await;
    let t = told(leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 4)], "{t:?}");
    let mut lt = told(leader, answer(4, 1, Some(1))).await;
    let mut mt = Trace::default();
    for (player, t) in [(&mut *leader, &mut lt), (&mut *member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            t.arrivals.iter().any(|a| a.0 == SPOOKY_WORLD)
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

/// The leader drops a spirit on the reactor at `x`, hears `notice`, and
/// beats all three forms of the boss that comes out, from `first` to
/// `last`.
async fn bring_out_and_beat(
    leader: &mut WorldSession,
    lt: &mut Trace,
    x: i16,
    slot: i16,
    notice: &str,
    forms: [i32; 3],
) {
    let name = leader.character_name.clone();
    let c = &mut leader.connection;
    send(c, walk_to(x, 630)).await;
    send(c, throw_down(slot, 1)).await;
    let notice = net::content::text().get(notice, &[]);
    until(c, lt, |t| t.broadcasts.contains(&(PINK, notice.clone()))).await;
    for mob_id in forms {
        until(c, lt, |t| !t.field.alive(mob_id).is_empty()).await;
        let oid = lt.field.alive(mob_id)[0];
        beat(c, &name, lt, oid).await;
    }
}

/// `player` takes Aldol's prize after the win, and goes back out.
async fn prize(player: &mut WorldSession) {
    let t = told(player, talk(SPOOKY_WORLD, ALDOL)).await;
    assert!(said(&t, keys::SCARGA_ALDOL_WON), "{t:?}");
    let mut t = told(player, answer(0, 1, None)).await;
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == SPOOKY_ENTRANCE)
    })
    .await;
    let got = (t.items.iter())
        .any(|&(item_id, n)| n > 0 && SCARGA_REWARDS.iter().any(|r| r.0 == item_id));
    assert!(got, "{t:?}");
}

#[tokio::test]
async fn the_spirits_bring_out_targa_and_scarlion_whose_last_deaths_win_aldol_s_prizes() {
    let mut leader = adventurer(100, 2).await;
    let mut member = adventurer(100, 0).await;

    // Without the spirit, no one may lead one.
    let t = told(&mut member, recruiter()).await;
    assert!(said(&t, keys::SCARGA_SQUAD_NO_SPIRIT), "{t:?}");
    told(&mut member, answer(0, 1, None)).await;

    // With it, the leader gathers the member and both go in. Before the
    // win, Aldol in the Spooky World offers only the way out.
    let (mut lt, mut mt) = set_out(&mut leader, &mut member).await;
    let t = told(&mut member, talk(SPOOKY_WORLD, ALDOL)).await;
    assert!(said(&t, keys::SCARGA_ALDOL_LEAVE_ASK), "{t:?}");
    told(&mut member, answer(1, 0, None)).await;

    // Targa, in all three forms: not won yet.
    let [furious_targa, furious_scarlion] = SCARGA_BOSSES;
    let targa = [TARGA, TARGA + 1, furious_targa];
    let notice = keys::SCARGA_TARGA_SUMMONED;
    bring_out_and_beat(&mut leader, &mut lt, -510, 1, notice, targa).await;
    read_for(&mut leader.connection, &mut lt, 1_000).await;
    assert!(!cleared(&lt), "{lt:?}");

    // Then Scarlion: won, and every mob goes.
    let scarlion = [SCARLION, SCARLION + 1, furious_scarlion];
    let notice = keys::SCARGA_SCARLION_SUMMONED;
    bring_out_and_beat(&mut leader, &mut lt, -220, 2, notice, scarlion).await;
    until(&mut leader.connection, &mut lt, cleared).await;
    read_for(&mut leader.connection, &mut lt, 1_000).await;
    assert_eq!(clears(&lt), 1, "{lt:?}");
    until(&mut member.connection, &mut mt, cleared).await;

    prize(&mut member).await;
    prize(&mut leader).await;
}
