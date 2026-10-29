//! The battle with the Balrog in its tomb: a leader of level 50 or more
//! gathers an expedition at Mu Young (1061014) in the temple's entrance and
//! takes it into a copy of the tomb, where the Balrog's body, right arm and
//! left claw wait. The claw soon gives way to the left arm; the body takes
//! no harm until both arms are dead, and its death wins the battle and
//! brings out the sealed Balrog. Mu Young in the tomb takes the winners on
//! to the clear map, whose box sprays loot and whose way out gives each a
//! Piece of Balrog Leather, which the Suspicious Man in the Balrog's hall
//! trades for a scroll. Everyone in the tomb counts the kill Balrog and the
//! Seal (2239) asks for.
//! The harness lets two set out (`RUSTMS_EXPEDITION_MIN`) and runs the
//! claw's minute in a second (`RUSTMS_EVENT_SCALE`).

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    replace_item, set_job, set_str, set_weapon_attack, start_quest,
};
use integration_harness::events::{cleared, clears, said, talk, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::hunting::{attack, hit_reactor, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{BALROG_CLEAR, BALROG_LEATHER, BALROG_TOMB, MU_YOUNG_MAP};
use net::mobs::{BALROG_BODY, BALROG_CLAW, BALROG_LEFT_ARM, BALROG_RIGHT_ARM, SEALED_BALROG};
use net::reactors::BALROG_BOX;
use packet::Packet;
use tokio::time::Duration;

const MU_YOUNG: i32 = 1061014;
const KEEPER: i32 = 1061018;
const SUSPICIOUS_MAN: i32 = 1061016;
/// The Balrog's hall, where the Suspicious Man stands.
const HALL: i32 = 105100000;
/// Balrog and the Seal.
const SEAL_QUEST: i16 = 2239;
/// The Balrog's Twilight Scroll 5%, the last of the man's scrolls.
const TWILIGHT_SCROLL: i32 = 2040739;
/// Under the most a level-100 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new character of level 100 on Balrog and the Seal, logged in at the
/// temple's entrance. A `strong` one hits very hard.
async fn adventurer(strong: bool) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 100, 50).await.unwrap();
    start_quest(id, SEAL_QUEST).await.unwrap();
    if strong {
        replace_item(id, -1, -11, 1302000).await.unwrap();
        set_weapon_attack(id, 1_999).await.unwrap();
        set_str(id, 32_000).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, MU_YOUNG_MAP, "", &mut seen).await, MU_YOUNG_MAP);
    session
}

fn mu_young() -> Packet {
    talk(MU_YOUNG_MAP, MU_YOUNG)
}

/// The leader gathers a squad that `member` joins, and sets out: both
/// arrive in the tomb.
async fn set_out(leader: &mut WorldSession, member: &mut WorldSession) -> (Trace, Trace) {
    let t = told(leader, mu_young()).await;
    assert_eq!(t.prompts, [(MU_YOUNG, 1)], "{t:?}");
    let lt = told(leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::SQUAD_OPENED), "{lt:?}");
    told(leader, answer(0, 1, None)).await;
    let t = told(member, mu_young()).await;
    assert!(said(&t, keys::BALROG_SQUAD_JOIN_ASK), "{t:?}");
    told(member, answer(1, 1, None)).await;
    told(member, answer(0, 1, None)).await;
    let t = told(leader, mu_young()).await;
    assert_eq!(t.prompts, [(MU_YOUNG, 4)], "{t:?}");
    let mut lt = told(leader, answer(4, 1, Some(1))).await;
    let mut mt = Trace::default();
    for (player, t) in [(&mut *leader, &mut lt), (&mut *member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            t.arrivals.iter().any(|a| a.0 == BALROG_TOMB)
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

#[tokio::test]
async fn the_balrog_s_arms_shield_its_body_whose_death_wins_the_leather() {
    let mut leader = adventurer(true).await;
    let mut member = adventurer(false).await;
    let (mut lt, mut mt) = set_out(&mut leader, &mut member).await;

    // Before the win, Mu Young in the tomb offers only the way out.
    let t = told(&mut member, talk(BALROG_TOMB, KEEPER)).await;
    assert!(said(&t, keys::BALROG_KEEPER_LEAVE_ASK), "{t:?}");
    told(&mut member, answer(1, 0, None)).await;

    // The claw gives way to the left arm.
    let name = leader.character_name.clone();
    let c = &mut leader.connection;
    until(c, &mut lt, |t| {
        !t.field.alive(BALROG_LEFT_ARM).is_empty() && t.field.alive(BALROG_CLAW).is_empty()
    })
    .await;
    // The body takes no harm while an arm is out.
    let body = lt.field.alive(BALROG_BODY)[0];
    for _ in 0..5 {
        send(c, attack(body, HIT)).await;
    }
    barrier(c, &name, &mut lt).await;
    assert!(!lt.field.killed.contains(&body), "{lt:?}");
    for arm in [BALROG_RIGHT_ARM, BALROG_LEFT_ARM] {
        let oid = lt.field.alive(arm)[0];
        beat(c, &name, &mut lt, oid).await;
    }
    beat(c, &name, &mut lt, body).await;

    // Won: the clear, the world's line, and the sealed Balrog; both count
    // the seal's kill.
    let victory = net::content::text().get(keys::BALROG_VICTORY, &[]);
    until(c, &mut lt, |t| {
        cleared(t)
            && !t.field.alive(SEALED_BALROG).is_empty()
            && t.progress.contains(&(SEAL_QUEST, 1))
    })
    .await;
    until(&mut member.connection, &mut mt, |t| {
        cleared(t)
            && t.item_lines.contains(&(victory.clone(), 0))
            && t.progress.contains(&(SEAL_QUEST, 1))
    })
    .await;
    read_for(&mut member.connection, &mut mt, 1_000).await;
    assert_eq!(clears(&mt), 1, "{mt:?}");

    // Mu Young takes each on to the clear map, whose box sprays its loot
    // and whose way out gives the leather and leads back to the temple's
    // entrance.
    for (player, hits) in [(&mut member, false), (&mut leader, true)] {
        let t = told(player, talk(BALROG_TOMB, KEEPER)).await;
        assert!(said(&t, keys::BALROG_KEEPER_WON), "{t:?}");
        let mut t = told(player, answer(0, 1, None)).await;
        let c = &mut player.connection;
        until(c, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == BALROG_CLEAR)
                && t.field.reactors.iter().any(|r| r.reactor_id == BALROG_BOX)
        })
        .await;
        if hits {
            let name = player.character_name.clone();
            let c = &mut player.connection;
            let reactors = t.field.reactors.iter();
            let reactor = *reactors
                .clone()
                .find(|r| r.reactor_id == BALROG_BOX)
                .unwrap();
            send(c, walk_to(reactor.x, reactor.y)).await;
            for _ in 0..8 {
                if t.field
                    .reactors_removed
                    .iter()
                    .any(|r| r.0 == reactor.object_id)
                {
                    break;
                }
                send(c, hit_reactor(reactor.object_id)).await;
                barrier(c, &name, &mut t).await;
            }
            until(c, &mut t, |t| t.field.drops.len() >= 15).await;
        }
        touch_script_portal(player, BALROG_CLEAR, "out99").await;
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == MU_YOUNG_MAP)
        })
        .await;
        assert!(t.added.iter().any(|a| a.2 == BALROG_LEATHER), "{t:?}");
    }

    // The Suspicious Man in the hall takes the member's leather for the
    // scroll they pick.
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut member.connection, HALL, "", &mut seen).await,
        HALL
    );
    let t = told(&mut member, talk(HALL, SUSPICIOUS_MAN)).await;
    assert!(said(&t, keys::BALROG_SCROLLS_HELLO), "{t:?}");
    let t = told(&mut member, answer(4, 1, Some(0))).await;
    assert!(said(&t, keys::BALROG_SCROLLS_PICK), "{t:?}");
    let mut t = told(&mut member, answer(4, 1, Some(11))).await;
    until(&mut member.connection, &mut t, |t| {
        t.added.iter().any(|a| a.2 == TWILIGHT_SCROLL)
    })
    .await;
    assert!(said(&t, keys::BALROG_SCROLLS_GIVEN), "{t:?}");
}
