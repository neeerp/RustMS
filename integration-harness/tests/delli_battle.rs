//! The Corsairs' "Mission: Free the Monsters" (6410): Shulynch's way leads
//! to Delli, who takes a party of one or two in to protect Willi on Save
//! Delli!. Willi's going loses it at once; Willi still there when the time
//! runs out wins it, and the other mobs go. Only then does the way out lead
//! on, with the record the quest completes on with Delli. The fight lasts 6
//! seconds here (`RUSTMS_EVENT_SCALE`).

use game_data::server_content::keys;
use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{cleared, maps, party, talk, told};
use integration_harness::hunting::{move_mob, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_completion};
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{DELLI, NAUTILUS_HIDEOUT, SAVE_DELLI, WILLI};
use net::npc_scripts::DELLI as DELLI_NPC;
use net::packet::op::RecvOpcode;
use net::scripts::{DELLI_QUEST, DELLI_RECORD, DELLI_SAVED};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::sync::Mutex;
use tokio::time::Duration;

const PINK: u8 = 5;

/// One fight at a time: a channel holds one Delli's rescue, and the loss
/// test's six seconds are tight enough without the other's load.
static FIGHT: Mutex<()> = Mutex::const_new(());

/// A new level-120 Corsair on Delli's quest, logged in where the party
/// gathers, Looking for Delli 3.
async fn corsair() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 522, 120, 50).await.unwrap();
    start_quest(id, DELLI_QUEST).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, DELLI.lobby, "", &mut seen).await,
        DELLI.lobby
    );
    session
}

/// CHANGE_MAP_SPECIAL: press Up on Save Delli!'s way out, `s4mpt`.
fn way_out() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("s4mpt").unwrap();
    p.write_short(0).unwrap();
    p
}

/// MOB_DAMAGE_MOB_FRIENDLY as the v83 client sends it (0x667e40): mob
/// `attacker` ran into the friendly mob `victim` near `character_id`.
fn mob_damage_friendly(attacker: i32, character_id: i32, victim: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MobDamageMobFriendly as i16)
        .unwrap();
    p.write_int(attacker).unwrap();
    p.write_int(character_id).unwrap();
    p.write_int(victim).unwrap();
    p
}

/// Two Corsairs in a party, which its leader takes in from Delli's menu
/// once the party before them is out; returns what each saw on Save Delli!
/// by the time Willi and the other mobs were out and someone controlled
/// them.
async fn in_the_fight() -> (Vec<WorldSession>, Vec<Trace>) {
    let mut team = vec![corsair().await, corsair().await];
    party(&mut team).await;
    let mut seen = None;
    for _ in 0..20 {
        told(&mut team[0], talk(DELLI.lobby, DELLI_NPC)).await;
        let mut trace = Trace::default();
        send(&mut team[0].connection, answer(4, 1, Some(0))).await;
        until(&mut team[0].connection, &mut trace, |t| {
            t.warps.contains(&SAVE_DELLI) || t.prompts.iter().any(|p| p.0 == DELLI_NPC)
        })
        .await;
        if trace.warps.contains(&SAVE_DELLI) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Delli never let the party in");
    let mut traces = Vec::new();
    for player in team.iter_mut() {
        // Up to the warp a packet at a time, so that only Save Delli!'s
        // mobs are left.
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&SAVE_DELLI)
        })
        .await;
        trace.field = Default::default();
        until(&mut player.connection, &mut trace, |t| {
            t.field.mobs.iter().any(|&(_, mob)| mob == WILLI)
                && t.field.mobs.iter().any(|&(_, mob)| mob != WILLI)
        })
        .await;
        traces.push(trace);
    }
    for _ in 0..50 {
        let hostile = |t: &Trace| {
            t.field
                .mobs
                .iter()
                .any(|&(oid, mob)| mob != WILLI && t.field.controlled.contains(&oid))
        };
        if traces.iter().any(hostile) {
            break;
        }
        for (player, trace) in team.iter_mut().zip(traces.iter_mut()) {
            read_for(&mut player.connection, trace, 50).await;
        }
    }
    (team, traces)
}

fn willi(trace: &Trace) -> i32 {
    trace.field.mobs.iter().find(|m| m.1 == WILLI).unwrap().0
}

#[tokio::test]
async fn willi_still_there_when_the_time_runs_out_wins_and_opens_the_way_to_delli() {
    let _fight = FIGHT.lock().await;
    let (mut team, mut traces) = in_the_fight().await;
    let willi = willi(&traces[0]);
    let hostile: Vec<i32> = traces[0]
        .field
        .mobs
        .iter()
        .filter(|m| m.1 != WILLI)
        .map(|m| m.0)
        .collect();

    // Before the win the way out stays shut.
    let portal = maps().field(SAVE_DELLI).unwrap();
    let out = portal.portal_by_name("s4mpt").unwrap();
    told(&mut team[0], walk_to(out.x as i16, out.y as i16)).await;
    let shut = told(&mut team[0], way_out()).await;
    let not_won = net::content::text().get(keys::DELLI_NOT_WON, &[]);
    assert!(shut.broadcasts.contains(&(PINK, not_won)), "{shut:?}");
    assert!(!shut.warps.contains(&NAUTILUS_HIDEOUT));

    // The time runs out with Willi there: the party has won, and every
    // other mob leaves.
    for (player, trace) in team.iter_mut().zip(traces.iter_mut()) {
        until(&mut player.connection, trace, |t| {
            cleared(t) && hostile.iter().all(|oid| t.field.killed.contains(oid))
        })
        .await;
        assert!(hostile.iter().all(|oid| trace.field.leave_types[oid] == 1));
        assert!(!trace.field.killed.contains(&willi));
    }

    // Now the way out leads on to Delli, with the record.
    let mut seen = told(&mut team[0], way_out()).await;
    until(&mut team[0].connection, &mut seen, |t| {
        t.kills.get(&DELLI_RECORD).map(String::as_str) == Some(DELLI_SAVED)
            && t.warps.contains(&NAUTILUS_HIDEOUT)
    })
    .await;

    // Delli completes the quest on it.
    let field = maps().field(NAUTILUS_HIDEOUT).unwrap();
    let at = field
        .map_npcs
        .iter()
        .find(|n| n.npc_id == DELLI_NPC)
        .unwrap();
    told(&mut team[0], walk_to(at.x, at.y)).await;
    let mut seen = told(
        &mut team[0],
        build_quest_completion(DELLI_QUEST, DELLI_NPC, -1),
    )
    .await;
    until(&mut team[0].connection, &mut seen, |t| {
        t.states.get(&DELLI_QUEST) == Some(&2)
    })
    .await;
}

#[tokio::test]
async fn willi_going_loses_the_fight_at_once_and_sends_the_party_back() {
    let _fight = FIGHT.lock().await;
    // Willi takes a mob's hit at most every 1/60 s here and has 95,000 HP,
    // so a busy machine can run out the six seconds first: the party has
    // won then, and another goes in.
    for _ in 0..3 {
        if willi_goes().await {
            return;
        }
    }
    panic!("the time ran out before Willi went, three times");
}

/// A party goes in and walks a mob into Willi until it goes, and then all
/// go back; false if the time ran out first.
async fn willi_goes() -> bool {
    let (mut team, mut traces) = in_the_fight().await;
    let willi = willi(&traces[0]);
    let at = traces[0].field.mob_positions[&willi];

    // A mob its controller walks into Willi hurts it until it goes. Its
    // controller is the one whose move the server answers: control can pass
    // on before a trace has read it.
    let (attacker, _) = *(traces[0].field.mobs.iter())
        .find(|&&(_, mob)| mob != WILLI)
        .unwrap();
    let mut controller = None;
    for _ in 0..10 {
        for (i, (player, trace)) in team.iter_mut().zip(traces.iter_mut()).enumerate() {
            send(&mut player.connection, move_mob(attacker, at, false, 0, 0)).await;
            read_for(&mut player.connection, trace, 100).await;
            if trace.field.move_responses.contains(&attacker) {
                controller = Some(i);
            }
        }
        if controller.is_some() {
            break;
        }
    }
    let controller = controller.expect("a player controls the mob");
    let character_id = team[controller].character_id;
    let c = &mut team[controller].connection;
    let t = &mut traces[controller];
    for _ in 0..1000 {
        if t.field.killed.contains(&willi) || t.warps.contains(&DELLI.exit) || cleared(t) {
            break;
        }
        send(c, mob_damage_friendly(attacker, character_id, willi)).await;
        read_for(c, t, 20).await;
    }
    if !t.field.killed.contains(&willi) && cleared(t) {
        return false;
    }
    assert!(t.field.killed.contains(&willi), "{t:?}");
    assert_eq!(t.field.leave_types[&willi], 0);

    // The party has lost: everyone goes back to Looking for Delli 3.
    for (player, trace) in team.iter_mut().zip(traces.iter_mut()) {
        until(&mut player.connection, trace, |t| {
            t.warps.contains(&DELLI.exit)
        })
        .await;
        assert!(!cleared(trace));
    }
    true
}
