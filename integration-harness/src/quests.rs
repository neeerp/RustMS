//! Driving the v83 quest flow: the client sends `QUEST_ACTION` and the
//! server answers with quest progress, rewards, and an action result.

use crate::connection::MapleTestConnection;
use crate::packets::{build_all_chat, build_change_map, decode_set_field_warp};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::collections::BTreeMap;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

pub const START: u8 = 1;
pub const COMPLETE: u8 = 2;
pub const FORFEIT: u8 = 3;

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

/// What the server reported about quests while a test read packets.
#[derive(Debug, Default)]
pub struct QuestTrace {
    /// Latest state per quest from SHOW_STATUS_INFO mode 1.
    pub states: BTreeMap<i16, u8>,
    /// Every state change in order.
    pub progress: Vec<(i16, u8)>,
    /// EXP gains from SHOW_STATUS_INFO mode 3.
    pub gains: Vec<i32>,
    /// Level, AP, and EXP from the last stat update.
    pub stats: Option<(u8, i16, i32)>,
    /// UPDATE_QUEST_INFO successes: quest, NPC, next quest.
    pub results: Vec<(i16, i32, i16)>,
}

impl QuestTrace {
    pub fn observe(&mut self, p: &Packet) {
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        if p.opcode() == SendOpcode::ShowStatusInfo as i16 {
            match r.read_byte().unwrap() {
                1 => {
                    let (quest, state) = (r.read_short().unwrap(), r.read_byte().unwrap());
                    self.states.insert(quest, state);
                    self.progress.push((quest, state));
                }
                3 => {
                    r.read_byte().unwrap();
                    self.gains.push(r.read_int().unwrap());
                }
                mode => panic!("unexpected status mode {mode}"),
            }
        } else if p.opcode() == SendOpcode::StatChange as i16 {
            r.read_byte().unwrap();
            let mask = r.read_int().unwrap();
            if mask != 0 {
                // LEVEL | AVAILABLEAP | EXP
                assert_eq!(mask, 0x10 | 0x4000 | 0x10000);
                self.stats = Some((
                    r.read_byte().unwrap(),
                    r.read_short().unwrap(),
                    r.read_int().unwrap(),
                ));
            }
        } else if p.opcode() == SendOpcode::UpdateQuestInfo as i16 {
            assert_eq!(r.read_byte().unwrap(), 8, "only success results are sent");
            self.results.push((
                r.read_short().unwrap(),
                r.read_int().unwrap(),
                r.read_short().unwrap(),
            ));
        }
    }
}

pub async fn send(c: &mut MapleTestConnection, p: Packet) {
    c.send_packet(p, "quest").await.unwrap();
}

pub async fn read(c: &mut MapleTestConnection, trace: &mut QuestTrace) -> Packet {
    let p = timeout(Duration::from_secs(5), c.read_packet("quest"))
        .await
        .expect("packet timeout")
        .expect("read packet")
        .packet;
    trace.observe(&p);
    p
}

/// Send a quest action and wait until the server has handled it (and any
/// packets it produced), using a chat line as a barrier.
pub async fn act(
    c: &mut MapleTestConnection,
    trace: &mut QuestTrace,
    marker: &str,
    action: u8,
    quest_id: i16,
    npc_id: i32,
) {
    send(c, build_quest_action(action, quest_id, npc_id)).await;
    barrier(c, marker, trace).await;
}

/// Wait for our own chat line to come back, recording quest packets on the way.
/// The field relays it only after earlier work for this client completed.
pub async fn barrier(c: &mut MapleTestConnection, marker: &str, trace: &mut QuestTrace) {
    send(c, build_all_chat(marker, 0).unwrap()).await;
    loop {
        let p = read(c, trace).await;
        assert_ne!(
            p.opcode(),
            SendOpcode::NpcDialogue as i16,
            "quests must not produce server dialogue"
        );
        if p.opcode() == SendOpcode::ChatText as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            if r.read_str_with_length().unwrap() == marker {
                return;
            }
        }
    }
}

/// Warp (debug map id or portal name) and return the new map id.
pub async fn warp(
    c: &mut MapleTestConnection,
    map: i32,
    portal: &str,
    trace: &mut QuestTrace,
) -> i32 {
    send(c, build_change_map(map, portal).unwrap()).await;
    loop {
        let p = read(c, trace).await;
        if p.opcode() == SendOpcode::SetField as i16 {
            return decode_set_field_warp(&p).unwrap().map_id;
        }
    }
}
