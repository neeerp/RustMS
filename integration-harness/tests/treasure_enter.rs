//! MV's Lair (Cosmic `TreasurePQ`): Charles in its lobby off New Leaf City
//! takes a party of four to six, all of level 140 or more, into the
//! dungeon. A party with anyone under 140 is turned away. There a doll
//! drops a secret letter, and the tunnel on stays blocked until the leader
//! hands the dungeon's Charles fifteen of them, which clears the stage.
//! Milla lets a member back out to the lobby.

use integration_harness::database::{grant_stack, set_job};
use integration_harness::events::{maps, party, talk, told};
use integration_harness::hunting::{hit_reactor, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    SECRET_LETTER, SECRET_LETTERS_WANTED, TREASURE_CAVE, TREASURE_CHARLES, TREASURE_DUNGEON,
    TREASURE_LOBBY,
};
use net::npc_scripts::CHARLES;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MILLA: i32 = 9220019;
const LETTER_DOLL: i32 = 6741015;

/// A new character of `level`, holding `letters`, logged in at the lobby.
async fn waiting(level: i16, letters: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 112, level, 50).await.unwrap();
    if letters > 0 {
        grant_stack(id, 4, 1, SECRET_LETTER, letters).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, TREASURE_LOBBY, "", &mut seen).await,
        TREASURE_LOBBY
    );
    session
}

/// The leader asks Charles to go in; returns what came back.
async fn enter(leader: &mut WorldSession) -> Trace {
    told(leader, talk(TREASURE_LOBBY, CHARLES)).await;
    let mut trace = Trace::default();
    send(&mut leader.connection, answer(4, 1, Some(0))).await;
    until(&mut leader.connection, &mut trace, |t| {
        t.warps.contains(&TREASURE_DUNGEON) || t.prompts.iter().any(|p| p.0 == CHARLES)
    })
    .await;
    trace
}

/// CHANGE_MAP_SPECIAL: walk to the dungeon's tunnel, `out00`, and press Up.
async fn tunnel(player: &mut WorldSession, seen: &mut Trace) {
    let field = maps().field(TREASURE_DUNGEON).unwrap();
    let out = field.portal_by_name("out00").unwrap();
    send(&mut player.connection, walk_to(out.x as i16, out.y as i16)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("out00").unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
    read_for(&mut player.connection, seen, 1500).await;
}

#[tokio::test]
async fn charles_takes_four_of_140_in_and_their_letters_open_the_tunnel() {
    // One of 139 is too low for the rest.
    let mut team = Vec::new();
    for level in [140, 140, 140, 139] {
        team.push(waiting(level, 0).await);
    }
    party(&mut team).await;
    let refused = enter(&mut team[0]).await;
    assert!(!refused.warps.contains(&TREASURE_DUNGEON), "{refused:?}");
    drop(team);

    let mut team = vec![waiting(140, SECRET_LETTERS_WANTED).await];
    for _ in 0..3 {
        team.push(waiting(140, 0).await);
    }
    party(&mut team).await;
    let mut seen = None;
    for _ in 0..20 {
        let trace = enter(&mut team[0]).await;
        if trace.warps.contains(&TREASURE_DUNGEON) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Charles never let the party in");
    let mut traces = Vec::new();
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&TREASURE_DUNGEON)
                && t.field.reactors.iter().any(|r| r.reactor_id == LETTER_DOLL)
        })
        .await;
        traces.push(trace);
    }

    // A doll with a letter drops it.
    let (player, trace) = (&mut team[1], &mut traces[1]);
    let doll = *trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == LETTER_DOLL)
        .unwrap();
    let c = &mut player.connection;
    send(c, walk_to(doll.x, doll.y)).await;
    for _ in 0..8 {
        if trace
            .field
            .reactors_removed
            .iter()
            .any(|r| r.0 == doll.object_id)
        {
            break;
        }
        send(c, hit_reactor(doll.object_id)).await;
        read_for(c, trace, 600).await;
    }
    until(c, trace, |t| {
        t.field
            .drops
            .iter()
            .any(|d| !d.meso && d.value == SECRET_LETTER)
    })
    .await;

    // The tunnel is blocked until the stage is cleared.
    let (player, trace) = (&mut team[2], &mut traces[2]);
    tunnel(player, trace).await;
    assert!(!trace.warps.contains(&TREASURE_CAVE), "{trace:?}");

    // Anyone but the leader is sent off; the leader's letters clear it.
    let member = told(&mut team[3], talk(TREASURE_DUNGEON, TREASURE_CHARLES)).await;
    assert!(member.prompts.iter().any(|p| p.0 == TREASURE_CHARLES));
    assert!(member.removed.is_empty(), "{member:?}");
    let leader = &mut team[0];
    let mut handed = told(leader, talk(TREASURE_DUNGEON, TREASURE_CHARLES)).await;
    until(&mut leader.connection, &mut handed, |t| {
        !t.removed.is_empty() && t.gains.iter().any(|&exp| exp >= 60_000)
    })
    .await;

    let (player, trace) = (&mut team[2], &mut traces[2]);
    trace.warps.clear();
    tunnel(player, trace).await;
    until(&mut player.connection, trace, |t| {
        t.warps.contains(&TREASURE_CAVE)
    })
    .await;

    // Milla in the cave sends a member back to the lobby.
    let member = &mut team[2];
    let asked = told(member, talk(TREASURE_CAVE, MILLA)).await;
    assert!(asked.prompts.iter().any(|p| p.0 == MILLA), "{asked:?}");
    let mut trace = told(member, answer(1, 1, None)).await;
    until(&mut member.connection, &mut trace, |t| {
        t.warps.contains(&TREASURE_LOBBY)
    })
    .await;
}
