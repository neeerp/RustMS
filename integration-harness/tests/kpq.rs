//! Kerning City's party quest, its first slice: Lakelis takes a party of
//! three or four at levels 21 to 30 from Kerning City into its own copy of
//! the quest's maps, one party at a time per channel; Nella lets players
//! out; and the quest ends for everyone when too few remain or its time is
//! up. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts 30 seconds.

use game_data::GameData;
use integration_harness::database::set_job;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::KPQ;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const LAKELIS: i32 = 9020000;
const NELLA: i32 = 9020002;
const OP_JOIN: u8 = 0x0F;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

fn party_operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartyOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

fn invite(name: &str) -> Packet {
    let mut p = party_operation(4);
    p.write_str_with_length(name).unwrap();
    p
}

fn accept(party_id: i32) -> Packet {
    let mut p = party_operation(3);
    p.write_int(party_id).unwrap();
    p
}

/// A new character at `level`, logged in and waiting in Kerning City.
async fn player(level: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut session.connection, KPQ.lobby, "", &mut trace).await,
        KPQ.lobby
    );
    session
}

/// The first player invites the rest into a party.
async fn party(players: &mut [WorldSession]) {
    let (leader, members) = players.split_first_mut().unwrap();
    for member in members {
        send(&mut leader.connection, invite(&member.character_name)).await;
        let mut trace = Trace::default();
        until(&mut member.connection, &mut trace, |t| {
            !t.party_invites.is_empty()
        })
        .await;
        let party_id = trace.party_invites[0].0;
        send(&mut member.connection, accept(party_id)).await;
        until(&mut member.connection, &mut trace, |t| {
            t.party_ops.contains(&OP_JOIN)
        })
        .await;
    }
}

/// What `player` is told on talking to `npc` on `map`: the NPC and the
/// prompt's message type.
async fn talk_to(player: &mut WorldSession, map: i32, npc: i32) -> Vec<(i32, u8)> {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, talk(map, npc))
        .await
        .prompts
}

/// Say yes to the open prompt; returns the trace up to the barrier.
async fn yes(player: &mut WorldSession) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, answer(1, 1, None)).await
}

/// Wait until `player` is moved to `map`, and return the trace.
async fn moved_to(player: &mut WorldSession, map: i32) -> Trace {
    let mut trace = Trace::default();
    until(&mut player.connection, &mut trace, |t| {
        t.warps.contains(&map)
    })
    .await;
    trace
}

#[tokio::test]
async fn parties_play_kerning_city_party_quest_in_turn_in_their_own_copy() {
    let mut a = Vec::new();
    for _ in 0..3 {
        a.push(player(25).await);
    }
    let mut b = Vec::new();
    for _ in 0..4 {
        b.push(player(21).await);
    }
    party(&mut a).await;
    party(&mut b).await;

    // Only the party's leader may take it in.
    assert_eq!(talk_to(&mut a[1], KPQ.lobby, LAKELIS).await, [(LAKELIS, 0)]);

    // Lakelis asks the leader, and in they go together, on the clock.
    assert_eq!(talk_to(&mut a[0], KPQ.lobby, LAKELIS).await, [(LAKELIS, 1)]);
    yes(&mut a[0]).await;
    for player in a.iter_mut() {
        let mut trace = moved_to(player, KPQ.entry).await;
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        assert!((25..=30).contains(&trace.countdowns[0]), "{trace:?}");
    }

    // Another party must wait its turn.
    assert_eq!(talk_to(&mut b[0], KPQ.lobby, LAKELIS).await, [(LAKELIS, 0)]);

    // The party plays in its own copy: someone warping onto the map meets
    // nobody, and nobody meets them.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut b[0].connection, KPQ.entry, "", &mut trace).await,
        KPQ.entry
    );
    read_for(&mut b[0].connection, &mut trace, 500).await;
    assert!(trace.players.is_empty(), "{:?}", trace.players);
    let mut trace = Trace::default();
    read_for(&mut a[0].connection, &mut trace, 500).await;
    assert!(trace.players.is_empty(), "{:?}", trace.players);
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut b[0].connection, KPQ.lobby, "", &mut trace).await,
        KPQ.lobby
    );

    // Nella lets a member out, which leaves too few: the quest ends, and
    // the rest are sent out after them.
    assert_eq!(talk_to(&mut a[2], KPQ.entry, NELLA).await, [(NELLA, 1)]);
    yes(&mut a[2]).await;
    let mut trace = moved_to(&mut a[2], KPQ.exit).await;
    until(&mut a[2].connection, &mut trace, |t| t.clocks_stopped > 0).await;
    for player in a[..2].iter_mut() {
        moved_to(player, KPQ.exit).await;
    }

    // The next party may go in once the copy closes.
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(talk_to(&mut b[0], KPQ.lobby, LAKELIS).await, [(LAKELIS, 1)]);
    yes(&mut b[0]).await;
    let mut seconds = 0;
    for player in b.iter_mut() {
        let mut trace = moved_to(player, KPQ.entry).await;
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        seconds = trace.countdowns[0];
    }

    // One of four leaving still leaves enough to play on...
    assert_eq!(talk_to(&mut b[3], KPQ.entry, NELLA).await, [(NELLA, 1)]);
    yes(&mut b[3]).await;
    moved_to(&mut b[3], KPQ.exit).await;
    let mut trace = Trace::default();
    read_for(&mut b[0].connection, &mut trace, 500).await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);

    // ...until time is up, and everyone left is sent out.
    tokio::time::sleep(Duration::from_secs((seconds as u64).saturating_sub(5))).await;
    for player in b[..3].iter_mut() {
        moved_to(player, KPQ.exit).await;
    }

    // Nella sees them home from the exit.
    assert_eq!(talk_to(&mut b[0], KPQ.exit, NELLA).await, [(NELLA, 1)]);
    let trace = yes(&mut b[0]).await;
    assert_eq!(trace.warps, [KPQ.lobby]);
}
