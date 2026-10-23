//! An expedition against Zakum: a holder of an Eye of Fire gathers a squad
//! at the Entrance to Zakum (2030013), whoever's party its members are in,
//! and takes it to an altar of its own. While it fights, the gate on the
//! Door to Zakum is shut and the recruiter turns others away; a member who
//! stepped out, or wasn't there at the start, may go in. Once Zakum falls
//! the gate opens and the channel may gather a new squad. The harness lets
//! two set out (`RUSTMS_EXPEDITION_MIN`).

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_str, set_weapon_attack, start_quest,
};
use integration_harness::events::{said, talk, throw_down, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const DOOR_TO_ZAKUM: i32 = 211042300;
const ENTRANCE_TO_ZAKUM: i32 = 211042400;
const ALTAR_MAP: i32 = 280030000;
const RECRUITER: i32 = 2030013;
const GATE: i32 = 2118002;
const ALTAR: i32 = 2111001;
const EYE_OF_FIRE: i32 = 4001017;
const BODY: i32 = 8800000;
const PINK: u8 = 5;
/// The chiefs' approval and the trials done (`npc_scripts::ZAKUM_APPROVED`
/// and `ZAKUM_TRIALS_DONE`).
const APPROVED: i16 = 32400;
const TRIALS_DONE: i16 = 32401;
/// Under the most a level-50 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new level-50 character holding an Eye of Fire if `eye`, through the
/// way to Zakum if `approved`, hitting very hard if `armed`, logged in on
/// `map`.
async fn adventurer(map: i32, eye: bool, approved: bool, armed: bool) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.unwrap();
    if eye {
        grant_stack(id, 4, 1, EYE_OF_FIRE, 1).await.unwrap();
    }
    if approved {
        start_quest(id, APPROVED).await.unwrap();
        start_quest(id, TRIALS_DONE).await.unwrap();
    }
    if armed {
        replace_item(id, -1, -11, 1302000).await.unwrap();
        set_weapon_attack(id, 1_999).await.unwrap();
        set_str(id, 32_000).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(warp(&mut session.connection, map, "", &mut seen).await, map);
    session
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn recruiter() -> Packet {
    talk(ENTRANCE_TO_ZAKUM, RECRUITER)
}

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

/// Go to the Door to Zakum and see its gate, which is `state`.
async fn at_the_gate(player: &mut WorldSession, state: u8) -> (Trace, i32) {
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, DOOR_TO_ZAKUM, "", &mut seen).await, DOOR_TO_ZAKUM);
    until(c, &mut seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == GATE)
    })
    .await;
    let gate = seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == GATE)
        .unwrap();
    assert_eq!(gate.state, state, "{:?}", seen.field.reactors);
    let gate = gate.object_id;
    (seen, gate)
}

#[tokio::test]
async fn an_expedition_shuts_the_gate_takes_its_own_back_and_opens_it_after_zakum() {
    let mut leader = adventurer(ENTRANCE_TO_ZAKUM, true, false, true).await;
    let mut member = adventurer(ENTRANCE_TO_ZAKUM, false, false, false).await;
    let mut other = adventurer(ENTRANCE_TO_ZAKUM, true, true, false).await;
    let mut late = adventurer(ENTRANCE_TO_ZAKUM, false, false, false).await;
    let (leader_name, member_name) = (leader.character_name.clone(), member.character_name.clone());

    // Without an Eye of Fire, no one may lead an expedition.
    let t = told(&mut member, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 0)]);
    assert!(said(&t, keys::ZAKUM_SQUAD_NO_EYE), "{t:?}");
    told(&mut member, answer(0, 1, None)).await;

    // The leader opens one, and those around hear of it.
    let t = told(&mut leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)]);
    let mut lt = told(&mut leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::ZAKUM_SQUAD_OPENED), "{lt:?}");
    until(&mut leader.connection, &mut lt, |t| {
        !t.countdowns.is_empty()
    })
    .await;
    let opened =
        net::content::text().get(keys::ZAKUM_SQUAD_NOTICE_OPENED, &[("name", &leader_name)]);
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.item_lines.contains(&(opened.clone(), 0))
    })
    .await;
    told(&mut leader, answer(0, 1, None)).await;

    // The member joins it, though in no party with the leader.
    let t = told(&mut member, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)]);
    assert!(said(&t, keys::ZAKUM_SQUAD_JOIN_ASK), "{t:?}");
    let mut mt = told(&mut member, answer(1, 1, None)).await;
    assert!(said(&mt, keys::ZAKUM_SQUAD_JOINED), "{mt:?}");
    until(&mut member.connection, &mut mt, |t| {
        !t.countdowns.is_empty()
    })
    .await;
    told(&mut member, answer(0, 1, None)).await;
    let joined =
        net::content::text().get(keys::ZAKUM_SQUAD_NOTICE_JOINED, &[("name", &member_name)]);
    let mut lt = Trace::default();
    until(&mut leader.connection, &mut lt, |t| {
        t.item_lines.contains(&(joined.clone(), 0))
    })
    .await;

    // A third joins, then wanders off before the start.
    told(&mut late, recruiter()).await;
    told(&mut late, answer(1, 1, None)).await;
    told(&mut late, answer(0, 1, None)).await;
    let mut t = Trace::default();
    warp(&mut late.connection, DOOR_TO_ZAKUM, "", &mut t).await;

    // Someone else waits at the gate, open for now.
    let (mut ot, gate) = at_the_gate(&mut other, 0).await;

    // The leader sets out, and both arrive on the altar.
    let t = told(&mut leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 4)]);
    let mut lt = told(&mut leader, answer(4, 1, Some(1))).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.arrivals.iter().any(|a| a.0 == ALTAR_MAP)
    })
    .await;
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.arrivals.iter().any(|a| a.0 == ALTAR_MAP)
    })
    .await;

    // The gate shuts, and turns away the next who would pass.
    until(&mut other.connection, &mut ot, |t| {
        t.field.reactor_hits.contains(&(gate, 1))
    })
    .await;
    let blocked = net::content::text().get(keys::ZAKUM_GATE_BLOCKED, &[]);
    let t = told(&mut other, enter("ps00")).await;
    assert!(
        t.broadcasts.contains(&(PINK, blocked)) && t.arrivals.is_empty(),
        "{t:?}"
    );
    // The recruiter turns them away too.
    let mut t = Trace::default();
    warp(&mut other.connection, ENTRANCE_TO_ZAKUM, "", &mut t).await;
    let t = told(&mut other, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 0)]);
    assert!(said(&t, keys::ZAKUM_SQUAD_TAKEN), "{t:?}");
    told(&mut other, answer(0, 1, None)).await;

    // The one who wandered off is still on the roster, and comes in late.
    let mut t = Trace::default();
    warp(&mut late.connection, ENTRANCE_TO_ZAKUM, "", &mut t).await;
    let t = told(&mut late, recruiter()).await;
    assert!(said(&t, keys::ZAKUM_SQUAD_REJOIN_ASK), "{t:?}");
    let mut t = told(&mut late, answer(1, 1, None)).await;
    until(&mut late.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == ALTAR_MAP)
            && t.players.iter().any(|p| p.character_name == leader_name)
    })
    .await;

    // The member steps out, and goes back in, to the leader's altar.
    let mut t = Trace::default();
    warp(&mut member.connection, ENTRANCE_TO_ZAKUM, "", &mut t).await;
    let t = told(&mut member, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)]);
    assert!(said(&t, keys::ZAKUM_SQUAD_REJOIN_ASK), "{t:?}");
    let mut mt = told(&mut member, answer(1, 1, None)).await;
    until(&mut member.connection, &mut mt, |t| {
        t.arrivals.iter().any(|a| a.0 == ALTAR_MAP)
            && t.players.iter().any(|p| p.character_name == leader_name)
    })
    .await;

    // The leader brings out Zakum on their altar and beats it, while the
    // other waits at the shut gate.
    let (mut ot, gate) = at_the_gate(&mut other, 1).await;
    let c = &mut leader.connection;
    until(c, &mut lt, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == ALTAR)
    })
    .await;
    let altar = *lt
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == ALTAR)
        .unwrap();
    send(c, walk_to(altar.x - 30, altar.y - 36)).await;
    send(c, throw_down(1, 1)).await;
    until(c, &mut lt, |t| t.field.mobs.len() == 9).await;
    let arms: Vec<i32> = lt
        .field
        .mobs
        .iter()
        .filter(|&&(_, id)| id != BODY)
        .map(|&(oid, _)| oid)
        .collect();
    for arm in arms {
        beat(c, &mut lt, arm).await;
    }
    for mob_id in [BODY, 8800001, 8800002] {
        until(c, &mut lt, |t| !t.field.alive(mob_id).is_empty()).await;
        let oid = lt.field.alive(mob_id)[0];
        beat(c, &mut lt, oid).await;
    }
    until(c, &mut lt, |t| {
        t.field_effects.contains(&(4, "Party1/Clear".to_string()))
    })
    .await;

    // Zakum fallen, the gate opens.
    until(&mut other.connection, &mut ot, |t| {
        t.field.reactor_hits.contains(&(gate, 0))
    })
    .await;
    let t = told(&mut other, enter("ps00")).await;
    assert_eq!(
        t.arrivals.iter().map(|a| a.0).collect::<Vec<_>>(),
        [ENTRANCE_TO_ZAKUM]
    );

    // Its expedition won, the channel may gather another, though the
    // winners are still on their altar.
    for _ in 0..20 {
        let t = told(&mut other, recruiter()).await;
        if t.prompts == [(RECRUITER, 1)] {
            return;
        }
        assert!(said(&t, keys::ZAKUM_SQUAD_TAKEN), "{t:?}");
        told(&mut other, answer(0, 1, None)).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("the expedition is still under way");
}
