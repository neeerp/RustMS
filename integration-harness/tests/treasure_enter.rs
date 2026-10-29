//! MV's Lair (Cosmic `TreasurePQ`): Charles in its lobby off New Leaf City
//! takes a party of four to six, all of level 140 or more, into the
//! dungeon. A party with anyone under 140 is turned away. There a doll
//! drops a secret letter, and the tunnel on stays blocked until the leader
//! hands the dungeon's Charles fifteen of them, which clears the stage.
//! Milla lets a member back out to the lobby. In the cave the rock brings
//! out MV, whose fall wins the run and takes the team on to the treasure
//! room, on its own clock, whose chests spray their loot, and then out. The world runs events 60 times
//! as fast (`RUSTMS_EVENT_SCALE`).

use integration_harness::database::{
    grant_stack, replace_item, set_job, set_str, set_weapon_attack,
};
use integration_harness::events::{maps, party, talk, told};
use integration_harness::hunting::{attack, hit_reactor, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    SECRET_LETTER, SECRET_LETTERS_WANTED, TREASURE_CAVE, TREASURE_CHARLES, TREASURE_DUNGEON,
    TREASURE_LOBBY, TREASURE_MV, TREASURE_ROCK, TREASURE_ROOM,
};
use net::npc_scripts::CHARLES;
use net::packet::op::RecvOpcode;
use net::reactors::TREASURE_CHEST;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MILLA: i32 = 9220019;
const LETTER_DOLL: i32 = 6741015;
/// Under the most a level-140 character with 32000 STR and a 1999-attack
/// sword may hit for, and more than MV's 400,000 HP.
const HIT: i32 = 5_000_000;

/// A new character of `level`, holding `letters`, logged in at the lobby;
/// one `strong` hits very hard.
async fn waiting(level: i16, letters: i16, strong: bool) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 112, level, 50).await.unwrap();
    if letters > 0 {
        grant_stack(id, 4, 1, SECRET_LETTER, letters).await.unwrap();
    }
    if strong {
        replace_item(id, -1, -11, 1302000).await.unwrap();
        set_weapon_attack(id, 1_999).await.unwrap();
        set_str(id, 32_000).await.unwrap();
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
async fn charles_takes_140s_in_whose_letters_open_the_tunnel_and_mvs_fall_wins() {
    // One of 139 is too low for the rest.
    let mut team = Vec::new();
    for level in [140, 140, 140, 139] {
        team.push(waiting(level, 0, false).await);
    }
    party(&mut team).await;
    let refused = enter(&mut team[0]).await;
    assert!(!refused.warps.contains(&TREASURE_DUNGEON), "{refused:?}");
    drop(team);

    // Five, so that one may leave and the run go on.
    let mut team = vec![waiting(140, SECRET_LETTERS_WANTED, false).await];
    for strong in [false, false, true, false] {
        team.push(waiting(140, 0, strong).await);
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
    let left = team.remove(2);
    traces.remove(2);

    // The rock brings out MV, whose fall wins it for everyone.
    let (player, trace) = (&mut team[2], &mut traces[2]);
    trace.warps.clear();
    tunnel(player, trace).await;
    until(&mut player.connection, trace, |t| {
        t.warps.contains(&TREASURE_CAVE)
            && t.field
                .reactors
                .iter()
                .any(|r| r.reactor_id == TREASURE_ROCK)
    })
    .await;
    let rock = *trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == TREASURE_ROCK)
        .unwrap();
    let c = &mut player.connection;
    send(c, walk_to(rock.x, rock.y)).await;
    for _ in 0..8 {
        if !trace.field.alive(TREASURE_MV).is_empty() {
            break;
        }
        send(c, hit_reactor(rock.object_id)).await;
        read_for(c, trace, 600).await;
    }
    until(c, trace, |t| !t.field.alive(TREASURE_MV).is_empty()).await;
    let mv = trace.field.alive(TREASURE_MV);
    assert_eq!(mv.len(), 1, "{trace:?}");
    for _ in 0..10 {
        if trace.field.killed.contains(&mv[0]) {
            break;
        }
        send(c, attack(mv[0], HIT)).await;
        read_for(c, trace, 500).await;
    }
    assert!(trace.field.killed.contains(&mv[0]), "MV lives: {trace:?}");

    // Each, wherever they are, gets the stage's EXP and goes on to the
    // treasure room on its own clock; the one who left gets nothing.
    for (player, trace) in team.iter_mut().zip(traces.iter_mut()) {
        until(&mut player.connection, trace, |t| {
            t.gains.iter().any(|&exp| exp >= 100_000) && t.warps.contains(&TREASURE_ROOM)
        })
        .await;
        read_for(&mut player.connection, trace, 300).await;
        assert!(
            trace.countdowns.iter().any(|c| (8..=10).contains(c)),
            "{:?}",
            trace.countdowns
        );
    }
    let mut left = left;
    let mut seen = Trace::default();
    read_for(&mut left.connection, &mut seen, 500).await;
    assert!(!seen.gains.iter().any(|&exp| exp >= 100_000), "{seen:?}");
    assert!(!seen.warps.contains(&TREASURE_ROOM), "{seen:?}");

    // A chest there sprays its loot with mesos.
    let (player, trace) = (&mut team[0], &mut traces[0]);
    until(&mut player.connection, trace, |t| {
        t.field
            .reactors
            .iter()
            .any(|r| r.reactor_id == TREASURE_CHEST)
    })
    .await;
    let chest = *trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == TREASURE_CHEST)
        .unwrap();
    // Object ids start over on each map, so the dungeon's would mix in.
    trace.field.drops.clear();
    trace.field.reactors_removed.clear();
    let c = &mut player.connection;
    send(c, walk_to(chest.x, chest.y)).await;
    for _ in 0..8 {
        if trace
            .field
            .reactors_removed
            .iter()
            .any(|r| r.0 == chest.object_id)
        {
            break;
        }
        send(c, hit_reactor(chest.object_id)).await;
        read_for(c, trace, 300).await;
    }
    until(c, trace, |t| {
        let sprayed = &t.field.drops;
        sprayed.len() >= 15 && sprayed.iter().any(|d| d.meso)
    })
    .await;

    // Once its time is up, out to the lobby (this trace has not been there).
    until(&mut player.connection, trace, |t| {
        t.warps.contains(&TREASURE_LOBBY)
    })
    .await;
}
