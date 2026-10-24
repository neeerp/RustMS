//! The warriors' "Protecting Tylus" (6192): Tylus in the Chief's Residence
//! takes a party of one to four in to protect him, where he drops his
//! Trust every so often for those on the quest. The way out leads on to
//! him past the room only with it, and he sends them to El Nath, where the
//! quest completes on it; without it, the way leads out. The fight lasts
//! 10 seconds here (`RUSTMS_EVENT_SCALE`), and the Trust comes every 1.7.

use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{maps, party, talk, told};
use integration_harness::hunting::{pickup, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_completion};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    TYLUS, TYLUS_EXIT, TYLUS_LOBBY, TYLUS_MOB, TYLUS_QUEST, TYLUS_ROOM, TYLUS_THANKS, TYLUS_TRUST,
};
use net::npc_scripts::TYLUS as TYLUS_NPC;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// Tylus past the room, and El Nath.
const TYLUS_GRATEFUL: i32 = 2022004;
const EL_NATH: i32 = 211_000_000;

/// A new level-120 Hero on "Protecting Tylus", logged in at Tylus.
async fn hero() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 112, 120, 50).await.unwrap();
    start_quest(id, TYLUS_QUEST).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, TYLUS.lobby, "", &mut seen).await,
        TYLUS.lobby
    );
    session
}

/// CHANGE_MAP_SPECIAL: walk to the room's way out, `out01`, and press Up.
async fn way_out(player: &mut WorldSession, seen: &mut Trace) {
    let field = maps().field(TYLUS_ROOM).unwrap();
    let out = field.portal_by_name("out01").unwrap();
    send(&mut player.connection, walk_to(out.x as i16, out.y as i16)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("out01").unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
    until(&mut player.connection, seen, |t| {
        t.warps.contains(&TYLUS_EXIT) || t.warps.contains(&TYLUS_THANKS)
    })
    .await;
}

#[tokio::test]
async fn tyluss_trust_leads_past_the_room_to_complete_the_quest() {
    let mut team = vec![hero().await, hero().await];
    party(&mut team).await;

    // The leader takes the party in once any party before them is out.
    let mut seen = None;
    for _ in 0..20 {
        told(&mut team[0], talk(TYLUS_LOBBY, TYLUS_NPC)).await;
        let mut trace = Trace::default();
        send(&mut team[0].connection, answer(4, 1, Some(0))).await;
        until(&mut team[0].connection, &mut trace, |t| {
            t.warps.contains(&TYLUS_ROOM) || t.prompts.iter().any(|p| p.0 == TYLUS_NPC)
        })
        .await;
        if trace.warps.contains(&TYLUS_ROOM) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Tylus never let the party in");
    let mut traces = Vec::new();
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&TYLUS_ROOM)
        })
        .await;
        trace.field = Default::default();
        until(&mut player.connection, &mut trace, |t| {
            t.field.mobs.iter().any(|&(_, mob)| mob == TYLUS_MOB)
                && t.field.mobs.iter().any(|&(_, mob)| mob != TYLUS_MOB)
        })
        .await;
        traces.push(trace);
    }

    // Without the Trust, the way leads out.
    way_out(&mut team[1], &mut traces[1]).await;
    assert!(traces[1].warps.contains(&TYLUS_EXIT), "{:?}", traces[1]);
    assert!(!traces[1].warps.contains(&TYLUS_THANKS));

    // Tylus drops his Trust; with it, the way leads on to him past the
    // room.
    let (player, trace) = (&mut team[0], &mut traces[0]);
    let c = &mut player.connection;
    until(c, trace, |t| {
        t.field
            .drops
            .iter()
            .any(|d| !d.meso && d.value == TYLUS_TRUST && t.field.removals(d.object_id).is_empty())
    })
    .await;
    let drop = *trace
        .field
        .drops
        .iter()
        .find(|d| !d.meso && d.value == TYLUS_TRUST && trace.field.removals(d.object_id).is_empty())
        .unwrap();
    send(c, walk_to(drop.x, drop.y)).await;
    send(c, pickup(drop.object_id, drop.x, drop.y)).await;
    until(c, trace, |t| t.added.iter().any(|a| a.2 == TYLUS_TRUST)).await;
    way_out(player, trace).await;
    assert!(trace.warps.contains(&TYLUS_THANKS), "{trace:?}");

    // He thanks them and sends them to El Nath.
    let thanks = told(player, talk(TYLUS_THANKS, TYLUS_GRATEFUL)).await;
    assert_eq!(thanks.prompts, [(TYLUS_GRATEFUL, 0)], "{thanks:?}");
    assert_eq!(told(player, answer(0, 1, None)).await.warps, [EL_NATH]);

    // Tylus completes the quest on the Trust.
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut player.connection, TYLUS_LOBBY, "", &mut seen).await,
        TYLUS_LOBBY
    );
    let field = maps().field(TYLUS_LOBBY).unwrap();
    let at = field
        .map_npcs
        .iter()
        .find(|n| n.npc_id == TYLUS_NPC)
        .unwrap();
    told(player, walk_to(at.x, at.y)).await;
    let mut seen = told(player, build_quest_completion(TYLUS_QUEST, TYLUS_NPC, -1)).await;
    until(&mut player.connection, &mut seen, |t| {
        t.states.get(&TYLUS_QUEST) == Some(&2)
    })
    .await;
}
