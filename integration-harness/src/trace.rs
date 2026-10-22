//! A record of what the server told one client, and helpers that read
//! packets into it.

use crate::connection::MapleTestConnection;
use crate::hunting::Field;
use crate::packets::{
    build_all_chat, build_change_map, decode_char_look, decode_item_stack, decode_set_field_warp,
    decode_spawn_player, SpawnPlayerPacket,
};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::collections::BTreeMap;
use std::io::Cursor;
use tokio::time::{timeout, Duration, Instant};

/// STAT_CHANGED mask bits the tests read.
pub mod stat {
    pub const HAIR: i32 = 0x4;
    pub const LEVEL: i32 = 0x10;
    pub const STR: i32 = 0x40;
    pub const HP: i32 = 0x400;
    pub const MAX_HP: i32 = 0x800;
    pub const MP: i32 = 0x1000;
    pub const AP: i32 = 0x4000;
    pub const SP: i32 = 0x8000;
    pub const EXP: i32 = 0x10000;
    pub const FAME: i32 = 0x20000;
    pub const MESO: i32 = 0x40000;
    /// The summoned pets' serials, which come right after HAIR (0x8).
    pub const PET_2: i32 = 0x80000;
    pub const PET_3: i32 = 0x100000;
    pub const GACHA_EXP: i32 = 0x200000;
}

/// Wire widths of the STAT_CHANGED values, in mask order.
/// GIVE_BUFF's flags with only Speed Infusion's bit 86.
const INFUSION: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0];

const STAT_WIDTHS: [(i32, u8); 20] = [
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
    (0x200000, 4),
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
    /// Other UPDATE_QUEST_INFO results: mode, and the quest for those
    /// that name one (else 0).
    pub quest_refusals: Vec<(u8, i16)>,
    /// Latest progress string (kill counts) per started quest.
    pub kills: BTreeMap<i16, String>,
    /// EXP gains from SHOW_STATUS_INFO mode 3.
    pub gains: Vec<i32>,
    /// Fame gains (or losses) from SHOW_STATUS_INFO mode 4.
    pub fame_gains: Vec<i32>,
    /// Mesos gained without a pickup, shown in chat (SHOW_STATUS_INFO 5).
    pub meso_gains: Vec<i32>,
    /// Every STAT_CHANGED, as values by mask bit.
    pub stat_updates: Vec<BTreeMap<i32, i64>>,
    /// HP_TAKEN (0xD4): HP about to go without a hit, in order.
    pub hp_taken: Vec<i32>,
    /// UPDATE_SKILLS: (skill id, level).
    pub skills: Vec<(i32, i32)>,
    /// Item gains and losses shown in chat (SHOW_ITEM_GAIN_INCHAT 3).
    pub items: Vec<(i32, i32)>,
    /// Items added to the bags: (inventory type, slot, item id).
    pub added: Vec<(u8, i16, i32)>,
    /// Stacks whose count changed: (inventory type, slot, quantity).
    pub quantities: Vec<(u8, i16, i16)>,
    /// Slots emptied: (inventory type, slot).
    pub removed: Vec<(u8, i16)>,
    /// Items said to have expired (SHOW_STATUS_INFO 8).
    pub expired: Vec<i32>,
    /// Items said to be sealed no more (SHOW_STATUS_INFO 11).
    pub seals_expired: Vec<i32>,
    /// Cash mists puffed on this player (SHOW_ITEM_GAIN_INCHAT 20).
    pub mist_puffs: Vec<i32>,
    /// Cash mists whose face this player pulls (ITEM_EMOTION).
    pub item_emotions: Vec<i32>,
    /// INVENTORY_GROW: (inventory type, new slot count).
    pub bag_slots: Vec<(u8, u8)>,
    /// "Quest Clear" effects (SHOW_ITEM_GAIN_INCHAT 9).
    pub cleared: usize,
    /// "Card get" effects (SHOW_ITEM_GAIN_INCHAT 0x0D).
    pub card_effects: usize,
    /// Portal sounds of scripted portals (SHOW_ITEM_GAIN_INCHAT 7).
    pub portal_sounds: usize,
    /// MONSTER_BOOK_SET_CARD: (went into the book, card, copies).
    pub cards: Vec<(bool, i32, i32)>,
    /// MONSTER_BOOK_SET_COVER answers.
    pub covers: Vec<i32>,
    /// UI images shown on the player (SHOW_ITEM_GAIN_INCHAT 0x17).
    pub images: Vec<String>,
    /// "You can't get anymore items." (SHOW_STATUS_INFO 0/-1).
    pub inventory_full: usize,
    /// "This item is unavailable for the pick-up." (SHOW_STATUS_INFO 0, -2).
    pub unavailable: usize,
    /// NPC dialogue prompts: (npc, message type).
    pub prompts: Vec<(i32, u8)>,
    /// Their text, in the same order.
    pub prompt_texts: Vec<String>,
    /// Maps warped to.
    pub warps: Vec<i32>,
    /// Each map change: (map, spawn point, where the player appears
    /// instead).
    pub arrivals: Vec<(i32, u8, Option<(i32, i32)>)>,
    /// SPAWN_PORTAL: the player's own Mystic Door (town, field, x, y), or
    /// None.
    pub own_doors: Vec<Option<(i32, i32, i16, i16)>>,
    /// Party Mystic Doors (op 0x25): (slot, town, field, x, y).
    pub party_doors: Vec<(u8, i32, i32, i16, i16)>,
    /// PARTY_OPERATION sub-operations, in order.
    pub party_ops: Vec<u8>,
    /// Party invitations: (party id, inviter).
    pub party_invites: Vec<(i32, String)>,
    /// Each invitation's last byte: nonzero for a party search's.
    pub party_invite_searches: Vec<u8>,
    /// UPDATE_PARTYMEMBER_HP: (character, HP, max HP).
    pub party_hp: Vec<(i32, i32, i32)>,
    /// Party chat lines: (sender, text).
    pub party_chat: Vec<(String, String)>,
    /// Every full BUDDYLIST: (id, name, channel) per buddy.
    pub buddy_lists: Vec<Vec<(i32, String, i32)>>,
    /// The same lists' groups: (id, group) per buddy.
    pub buddy_groups: Vec<Vec<(i32, String)>>,
    /// The unanswered requests each full BUDDYLIST carries, by requester.
    pub buddy_list_requests: Vec<Vec<i32>>,
    /// Buddy requests: (requester id, name).
    pub buddy_requests: Vec<(i32, String)>,
    /// Buddy channel changes: (buddy, channel or -1).
    pub buddy_channels: Vec<(i32, i32)>,
    /// Failed buddy adds, by BUDDYLIST mode.
    pub buddy_errors: Vec<u8>,
    /// Buddy chat lines: (sender, text).
    pub buddy_chat: Vec<(String, String)>,
    /// Guild chat lines: (sender, text).
    pub guild_chat: Vec<(String, String)>,
    /// Every GUILD_OPERATION body (mode first), in order.
    pub guild_ops: Vec<Vec<u8>>,
    /// Alliance chat lines: (sender, text).
    pub alliance_chat: Vec<(String, String)>,
    /// Every ALLIANCE_OPERATION body (mode first), in order.
    pub alliance_ops: Vec<Vec<u8>>,
    /// Each GUILD_BBS packet's body, from its mode on.
    pub bbs: Vec<Vec<u8>>,
    /// GUILD_NAME_CHANGED: (character, guild name).
    pub guild_names: Vec<(i32, String)>,
    /// GUILD_MARK_CHANGED: (character, [background, its colour, mark, its
    /// colour]).
    pub guild_marks: Vec<(i32, [u16; 4])>,
    /// NOTIFY_LEVELUP and NOTIFY_JOB_CHANGE: (opcode, type, level or job,
    /// name).
    pub notices: Vec<(i16, u8, i32, String)>,
    /// Other players appearing on the field.
    pub players: Vec<SpawnPlayerPacket>,
    /// REMOVE_PLAYER_FROM_MAP: the players gone from the field.
    pub gone: Vec<i32>,
    /// ADMIN_RESULT 0x10: whether the GM is now hidden.
    pub hidden: Vec<bool>,
    /// NPC shops opened: the NPC and (item id, price) per row.
    pub shops: Vec<(i32, Vec<(i32, i32)>)>,
    /// CONFIRM_SHOP_TRANSACTION codes.
    pub shop_results: Vec<u8>,
    /// Every TRUNK_RESULT, in order.
    pub storage: Vec<StorageResult>,
    /// Every PARCEL body (mode first), in order.
    pub parcels: Vec<Vec<u8>>,
    /// Every RPS_GAME body (mode first), in order.
    pub rps: Vec<Vec<u8>>,
    /// Every OWL_RESULT and OWL_GO_RESULT: (opcode, body), in order.
    pub owl: Vec<(i16, Vec<u8>)>,
    /// Every family packet (FAMILY_CHART_RESULT to
    /// FAMILY_NOTIFY_LOGIN_OR_LOGOUT): (opcode, body), in order.
    pub family: Vec<(i16, Vec<u8>)>,
    /// Every MARRIAGE_REQUEST and MARRIAGE_RESULT: (opcode, body), in order.
    pub marriage: Vec<(i16, Vec<u8>)>,
    /// UPDATE_CHAR_LOOK: (character, ring blocks), in order.
    pub looks: Vec<(i32, crate::packets::RingLooks)>,
    /// NOTIFY_MARRIED_PARTNER_MAP_TRANSFER: (map, partner), in order.
    pub partner_maps: Vec<(i32, i32)>,
    /// SHOW_STATUS_INFO mode 9: the info lines, in order.
    pub info_texts: Vec<String>,
    /// SCRIPT_PROGRESS_MESSAGE lines, in order.
    pub script_messages: Vec<String>,
    /// QUEST_CLEAR: the quests whose titles were earned, in order.
    pub quest_clears: Vec<i16>,
    /// Every PLAYER_INTERACTION body (mode first), in order.
    pub rooms: Vec<Vec<u8>>,
    /// Every UPDATE_CHAR_BOX body (character id first), in order.
    pub char_boxes: Vec<Vec<u8>>,
    /// GIVE_BUFF: (skill id, value, duration in ms).
    pub buffs: Vec<(i32, i16, i32)>,
    /// CANCEL_BUFF: the second mask of each.
    pub cancelled_buffs: Vec<i64>,
    /// COOLDOWN: (skill id, seconds).
    pub cooldowns: Vec<(i32, i16)>,
    /// BLOCKED_MAP and BLOCKED_SERVER: (opcode, reason).
    pub blocked: Vec<(i16, u8)>,
    /// FIELD_EFFECT modes naming something: (mode, name), for 2 a map
    /// object, 3 a screen effect and 4 a sound.
    pub field_effects: Vec<(u8, String)>,
    /// SKILL_LEARN_ITEM_RESULT: (reader, skill, master level, used, success).
    pub books: Vec<(i32, i32, i32, bool, bool)>,
    /// FAME_RESPONSE: (mode, name, up, new fame); the name, up and fame are
    /// empty, false and 0 where the mode has none.
    pub fame: Vec<(u8, String, bool, i32)>,
    /// BROADCAST_MSG: (kind, message) of each whose kind carries only its
    /// message (0, 1, 5).
    pub broadcasts: Vec<(u8, String)>,
    /// BROADCAST_MSG kind 6: (message, linked item or 0).
    pub item_lines: Vec<(String, i32)>,
    /// BLOW_WEATHER: (item, message) of each weather that starts blowing.
    pub weathers: Vec<(i32, String)>,
    /// PLAYER_HINT: (text, width, height) of each instruction balloon.
    pub hints: Vec<(String, i16, i16)>,
    /// CLOCK kind 1, a wall clock's time of day: (hour, minute, second).
    pub clocks: Vec<(u8, u8, u8)>,
    /// CLOCK kind 2, the field's countdown: its seconds.
    pub countdowns: Vec<i32>,
    /// STOP_CLOCK: the countdown taken down.
    pub clocks_stopped: usize,
    /// CONTI_MOVE: (target, state) of each ship shown leaving or docking.
    pub ship_moves: Vec<(u8, u8)>,
    /// CONTI_STATE: the ship's state, as answered...
    pub ship_states: Vec<u8>,
    /// ...and whether the Crimson Balrog's ship is out.
    pub balrog_out: Vec<bool>,
    /// SPAWN_PET: (owner, index, the pet's item id, name and serial), or
    /// for a pet going back, (owner, index, why) in `pets_gone`.
    pub pets: Vec<(i32, u8, i32, String, i64)>,
    pub pets_gone: Vec<(i32, u8, u8)>,
    /// MOVE_PET: (owner, index, start x, start y).
    pub pet_moves: Vec<(i32, u8, i16, i16)>,
    /// Mobs and drops on the current field.
    pub field: Field,
}

/// One TRUNK_RESULT: its mode, and what of the storage it carries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageResult {
    pub mode: u8,
    /// The storage keeper, when it opens.
    pub npc: Option<i32>,
    pub slots: Option<u8>,
    pub meso: Option<i32>,
    /// (item id, quantity) per bag carried, by inventory type.
    pub bags: BTreeMap<u8, Vec<(i32, i16)>>,
}

impl StorageResult {
    fn read(mode: u8, body: &[u8]) -> Self {
        let mut r = Cursor::new(body);
        let mut result = Self {
            mode,
            ..Self::default()
        };
        // Only these modes carry contents; the rest are a bare notice.
        if !matches!(mode, 0x09 | 0x0d | 0x0f | 0x13 | 0x16) {
            return result;
        }
        if mode == 0x16 {
            result.npc = Some(r.read_int().unwrap());
        }
        result.slots = Some(r.read_byte().unwrap());
        let flags = r.read_long().unwrap();
        if flags & 2 != 0 {
            result.meso = Some(r.read_int().unwrap());
        }
        for kind in 1..=5u8 {
            if flags & 2 << kind != 0 {
                let items = (0..r.read_byte().unwrap())
                    .map(|_| decode_item_stack(&mut r).unwrap())
                    .collect();
                result.bags.insert(kind, items);
            }
        }
        result
    }
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
                0 => match r.read_byte().unwrap() {
                    0xFF => self.inventory_full += 1,
                    0xFE => self.unavailable += 1,
                    _ => {}
                },
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
                4 => self.fame_gains.push(r.read_int().unwrap()),
                5 => self.meso_gains.push(r.read_int().unwrap()),
                9 => self.info_texts.push(r.read_str_with_length().unwrap()),
                8 => {
                    for _ in 0..r.read_byte().unwrap() {
                        self.expired.push(r.read_int().unwrap());
                    }
                }
                11 => {
                    for _ in 0..r.read_byte().unwrap() {
                        self.seals_expired.push(r.read_int().unwrap());
                    }
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
                7 => self.portal_sounds += 1,
                9 => self.cleared += 1,
                0x0D => self.card_effects += 1,
                0x14 => self.mist_puffs.push(r.read_int().unwrap()),
                0x17 => self.images.push(r.read_str_with_length().unwrap()),
                _ => {}
            }
        } else if op == SendOpcode::ItemEmotion as i16 {
            self.item_emotions.push(r.read_int().unwrap());
        } else if op == SendOpcode::MonsterBookSetCard as i16 {
            let added = r.read_byte().unwrap() == 1;
            self.cards
                .push((added, r.read_int().unwrap(), r.read_int().unwrap()));
        } else if op == SendOpcode::MonsterBookSetCover as i16 {
            self.covers.push(r.read_int().unwrap());
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
                if bit == 0x8 {
                    for pet in [stat::PET_2, stat::PET_3]
                        .into_iter()
                        .filter(|b| mask & b != 0)
                    {
                        values.insert(pet, r.read_long().unwrap());
                    }
                }
            }
            self.stat_updates.push(values);
        } else if op == SendOpcode::HpTaken as i16 {
            self.hp_taken.push(r.read_int().unwrap());
        } else if op == SendOpcode::BlockedMap as i16 || op == SendOpcode::BlockedServer as i16 {
            self.blocked.push((op, r.read_byte().unwrap()));
        } else if op == SendOpcode::FieldEffect as i16 {
            let mode = r.read_byte().unwrap();
            if matches!(mode, 2..=4) {
                self.field_effects
                    .push((mode, r.read_str_with_length().unwrap()));
            }
        } else if op == SendOpcode::BroadcastMsg as i16 {
            let kind = r.read_byte().unwrap();
            if matches!(kind, 0 | 1 | 5) {
                self.broadcasts
                    .push((kind, r.read_str_with_length().unwrap()));
            } else if kind == 6 {
                let message = r.read_str_with_length().unwrap();
                self.item_lines.push((message, r.read_int().unwrap()));
            }
        } else if op == SendOpcode::BlowWeather as i16 {
            r.read_byte().unwrap();
            let item_id = r.read_int().unwrap();
            if item_id != 0 {
                self.weathers
                    .push((item_id, r.read_str_with_length().unwrap()));
            }
        } else if op == SendOpcode::PlayerHint as i16 {
            let text = r.read_str_with_length().unwrap();
            self.hints
                .push((text, r.read_short().unwrap(), r.read_short().unwrap()));
        } else if op == SendOpcode::FameResponse as i16 {
            let mode = r.read_byte().unwrap();
            let (name, up, fame) = match mode {
                0 => (
                    r.read_str_with_length().unwrap(),
                    r.read_byte().unwrap() != 0,
                    r.read_int().unwrap(),
                ),
                5 => (
                    r.read_str_with_length().unwrap(),
                    r.read_byte().unwrap() != 0,
                    0,
                ),
                _ => (String::new(), false, 0),
            };
            self.fame.push((mode, name, up, fame));
        } else if op == SendOpcode::SkillBookResult as i16 {
            let reader = r.read_int().unwrap();
            r.read_byte().unwrap();
            let skill_id = r.read_int().unwrap();
            let master_level = r.read_int().unwrap();
            let used = r.read_byte().unwrap() != 0;
            self.books.push((
                reader,
                skill_id,
                master_level,
                used,
                r.read_byte().unwrap() != 0,
            ));
        } else if op == SendOpcode::UpdateSkills as i16 {
            r.read_byte().unwrap();
            for _ in 0..r.read_short().unwrap() {
                let skill_id = r.read_int().unwrap();
                self.skills.push((skill_id, r.read_int().unwrap()));
                r.read_int().unwrap();
                r.read_long().unwrap();
            }
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
                    1 => {
                        self.quantities.push((kind, slot, r.read_short().unwrap()));
                    }
                    2 => {
                        r.read_short().unwrap();
                    }
                    3 => self.removed.push((kind, slot)),
                    _ => {}
                }
            }
        } else if op == SendOpcode::InventoryGrow as i16 {
            self.bag_slots
                .push((r.read_byte().unwrap(), r.read_byte().unwrap()));
        } else if op == SendOpcode::UpdateQuestInfo as i16 {
            match r.read_byte().unwrap() {
                8 => self.results.push((
                    r.read_short().unwrap(),
                    r.read_int().unwrap(),
                    r.read_short().unwrap(),
                )),
                mode => self
                    .quest_refusals
                    .push((mode, r.read_short().unwrap_or(0))),
            }
        } else if op == SendOpcode::NpcDialogue as i16 {
            r.read_byte().unwrap();
            let npc = r.read_int().unwrap();
            self.prompts.push((npc, r.read_byte().unwrap()));
            r.read_byte().unwrap();
            self.prompt_texts.push(r.read_str_with_length().unwrap());
        } else if op == SendOpcode::PartyOperation as i16 {
            let party_op = r.read_byte().unwrap();
            self.party_ops.push(party_op);
            if party_op == 4 {
                let party_id = r.read_int().unwrap();
                self.party_invites
                    .push((party_id, r.read_str_with_length().unwrap()));
                self.party_invite_searches.push(r.read_byte().unwrap_or(0));
            } else if party_op == 0x25 {
                self.party_doors.push((
                    r.read_byte().unwrap(),
                    r.read_int().unwrap(),
                    r.read_int().unwrap(),
                    r.read_short().unwrap(),
                    r.read_short().unwrap(),
                ));
            }
        } else if op == SendOpcode::UpdatePartyMemberHp as i16 {
            self.party_hp.push((
                r.read_int().unwrap(),
                r.read_int().unwrap(),
                r.read_int().unwrap(),
            ));
        } else if op == SendOpcode::MultiChat as i16 {
            let mode = r.read_byte().unwrap();
            let line = (
                r.read_str_with_length().unwrap(),
                r.read_str_with_length().unwrap(),
            );
            match mode {
                0 => self.buddy_chat.push(line),
                1 => self.party_chat.push(line),
                2 => self.guild_chat.push(line),
                3 => self.alliance_chat.push(line),
                _ => {}
            }
        } else if op == SendOpcode::BuddyList as i16 {
            match r.read_byte().unwrap() {
                7 => {
                    let count = r.read_byte().unwrap();
                    let (mut list, mut requests, mut groups) = (Vec::new(), Vec::new(), Vec::new());
                    for _ in 0..count {
                        let id = r.read_int().unwrap();
                        let name = read_padded(&mut r, 13);
                        let requesting = r.read_byte().unwrap() == 1;
                        let channel = r.read_int().unwrap();
                        let group = read_padded(&mut r, 17);
                        if requesting {
                            requests.push(id);
                        } else {
                            list.push((id, name, channel));
                            groups.push((id, group));
                        }
                    }
                    self.buddy_lists.push(list);
                    self.buddy_groups.push(groups);
                    self.buddy_list_requests.push(requests);
                }
                9 => {
                    let id = r.read_int().unwrap();
                    self.buddy_requests
                        .push((id, r.read_str_with_length().unwrap()));
                }
                0x14 => {
                    let id = r.read_int().unwrap();
                    r.read_byte().unwrap();
                    self.buddy_channels.push((id, r.read_int().unwrap()));
                }
                mode @ 0x0B..=0x0F => self.buddy_errors.push(mode),
                _ => {}
            }
        } else if op == SendOpcode::OpenNpcShop as i16 {
            let npc = r.read_int().unwrap();
            let rows = (0..r.read_short().unwrap())
                .map(|_| {
                    let (item, price) = (r.read_int().unwrap(), r.read_int().unwrap());
                    for _ in 0..3 {
                        r.read_int().unwrap();
                    }
                    if matches!(item / 10_000, 207 | 233) {
                        r.read_long().unwrap(); // recharge rate
                    } else {
                        r.read_short().unwrap(); // bundle size
                    }
                    r.read_short().unwrap();
                    (item, price)
                })
                .collect();
            self.shops.push((npc, rows));
        } else if op == SendOpcode::TrunkResult as i16 {
            let mode = r.read_byte().unwrap();
            self.storage.push(StorageResult::read(mode, &p.bytes[3..]));
        } else if op == SendOpcode::Parcel as i16 {
            self.parcels.push(p.bytes[2..].to_vec());
        } else if op == SendOpcode::RpsGame as i16 {
            self.rps.push(p.bytes[2..].to_vec());
        } else if op == SendOpcode::OwlResult as i16 || op == SendOpcode::OwlGoResult as i16 {
            self.owl.push((op, p.bytes[2..].to_vec()));
        } else if (SendOpcode::FamilyChart as i16..=SendOpcode::FamilySummonRequest as i16)
            .contains(&op)
        {
            self.family.push((op, p.bytes[2..].to_vec()));
        } else if op == SendOpcode::MarriageRequest as i16
            || op == SendOpcode::MarriageResult as i16
        {
            self.marriage.push((op, p.bytes[2..].to_vec()));
        } else if op == SendOpcode::UpdateCharLook as i16 {
            self.looks
                .push(decode_char_look(p).expect("UPDATE_CHAR_LOOK"));
        } else if op == SendOpcode::NotifyMarriedPartnerMapTransfer as i16 {
            self.partner_maps
                .push((r.read_int().unwrap(), r.read_int().unwrap()));
        } else if op == SendOpcode::ScriptProgressMessage as i16 {
            self.script_messages.push(r.read_str_with_length().unwrap());
        } else if op == SendOpcode::QuestClear as i16 {
            self.quest_clears.push(r.read_short().unwrap());
        } else if op == SendOpcode::PlayerInteraction as i16 {
            self.rooms.push(p.bytes[2..].to_vec());
        } else if op == SendOpcode::UpdateCharBox as i16 {
            self.char_boxes.push(p.bytes[2..].to_vec());
        } else if op == SendOpcode::GuildOperation as i16 {
            self.guild_ops.push(p.bytes[2..].to_vec());
        } else if op == SendOpcode::AllianceOperation as i16 {
            self.alliance_ops.push(p.bytes[2..].to_vec());
        } else if op == SendOpcode::GuildBbs as i16 {
            self.bbs.push(p.bytes[2..].to_vec());
        } else if op == SendOpcode::NotifyLevelUp as i16 || op == SendOpcode::NotifyJobChange as i16
        {
            let (kind, value) = (r.read_byte().unwrap(), r.read_int().unwrap());
            self.notices
                .push((op, kind, value, r.read_str_with_length().unwrap()));
        } else if op == SendOpcode::GuildNameChanged as i16 {
            self.guild_names
                .push((r.read_int().unwrap(), r.read_str_with_length().unwrap()));
        } else if op == SendOpcode::GuildMarkChanged as i16 {
            let cid = r.read_int().unwrap();
            let emblem = [
                r.read_short().unwrap() as u16,
                u16::from(r.read_byte().unwrap()),
                r.read_short().unwrap() as u16,
                u16::from(r.read_byte().unwrap()),
            ];
            self.guild_marks.push((cid, emblem));
        } else if op == SendOpcode::SpawnPet as i16 {
            let (owner, index) = (r.read_int().unwrap(), r.read_byte().unwrap());
            if r.read_byte().unwrap() == 0 {
                self.pets_gone.push((owner, index, r.read_byte().unwrap()));
            } else {
                r.read_byte().unwrap();
                let item_id = r.read_int().unwrap();
                let name = r.read_str_with_length().unwrap();
                self.pets
                    .push((owner, index, item_id, name, r.read_long().unwrap()));
            }
        } else if op == SendOpcode::MovePet as i16 {
            let (owner, index) = (r.read_int().unwrap(), r.read_byte().unwrap());
            self.pet_moves.push((
                owner,
                index,
                r.read_short().unwrap(),
                r.read_short().unwrap(),
            ));
        } else if op == SendOpcode::SpawnPlayer as i16 {
            self.players.extend(decode_spawn_player(p));
        } else if op == SendOpcode::RemovePlayerFromMap as i16 {
            self.gone.push(r.read_int().unwrap());
        } else if op == SendOpcode::AdminResult as i16 && r.read_byte().unwrap() == 0x10 {
            self.hidden.push(r.read_byte().unwrap() != 0);
        } else if op == SendOpcode::ConfirmShopTransaction as i16 {
            self.shop_results.push(r.read_byte().unwrap());
        } else if op == SendOpcode::GiveBuff as i16 && p.bytes.get(2..18) == Some(&INFUSION) {
            // Speed Infusion's two-state data follows the defense bytes.
            r.read_bytes(16 + 2).unwrap();
            let value = r.read_int().unwrap() as i16;
            let skill = r.read_int().unwrap();
            r.read_bytes(10).unwrap();
            let seconds = i32::from(r.read_short().unwrap());
            self.buffs.push((skill, value, seconds * 1000));
        } else if op == SendOpcode::GiveBuff as i16 {
            r.read_long().unwrap();
            r.read_long().unwrap();
            let value = r.read_short().unwrap();
            self.buffs
                .push((r.read_int().unwrap(), value, r.read_int().unwrap()));
        } else if op == SendOpcode::CancelBuff as i16 {
            r.read_long().unwrap();
            self.cancelled_buffs.push(r.read_long().unwrap());
        } else if op == SendOpcode::Cooldown as i16 {
            self.cooldowns
                .push((r.read_int().unwrap(), r.read_short().unwrap()));
        } else if op == SendOpcode::SetField as i16 && p.bytes.get(6) == Some(&0) {
            // A map change rather than entering the world.
            let warp = decode_set_field_warp(p).unwrap();
            self.warps.push(warp.map_id);
            self.arrivals
                .push((warp.map_id, warp.spawn_point, warp.chase));
        } else if op == SendOpcode::Clock as i16 {
            match r.read_byte().unwrap() {
                1 => self.clocks.push((
                    r.read_byte().unwrap(),
                    r.read_byte().unwrap(),
                    r.read_byte().unwrap(),
                )),
                2 => self.countdowns.push(r.read_int().unwrap()),
                _ => {}
            }
        } else if op == SendOpcode::StopClock as i16 {
            self.clocks_stopped += 1;
        } else if op == SendOpcode::ContiMove as i16 {
            self.ship_moves
                .push((r.read_byte().unwrap(), r.read_byte().unwrap()));
        } else if op == SendOpcode::ContiState as i16 {
            self.ship_states.push(r.read_byte().unwrap());
            self.balrog_out.push(r.read_byte().unwrap() == 1);
        } else if op == SendOpcode::SpawnPortal as i16 {
            let (town, field) = (r.read_int().unwrap(), r.read_int().unwrap());
            self.own_doors.push((town != 999_999_999).then(|| {
                (
                    town,
                    field,
                    r.read_short().unwrap(),
                    r.read_short().unwrap(),
                )
            }));
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

/// A NUL-padded fixed-length string.
fn read_padded(r: &mut Cursor<&Vec<u8>>, len: usize) -> String {
    let bytes: Vec<u8> = (0..len).map(|_| r.read_byte().unwrap()).collect();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(len);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
