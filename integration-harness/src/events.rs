//! Helpers for party events (`tests/kpq*.rs`, `tests/hpq*.rs`): a party
//! gathered in the event's lobby, NPC talk on the event's maps, and getting
//! the party in.

use crate::database::{grant_stack, set_job};
use crate::preconditions::load_harness_config_or_fail;
use crate::quests::answer;
use crate::trace::{send, step, until, warp, Trace};
use crate::{login_to_world_session, relogin, WorldSession};
use game_data::GameData;
use net::events::{HPQ, KPQ};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

pub const LAKELIS: i32 = 9020000;
pub const CLOTO: i32 = 9020001;
pub const NELLA: i32 = 9020002;
pub const TORY: i32 = 1012112;
pub const TOMMY: i32 = 1012113;
pub const GROWLIE: i32 = 1012114;
const OP_JOIN: u8 = 0x0F;

/// The maps, from the Map.nx the server reads.
pub fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

/// TALK_TO_NPC: click `npc` on `map`.
pub fn talk(map: i32, npc: i32) -> Packet {
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

/// Throw `quantity` of the Etc stack in `slot` on the ground.
pub fn throw_down(slot: i16, quantity: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(4).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(quantity).unwrap();
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

/// Leave the party, which its leader's leaving disbands.
pub fn leave_party() -> Packet {
    party_operation(2)
}

fn accept(party_id: i32) -> Packet {
    let mut p = party_operation(3);
    p.write_int(party_id).unwrap();
    p
}

/// A new character at `level` holding `items` (Etc stacks, from slot 1),
/// logged in and waiting in Kerning City.
pub async fn in_lobby(level: i16, items: &[(i32, i16)]) -> WorldSession {
    waiting_in(KPQ.lobby, level, items).await
}

/// A new character at `level`, holding `items` in their Etc tab, logged in
/// and waiting on `lobby`.
pub async fn waiting_in(lobby: i32, level: i16, items: &[(i32, i16)]) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.unwrap();
    for (slot, &(item_id, quantity)) in (1..).zip(items) {
        grant_stack(id, 4, slot, item_id, quantity).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut session.connection, lobby, "", &mut trace).await,
        lobby
    );
    session
}

/// The first player invites the rest into a party.
pub async fn party(players: &mut [WorldSession]) {
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

/// The party's leader, first, takes it into Kerning City's party quest,
/// once the party before them is out (tests/kpq.rs), and everyone arrives
/// on its first stage.
pub async fn enter_kpq(team: &mut [WorldSession]) {
    let name = team[0].character_name.clone();
    let mut asked = false;
    for _ in 0..5 {
        let trace = step(&mut team[0].connection, &name, talk(KPQ.lobby, LAKELIS)).await;
        if trace.prompts == [(LAKELIS, 1)] {
            asked = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(asked, "Lakelis never let the party in");
    let mut seen = Some(step(&mut team[0].connection, &name, answer(1, 1, None)).await);
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&KPQ.entry)
        })
        .await;
    }
}

/// The party's leader, first, takes it up to Primrose Hill, once the party
/// before them is out, and everyone arrives there; returns what each saw
/// up to their arrival.
pub async fn enter_hpq(team: &mut [WorldSession]) -> Vec<Trace> {
    let name = team[0].character_name.clone();
    let mut asked = false;
    for _ in 0..5 {
        step(&mut team[0].connection, &name, talk(HPQ.lobby, TORY)).await;
        let trace = step(&mut team[0].connection, &name, answer(4, 1, Some(0))).await;
        if trace.prompts == [(TORY, 1)] {
            asked = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(asked, "Tory never let the party up");
    let mut seen = Some(step(&mut team[0].connection, &name, answer(1, 1, None)).await);
    let mut traces = Vec::new();
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&HPQ.entry)
        })
        .await;
        traces.push(trace);
    }
    traces
}
