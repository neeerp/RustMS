//! A record of what the server told one client, and helpers that read
//! packets into it.

use crate::connection::MapleTestConnection;
use crate::hunting::Field;
use crate::packets::{build_all_chat, build_change_map, decode_set_field_warp};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::collections::BTreeMap;
use std::io::Cursor;
use tokio::time::{timeout, Duration, Instant};

/// STAT_CHANGED mask bits the tests read.
pub mod stat {
    pub const LEVEL: i32 = 0x10;
    pub const HP: i32 = 0x400;
    pub const MAX_HP: i32 = 0x800;
    pub const AP: i32 = 0x4000;
    pub const EXP: i32 = 0x10000;
    pub const MESO: i32 = 0x40000;
}

/// Wire widths of the STAT_CHANGED values, in mask order.
const STAT_WIDTHS: [(i32, u8); 19] = [
    (0x1, 1),
    (0x2, 4),
    (0x4, 4),
    (0x8, 8),
    (0x10, 1),
    (0x20, 2),
    (0x40, 2),
    (0x80, 2),
    (0x100, 2),
    (0x200, 2),
    (0x400, 2),
    (0x800, 2),
    (0x1000, 2),
    (0x2000, 2),
    (0x4000, 2),
    (0x8000, 2),
    (0x10000, 4),
    (0x20000, 2),
    (0x40000, 4),
];

/// Everything a test reads back from the server for one client.
#[derive(Debug, Default)]
pub struct Trace {
    /// Latest state per quest from SHOW_STATUS_INFO mode 1.
    pub states: BTreeMap<i16, u8>,
    /// Every quest state change in order.
    pub progress: Vec<(i16, u8)>,
    /// UPDATE_QUEST_INFO successes: quest, NPC, next quest.
    pub results: Vec<(i16, i32, i16)>,
    /// Latest progress string (kill counts) per started quest.
    pub kills: BTreeMap<i16, String>,
    /// EXP gains from SHOW_STATUS_INFO mode 3.
    pub gains: Vec<i32>,
    /// Every STAT_CHANGED, as values by mask bit.
    pub stat_updates: Vec<BTreeMap<i32, i64>>,
    /// Item gains and losses shown in chat (SHOW_ITEM_GAIN_INCHAT 3).
    pub items: Vec<(i32, i32)>,
    /// Items added to the bags: (inventory type, slot, item id).
    pub added: Vec<(u8, i16, i32)>,
    /// "Quest Clear" effects (SHOW_ITEM_GAIN_INCHAT 9).
    pub cleared: usize,
    /// UI images shown on the player (SHOW_ITEM_GAIN_INCHAT 0x17).
    pub images: Vec<String>,
    /// "You can't get anymore items." (SHOW_STATUS_INFO 0/-1).
    pub inventory_full: usize,
    /// NPC dialogue prompts: (npc, message type).
    pub prompts: Vec<(i32, u8)>,
    /// Maps warped to.
    pub warps: Vec<i32>,
    /// Mobs and drops on the current field.
    pub field: Field,
}

impl Trace {
    pub fn observe(&mut self, p: &Packet) {
        self.field.observe(p);
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        let op = p.opcode();
        if op == SendOpcode::ShowStatusInfo as i16 {
            match r.read_byte().unwrap() {
                // Only the "inventory full" notice; item gains from
                // pickups are not recorded.
                0 if r.read_byte().unwrap() == 0xFF => self.inventory_full += 1,
                1 => {
                    let (quest, state) = (r.read_short().unwrap(), r.read_byte().unwrap());
                    if state == 1 {
                        self.kills.insert(quest, r.read_str_with_length().unwrap());
                    }
                    if self.states.get(&quest) != Some(&state) {
                        self.progress.push((quest, state));
                    }
                    self.states.insert(quest, state);
                }
                3 => {
                    r.read_byte().unwrap();
                    self.gains.push(r.read_int().unwrap());
                }
                _ => {}
            }
        } else if op == SendOpcode::ShowItemGainInChat as i16 {
            match r.read_byte().unwrap() {
                3 => {
                    for _ in 0..r.read_byte().unwrap() {
                        self.items
                            .push((r.read_int().unwrap(), r.read_int().unwrap()));
                    }
                }
                9 => self.cleared += 1,
                0x17 => self.images.push(r.read_str_with_length().unwrap()),
                _ => {}
            }
        } else if op == SendOpcode::StatChange as i16 {
            r.read_byte().unwrap();
            let mask = r.read_int().unwrap();
            let mut values = BTreeMap::new();
            for (bit, width) in STAT_WIDTHS.into_iter().filter(|(bit, _)| mask & bit != 0) {
                let value = match width {
                    1 => i64::from(r.read_byte().unwrap()),
                    2 => i64::from(r.read_short().unwrap()),
                    4 => i64::from(r.read_int().unwrap()),
                    _ => r.read_long().unwrap(),
                };
                values.insert(bit, value);
            }
            self.stat_updates.push(values);
        } else if op == SendOpcode::InventoryOperation as i16 {
            r.read_byte().unwrap();
            for _ in 0..r.read_byte().unwrap() {
                let mode = r.read_byte().unwrap();
                let kind = r.read_byte().unwrap();
                let slot = r.read_short().unwrap();
                match mode {
                    0 => {
                        r.read_byte().unwrap();
                        self.added.push((kind, slot, r.read_int().unwrap()));
                        // The rest of the item is not needed.
                        return;
                    }
                    1 | 2 => {
                        r.read_short().unwrap();
                    }
                    _ => {}
                }
            }
        } else if op == SendOpcode::UpdateQuestInfo as i16 {
            assert_eq!(r.read_byte().unwrap(), 8, "only success results are sent");
            self.results.push((
                r.read_short().unwrap(),
                r.read_int().unwrap(),
                r.read_short().unwrap(),
            ));
        } else if op == SendOpcode::NpcDialogue as i16 {
            r.read_byte().unwrap();
            let npc = r.read_int().unwrap();
            self.prompts.push((npc, r.read_byte().unwrap()));
        } else if op == SendOpcode::SetField as i16 && p.bytes.get(6) == Some(&0) {
            // A map change rather than entering the world.
            self.warps.push(decode_set_field_warp(p).unwrap().map_id);
        }
    }

    /// The latest value of a stat (a `stat` mask bit).
    pub fn stat(&self, bit: i32) -> Option<i64> {
        self.stat_updates
            .iter()
            .rev()
            .find_map(|u| u.get(&bit).copied())
    }

    /// Every value a stat was updated to, in order.
    pub fn stat_values(&self, bit: i32) -> Vec<i64> {
        self.stat_updates
            .iter()
            .filter_map(|u| u.get(&bit).copied())
            .collect()
    }
}

pub async fn send(c: &mut MapleTestConnection, p: Packet) {
    c.send_packet(p, "trace").await.unwrap();
}

/// Read one packet into `trace`, failing after 5 seconds.
pub async fn read(c: &mut MapleTestConnection, trace: &mut Trace) -> Packet {
    let p = timeout(Duration::from_secs(5), c.read_packet("trace"))
        .await
        .expect("packet timeout")
        .expect("read packet")
        .packet;
    trace.observe(&p);
    p
}

/// Read until `done` holds, failing after 15 seconds.
pub async fn until(c: &mut MapleTestConnection, trace: &mut Trace, done: impl Fn(&Trace) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !done(trace) {
        let envelope = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("trace"),
        )
        .await
        .unwrap_or_else(|_| panic!("timed out: {trace:?}"))
        .unwrap();
        trace.observe(&envelope.packet);
    }
}

/// Read whatever arrives within `ms`.
pub async fn read_for(c: &mut MapleTestConnection, trace: &mut Trace, ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(ms);
    while let Ok(Ok(envelope)) = timeout(
        deadline.saturating_duration_since(Instant::now()),
        c.read_packet("trace"),
    )
    .await
    {
        trace.observe(&envelope.packet);
    }
}

/// Wait for our own chat line to come back, recording packets on the way.
/// The field relays it only after earlier work for this client completed.
pub async fn barrier(c: &mut MapleTestConnection, marker: &str, trace: &mut Trace) {
    send(c, build_all_chat(marker, 0).unwrap()).await;
    loop {
        let p = read(c, trace).await;
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

/// Send `packet` and return what the server answered once it was handled.
pub async fn step(c: &mut MapleTestConnection, marker: &str, packet: Packet) -> Trace {
    let mut trace = Trace::default();
    send(c, packet).await;
    barrier(c, marker, &mut trace).await;
    trace
}

/// Warp (debug map id, or -1 and a portal name) and return the new map id.
/// The field record starts over.
pub async fn warp(c: &mut MapleTestConnection, map: i32, portal: &str, trace: &mut Trace) -> i32 {
    send(c, build_change_map(map, portal).unwrap()).await;
    let before = trace.warps.len();
    while trace.warps.len() == before {
        read(c, trace).await;
    }
    trace.field = Field::default();
    *trace.warps.last().unwrap()
}
