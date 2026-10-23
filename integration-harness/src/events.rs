//! Helpers for party events (`tests/kpq*.rs`, `tests/hpq*.rs`,
//! `tests/lpq*.rs`, `tests/opq*.rs`, `tests/zpq*.rs`): a party
//! gathered in the event's lobby, NPC talk on the event's maps, and getting
//! the party in.

use crate::database::{grant_stack, replace_item, set_job, set_str, set_weapon_attack};
use crate::hunting::{hit_reactor, walk_to, MapReactor};
use crate::preconditions::load_harness_config_or_fail;
use crate::quests::answer;
use crate::trace::{send, step, until, warp, Trace};
use crate::{login_to_world_session, relogin, WorldSession};
use game_data::GameData;
use net::events::{Event, HPQ, KPQ, LPQ, OPQ, ZPQ};
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
pub const RED_SIGN: i32 = 2040034;
pub const ANDERSON: i32 = 2040047;
pub const WONKY: i32 = 2013000;
pub const EAK: i32 = 2013001;
pub const ADOBIS: i32 = 2030008;
pub const AURA: i32 = 2032002;
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
    talk_to(1_000_000_000 + index as i32)
}

/// TALK_TO_NPC: click the NPC with object id `object_id`, such as one that
/// came out on the map.
pub fn talk_to(object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(object_id).unwrap();
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

/// Pass the party's lead to `member_id`.
pub fn pass_lead(member_id: i32) -> Packet {
    let mut p = party_operation(6);
    p.write_int(member_id).unwrap();
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
    waiting_with(lobby, level, 0, items, false).await
}

/// A new character at `level` with `job`, logged in and waiting on
/// `lobby`.
pub async fn waiting_as(lobby: i32, level: i16, job: i16) -> WorldSession {
    waiting_with(lobby, level, job, &[], false).await
}

/// A new character at `level`, as `waiting_in`, with a sword and the
/// strength to kill the event's monsters in a blow or two.
pub async fn armed_in(lobby: i32, level: i16) -> WorldSession {
    waiting_with(lobby, level, 0, &[], true).await
}

async fn waiting_with(
    lobby: i32,
    level: i16,
    job: i16,
    items: &[(i32, i16)],
    armed: bool,
) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, level, 50).await.unwrap();
    for (slot, &(item_id, quantity)) in (1..).zip(items) {
        grant_stack(id, 4, slot, item_id, quantity).await.unwrap();
    }
    if armed {
        replace_item(id, -1, -11, 1302000).await.unwrap();
        set_weapon_attack(id, 500).await.unwrap();
        set_str(id, 999).await.unwrap();
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

/// The party's leader, first, takes it into the Dimensional Crack from the
/// Red Sign, once the party before them is out, and everyone arrives there;
/// returns what each saw up to their arrival.
pub async fn enter_lpq(team: &mut [WorldSession]) -> Vec<Trace> {
    enter_from_menu(team, &LPQ, RED_SIGN).await
}

/// The party's leader, first, takes it into the Tower of Goddess from
/// Wonky, as `enter_lpq` does.
pub async fn enter_opq(team: &mut [WorldSession]) -> Vec<Trace> {
    let mut traces = enter_from_menu(team, &OPQ, WONKY).await;
    // Eak tells each of them what to bring him.
    for (player, trace) in team.iter_mut().zip(&mut traces) {
        until(&mut player.connection, trace, |t| {
            t.prompts.contains(&(EAK, 0))
        })
        .await;
    }
    traces
}

/// The party's leader, first, takes it into the Dead Mine from Adobis, as
/// `enter_lpq` does; each needs a chief's approval for Zakum.
pub async fn enter_zpq(team: &mut [WorldSession]) -> Vec<Trace> {
    enter_from_menu(team, &ZPQ, ADOBIS).await
}

/// The leader picks the first choice of `npc`'s menu in `event`'s lobby
/// until it takes the party in, and everyone arrives at its entry.
async fn enter_from_menu(team: &mut [WorldSession], event: &Event, npc: i32) -> Vec<Trace> {
    let name = team[0].character_name.clone();
    let mut seen = None;
    for _ in 0..5 {
        step(&mut team[0].connection, &name, talk(event.lobby, npc)).await;
        let trace = step(&mut team[0].connection, &name, answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == npc) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "{npc} never let the party in");
    let mut traces = Vec::new();
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&event.entry)
        })
        .await;
        traces.push(trace);
    }
    traces
}

/// The trace of `player` sending `packet`, up to the barrier.
pub async fn told(player: &mut WorldSession, packet: Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

/// Whether an NPC said the line `key` in `trace`.
pub fn said(trace: &Trace, key: &str) -> bool {
    let text = net::content::text().get(key, &[]);
    trace.prompt_texts.iter().any(|said| said.contains(&text))
}

/// Whether `trace` shows a stage cleared.
pub fn cleared(trace: &Trace) -> bool {
    trace
        .field_effects
        .iter()
        .any(|(_, e)| e == "quest/party/clear")
}

/// Whether `trace` shows a wrong answer to a stage's puzzle.
pub fn wrong(trace: &Trace) -> bool {
    trace
        .field_effects
        .iter()
        .any(|(_, e)| e == "quest/party/wrong_kor")
}

/// Go to `map` and wait to see its reactor `reactor_id`.
pub async fn go_to(player: &mut WorldSession, map: i32, reactor_id: i32) -> (Trace, MapReactor) {
    let mut seen = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut seen).await, map);
    until(&mut player.connection, &mut seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == reactor_id)
    })
    .await;
    let reactor = *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == reactor_id)
        .unwrap();
    (seen, reactor)
}

/// Break open the stone box `stone`, out after Eak's fade-in, for `piece`:
/// it takes four hits.
pub async fn break_open(
    player: &mut WorldSession,
    seen: &mut Trace,
    stone: MapReactor,
    piece: i32,
) {
    let c = &mut player.connection;
    send(c, walk_to(stone.x, stone.y)).await;
    for state in 2..=4u8 {
        send(c, hit_reactor(stone.object_id)).await;
        until(c, seen, |t| {
            t.field.reactor_hits.contains(&(stone.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(stone.object_id)).await;
    until(c, seen, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == stone.object_id)
            && t.field.item_drop(piece).is_some()
    })
    .await;
}
