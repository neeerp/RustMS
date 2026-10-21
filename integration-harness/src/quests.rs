//! Driving the v83 quest flow: the client sends `QUEST_ACTION` and the
//! server answers with quest progress, rewards, and an action result.

use crate::connection::MapleTestConnection;
use crate::trace::{self, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

pub const START: u8 = 1;
pub const COMPLETE: u8 = 2;
pub const FORFEIT: u8 = 3;
/// Accept or finish a scripted quest; HeavenClient sends these from the
/// quest log and its info window.
pub const SCRIPT_START: u8 = 4;
pub const SCRIPT_END: u8 = 5;

/// `QUEST_ACTION` as HeavenClient sends it: the NPC for start/complete, then
/// the player's position.
pub fn build_quest_action(action: u8, quest_id: i16, npc_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::QuestAction as i16).unwrap();
    p.write_byte(action).unwrap();
    p.write_short(quest_id).unwrap();
    if action != FORFEIT {
        p.write_int(npc_id).unwrap();
        p.write_short(0).unwrap();
        p.write_short(0).unwrap();
    }
    p
}

/// A completion as v83 sends it: the NPC, the player's position, then the
/// picked reward (-1 for none).
pub fn build_quest_completion(quest_id: i16, npc_id: i32, choice: i32) -> Packet {
    let mut p = build_quest_action(COMPLETE, quest_id, npc_id);
    p.write_int(choice).unwrap();
    p
}

/// `NPC_TALK_MORE`: the answer to an NPC prompt of `kind`.
pub fn answer(kind: u8, response: i8, selection: Option<i32>) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcTalkMore as i16).unwrap();
    p.write_byte(kind).unwrap();
    p.write_byte(response as u8).unwrap();
    if let Some(selection) = selection {
        p.write_int(selection).unwrap();
    }
    p
}

/// Send a quest action and wait until the server has handled it (and any
/// packets it produced), using a chat line as a barrier.
pub async fn act(
    c: &mut MapleTestConnection,
    trace: &mut Trace,
    marker: &str,
    action: u8,
    quest_id: i16,
    npc_id: i32,
) {
    let prompts = trace.prompts.len();
    trace::send(c, build_quest_action(action, quest_id, npc_id)).await;
    trace::barrier(c, marker, trace).await;
    assert_eq!(
        trace.prompts.len(),
        prompts,
        "quests must not produce server dialogue"
    );
}
