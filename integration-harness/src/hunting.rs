//! Hunting as HeavenClient does it: walking, melee attacks, picking up
//! drops, and using items, plus a record of what a test has seen on its
//! field.

use crate::packets::build_player_move;
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

/// CLOSE_RANGE_ATTACK hitting one mob once for `damage`.
pub fn attack(object_id: i32, damage: i32) -> Packet {
    skill_attack(0, object_id, damage)
}

/// CLOSE_RANGE_ATTACK with a skill (0 for none), hitting one mob once.
pub fn skill_attack(skill_id: i32, object_id: i32, damage: i32) -> Packet {
    kind_attack(RecvOpcode::CloseRangeAttack, skill_id, 0, object_id, damage)
}

/// RANGED_ATTACK with a skill (0 for none) firing the ammo in USE slot
/// `ammo_slot`, hitting one mob once.
pub fn ranged_attack(skill_id: i32, ammo_slot: i16, object_id: i32, damage: i32) -> Packet {
    kind_attack(
        RecvOpcode::RangedAttack,
        skill_id,
        ammo_slot,
        object_id,
        damage,
    )
}

/// MAGIC_ATTACK with a skill, hitting one mob once.
pub fn magic_attack(skill_id: i32, object_id: i32, damage: i32) -> Packet {
    kind_attack(RecvOpcode::MagicAttack, skill_id, 0, object_id, damage)
}

/// An attack hitting one mob once: a shot has 5 more bytes before its
/// target, its ammo slot among them.
fn kind_attack(
    op: RecvOpcode,
    skill_id: i32,
    ammo_slot: i16,
    object_id: i32,
    damage: i32,
) -> Packet {
    let ranged = matches!(op, RecvOpcode::RangedAttack);
    let mut p = Packet::new_empty();
    p.write_short(op as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0x11).unwrap();
    p.write_int(skill_id).unwrap();
    p.write_bytes(&[0; 8]).unwrap();
    // Flags, then action 5 facing left (v83 `action | left << 15`), then
    // a byte and the attack speed.
    p.write_byte(0).unwrap();
    p.write_short(0x8005u16 as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(4).unwrap();
    if ranged {
        p.write_int(0).unwrap();
        p.write_short(ammo_slot).unwrap();
        p.write_short(0).unwrap();
        p.write_byte(0).unwrap();
    } else {
        p.write_bytes(&[0; 4]).unwrap();
    }
    p.write_int(object_id).unwrap();
    p.write_bytes(&[0; 14]).unwrap();
    p.write_int(damage).unwrap();
    p.write_bytes(&[0; 4]).unwrap();
    p
}

/// DAMAGE_REACTOR: a plain attack on a reactor, standing at (x, y).
pub fn hit_reactor(object_id: i32, x: i16, y: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::DamageReactor as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_short(0).unwrap();
    p.write_int(0).unwrap();
    p.write_int(0).unwrap();
    p
}

/// ITEM_PICKUP of a drop, standing at (x, y).
pub fn pickup(object_id: i32, x: i16, y: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemPickup as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_int(object_id).unwrap();
    p
}

/// MONSTER_BOOK_COVER: show a card (0 for none) on the book's cover.
pub fn choose_cover(card: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MonsterBookCover as i16).unwrap();
    p.write_int(card).unwrap();
    p
}

/// USE_ITEM of the item in a USE slot.
pub fn use_item(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseItem as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// USE_SKILL_BOOK of the skill or mastery book in a USE slot.
pub fn use_skill_book(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSkillBook as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// SPEND_AP on the stat with this STAT_CHANGED mask bit.
pub fn spend_ap(stat: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpendAp as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(stat).unwrap();
    p
}

/// SPEND_SP on a skill.
pub fn spend_sp(skill_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpendSp as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(skill_id).unwrap();
    p
}

/// USE_SKILL of a skill at the level the client thinks it has.
pub fn use_skill(skill_id: i32, level: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSkill as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(skill_id).unwrap();
    p.write_byte(level).unwrap();
    p
}

/// USE_SKILL of a summon skill, putting it at (x, y) facing left or not.
pub fn summon(skill_id: i32, level: u8, x: i16, y: i16, left: bool) -> Packet {
    let mut p = use_skill(skill_id, level);
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_byte(u8::from(left)).unwrap();
    p
}

/// SUMMON_ATTACK: summon `object_id` hitting mob `mob` once, as HeavenClient
/// sends it (0x7a57d4).
pub fn summon_attack(object_id: i32, mob: i32, mob_id: i32, damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SummonAttack as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0x84).unwrap();
    p.write_byte(1).unwrap();
    p.write_bytes(&[0; 8]).unwrap();
    p.write_int(mob).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_bytes(&[6, 0x80, 0, 0]).unwrap();
    p.write_bytes(&[0; 8]).unwrap();
    p.write_short(100).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(0).unwrap();
    p
}

/// DAMAGE_SUMMON: a mob's touch (attack -1) hurting summon `object_id`.
pub fn damage_summon(object_id: i32, damage: i32, mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::DamageSummon as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_byte(0xFF).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// BEHOLDER: Beholder `object_id` healing its owner (Aura of the
/// Beholder), or buffing stat `buff` (Hex of the Beholder), as the client
/// sends it (FUN_007a5ab4 / FUN_007a5c99).
pub fn beholder(object_id: i32, skill_id: i32, buff: Option<u8>) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::Beholder as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_int(skill_id).unwrap();
    p.write_byte(buff.map_or(6, |buff| 7 + buff)).unwrap();
    if let Some(buff) = buff {
        p.write_byte(buff).unwrap();
    }
    p
}

/// CANCEL_BUFF: right-clicking a skill's buff icon.
pub fn cancel_buff(skill_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CancelBuff as i16).unwrap();
    p.write_int(skill_id).unwrap();
    p
}

/// CANCEL_ITEM_EFFECT: right-clicking an item buff's icon (its id is the
/// buff's, `-item_id`).
pub fn cancel_item_effect(buff_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CancelItemEffect as i16).unwrap();
    p.write_int(buff_id).unwrap();
    p
}

/// MOVE_PLAYER: one absolute movement to (x, y), standing.
pub fn walk_to(x: i16, y: i16) -> Packet {
    let mut movement = vec![1u8, 0];
    movement.extend(x.to_le_bytes());
    movement.extend(y.to_le_bytes());
    movement.extend([0; 6]);
    movement.push(4);
    movement.extend(100i16.to_le_bytes());
    build_player_move(&movement).unwrap()
}

/// MOVE_LIFE for `object_id` at (x, y): wanting to attack, or playing
/// `action` (its first skill animation is 21, doubled with the facing)
/// with `skill_data` (skill | level << 8 | delay << 16).
pub fn move_mob(
    object_id: i32,
    (x, y): (i16, i16),
    want_attack: bool,
    action: u8,
    skill_data: i32,
) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MoveLife as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_short(1).unwrap();
    p.write_byte(u8::from(want_attack)).unwrap();
    p.write_byte(action).unwrap();
    p.write_int(skill_data).unwrap();
    p.write_bytes(&[0; 13]).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_byte(1).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_bytes(&[0; 6]).unwrap();
    p.write_byte(4).unwrap();
    p.write_short(100).unwrap();
    p
}

/// An item or mesos dropped on the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapDrop {
    pub object_id: i32,
    pub meso: bool,
    /// The item id, or the amount of mesos.
    pub value: i32,
    pub x: i16,
    pub y: i16,
}

/// A reactor spawned on the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapReactor {
    pub object_id: i32,
    pub reactor_id: i32,
    pub state: u8,
    pub x: i16,
    pub y: i16,
}

/// Mobs, attacks, drops, reactors, and effects seen on the current field.
#[derive(Debug, Default)]
pub struct Field {
    /// (object id, mob id)
    pub mobs: Vec<(i32, i32)>,
    /// Mobs this client controls.
    pub controlled: BTreeSet<i32>,
    /// Mobs handed to this client with aggro, in order.
    pub aggro: Vec<i32>,
    /// Mob movements relayed from their controller.
    pub moved: Vec<i32>,
    /// Answers to this client's own mob movements.
    pub move_responses: Vec<i32>,
    /// Skills those answers let a mob cast: (object id, skill, level).
    pub grants: Vec<(i32, u8, u8)>,
    /// Mobs healed: (object id, HP regained).
    pub heals: Vec<(i32, i32)>,
    /// How each mob appeared: a summon effect, -2 fading in, or -1 just
    /// there.
    pub appearances: BTreeMap<i32, i8>,
    /// (object id, HP percentage)
    pub hp_bars: Vec<(i32, u8)>,
    pub killed: Vec<i32>,
    /// Characters seen attacking.
    pub attacks_by: Vec<i32>,
    /// The skill of each attack seen: (character id, skill id, level).
    pub attack_skills: Vec<(i32, i32, u8)>,
    /// The opcode of each attack seen.
    pub attack_ops: Vec<i16>,
    pub drops: Vec<MapDrop>,
    /// (object id, character who picked it up)
    pub removed: Vec<(i32, Option<i32>)>,
    /// Effects on other characters: (character id, effect).
    pub effects: Vec<(i32, u8)>,
    pub reactors: Vec<MapReactor>,
    /// (object id, state it moved to)
    pub reactor_hits: Vec<(i32, u8)>,
    /// (object id, state it broke from)
    pub reactors_removed: Vec<(i32, u8)>,
    /// (object id, NPC id)
    pub npcs: Vec<(i32, i32)>,
    /// Where each mob spawned.
    pub mob_positions: BTreeMap<i32, (i16, i16)>,
    /// Statuses put on mobs: (object id, stats, first one's value, first
    /// one's skill).
    pub stat_sets: Vec<(i32, u128, i16, i32)>,
    /// Statuses taken off mobs: (object id, stats).
    pub stat_resets: Vec<(i32, u128)>,
    pub mists: Vec<MapMist>,
    /// Object ids of the mists removed.
    pub mists_removed: Vec<i32>,
    pub summons: Vec<MapSummon>,
    /// (object id, why it went)
    pub summons_removed: Vec<(i32, u8)>,
    /// Summon paths relayed from their owners: object ids.
    pub summons_moved: Vec<i32>,
    /// (object id, action, (mob object id, damage) per hit)
    pub summon_attacks: Vec<(i32, u8, Vec<(i32, i32)>)>,
    /// SUMMON_SKILL: (object id, action).
    pub summon_skills: Vec<(i32, u8)>,
    /// SPAWN_DOOR: (owner, where, just cast).
    pub doors: Vec<(i32, (i16, i16), bool)>,
    /// REMOVE_DOOR: owners.
    pub doors_removed: Vec<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapSummon {
    pub owner_id: i32,
    pub object_id: i32,
    pub skill_id: i32,
    pub level: u8,
    pub x: i16,
    pub y: i16,
    pub moves: u8,
    pub assist: u8,
    /// Whether it plays its `summoned` animation.
    pub appears: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapMist {
    pub object_id: i32,
    pub kind: i32,
    pub owner_id: i32,
    pub skill_id: i32,
    pub level: u8,
    pub delay: i16,
    /// Left, top, right, bottom.
    pub area: (i32, i32, i32, i32),
}

/// A mob's 16 status flag bytes as bits by stat: the second long holds
/// stats 0 to 63, the first the rest, each with its dwords swapped.
fn read_mob_stats(r: &mut Cursor<&Vec<u8>>) -> u128 {
    let high = r.read_long().unwrap() as u64;
    let low = r.read_long().unwrap() as u64;
    ((high.rotate_left(32) as u128) << 64) | low.rotate_left(32) as u128
}

impl Field {
    pub fn observe(&mut self, p: &Packet) {
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        let op = p.opcode();
        if op == SendOpcode::SpawnMonster as i16 {
            let oid = r.read_int().unwrap();
            r.read_byte().unwrap();
            self.mobs.push((oid, r.read_int().unwrap()));
            // Each status the server sends is a value, a skill, and a
            // duration; the counters' spreads and their chance follow.
            let stats = read_mob_stats(&mut r);
            let counters = (stats >> 29 & 0b11).count_ones() as usize;
            let extras = 4 * (counters + usize::from(counters > 0));
            r.read_bytes(8 * stats.count_ones() as usize + extras)
                .unwrap();
            let position = (r.read_short().unwrap(), r.read_short().unwrap());
            self.mob_positions.insert(oid, position);
            r.read_byte().unwrap();
            r.read_short().unwrap();
            r.read_short().unwrap();
            self.appearances.insert(oid, r.read_byte().unwrap() as i8);
        } else if op == SendOpcode::MobStatSet as i16 {
            let oid = r.read_int().unwrap();
            let stats = read_mob_stats(&mut r);
            let value = r.read_short().unwrap();
            let skill_id = r.read_int().unwrap();
            self.stat_sets.push((oid, stats, value, skill_id));
        } else if op == SendOpcode::MobStatReset as i16 {
            let oid = r.read_int().unwrap();
            self.stat_resets.push((oid, read_mob_stats(&mut r)));
        } else if op == SendOpcode::SpawnMist as i16 {
            self.mists.push(MapMist {
                object_id: r.read_int().unwrap(),
                kind: r.read_int().unwrap(),
                owner_id: r.read_int().unwrap(),
                skill_id: r.read_int().unwrap(),
                level: r.read_byte().unwrap(),
                delay: r.read_short().unwrap(),
                area: (
                    r.read_int().unwrap(),
                    r.read_int().unwrap(),
                    r.read_int().unwrap(),
                    r.read_int().unwrap(),
                ),
            });
        } else if op == SendOpcode::RemoveMist as i16 {
            self.mists_removed.push(r.read_int().unwrap());
        } else if op == SendOpcode::SpawnDoor as i16 {
            let just_cast = r.read_byte().unwrap() == 0;
            let owner = r.read_int().unwrap();
            let at = (r.read_short().unwrap(), r.read_short().unwrap());
            self.doors.push((owner, at, just_cast));
        } else if op == SendOpcode::RemoveDoor as i16 {
            r.read_byte().unwrap();
            self.doors_removed.push(r.read_int().unwrap());
        } else if op == SendOpcode::SpawnSummon as i16 {
            let owner_id = r.read_int().unwrap();
            let object_id = r.read_int().unwrap();
            let skill_id = r.read_int().unwrap();
            r.read_byte().unwrap();
            let level = r.read_byte().unwrap();
            let (x, y) = (r.read_short().unwrap(), r.read_short().unwrap());
            r.read_byte().unwrap();
            r.read_short().unwrap();
            self.summons.push(MapSummon {
                owner_id,
                object_id,
                skill_id,
                level,
                x,
                y,
                moves: r.read_byte().unwrap(),
                assist: r.read_byte().unwrap(),
                appears: r.read_byte().unwrap() == 0,
            });
        } else if op == SendOpcode::RemoveSummon as i16 {
            r.read_int().unwrap();
            let object_id = r.read_int().unwrap();
            self.summons_removed
                .push((object_id, r.read_byte().unwrap()));
        } else if op == SendOpcode::MoveSummon as i16 {
            r.read_int().unwrap();
            self.summons_moved.push(r.read_int().unwrap());
        } else if op == SendOpcode::SummonAttack as i16 {
            r.read_int().unwrap();
            let object_id = r.read_int().unwrap();
            r.read_byte().unwrap();
            let action = r.read_byte().unwrap();
            let count = r.read_byte().unwrap();
            let hits = (0..count)
                .map(|_| {
                    let mob = r.read_int().unwrap();
                    r.read_byte().unwrap();
                    (mob, r.read_int().unwrap())
                })
                .collect();
            self.summon_attacks.push((object_id, action, hits));
        } else if op == SendOpcode::SummonSkill as i16 {
            r.read_int().unwrap();
            let object_id = r.read_int().unwrap();
            self.summon_skills.push((object_id, r.read_byte().unwrap()));
        } else if op == SendOpcode::SpawnMonsterControl as i16 {
            let mode = r.read_byte().unwrap();
            let oid = r.read_int().unwrap();
            if mode == 0 {
                self.controlled.remove(&oid);
            } else {
                self.controlled.insert(oid);
            }
            if mode == 2 {
                self.aggro.push(oid);
            }
        } else if op == SendOpcode::MoveMonster as i16 {
            self.moved.push(r.read_int().unwrap());
        } else if op == SendOpcode::MoveMonsterResponse as i16 {
            let oid = r.read_int().unwrap();
            self.move_responses.push(oid);
            r.read_short().unwrap();
            r.read_byte().unwrap();
            r.read_short().unwrap();
            let (skill, level) = (r.read_byte().unwrap(), r.read_byte().unwrap());
            if skill != 0 {
                self.grants.push((oid, skill, level));
            }
        } else if op == SendOpcode::DamageMonster as i16 {
            let oid = r.read_int().unwrap();
            r.read_byte().unwrap();
            self.heals.push((oid, -r.read_int().unwrap()));
        } else if op == SendOpcode::ShowMonsterHp as i16 {
            self.hp_bars
                .push((r.read_int().unwrap(), r.read_byte().unwrap()));
        } else if op == SendOpcode::KillMonster as i16 {
            self.killed.push(r.read_int().unwrap());
        } else if op == SendOpcode::CloseRangeAttack as i16
            || op == SendOpcode::RangedAttack as i16
            || op == SendOpcode::MagicAttack as i16
        {
            self.attack_ops.push(op);
            let character_id = r.read_int().unwrap();
            self.attacks_by.push(character_id);
            r.read_bytes(2).unwrap();
            let level = r.read_byte().unwrap();
            let skill_id = if level > 0 { r.read_int().unwrap() } else { 0 };
            self.attack_skills.push((character_id, skill_id, level));
        } else if op == SendOpcode::DropItemFromMapObject as i16 {
            r.read_byte().unwrap();
            let object_id = r.read_int().unwrap();
            let meso = r.read_byte().unwrap() != 0;
            let value = r.read_int().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            let (x, y) = (r.read_short().unwrap(), r.read_short().unwrap());
            self.drops.push(MapDrop {
                object_id,
                meso,
                value,
                x,
                y,
            });
        } else if op == SendOpcode::RemoveItemFromMap as i16 {
            let mode = r.read_byte().unwrap();
            let oid = r.read_int().unwrap();
            self.removed
                .push((oid, (mode >= 2).then(|| r.read_int().unwrap())));
        } else if op == SendOpcode::SpawnNpc as i16 {
            self.npcs
                .push((r.read_int().unwrap(), r.read_int().unwrap()));
        } else if op == SendOpcode::ShowForeignEffect as i16 {
            self.effects
                .push((r.read_int().unwrap(), r.read_byte().unwrap()));
        } else if op == SendOpcode::SpawnReactor as i16 {
            self.reactors.push(MapReactor {
                object_id: r.read_int().unwrap(),
                reactor_id: r.read_int().unwrap(),
                state: r.read_byte().unwrap(),
                x: r.read_short().unwrap(),
                y: r.read_short().unwrap(),
            });
        } else if op == SendOpcode::HitReactor as i16 {
            self.reactor_hits
                .push((r.read_int().unwrap(), r.read_byte().unwrap()));
        } else if op == SendOpcode::RemoveReactor as i16 {
            self.reactors_removed
                .push((r.read_int().unwrap(), r.read_byte().unwrap()));
        }
    }

    /// The object id of NPC `npc_id`, once it has spawned.
    pub fn npc(&self, npc_id: i32) -> Option<i32> {
        self.npcs
            .iter()
            .find(|&&(_, npc)| npc == npc_id)
            .map(|&(object_id, _)| object_id)
    }

    /// The first item drop of `item_id`.
    pub fn item_drop(&self, item_id: i32) -> Option<MapDrop> {
        self.drops
            .iter()
            .find(|d| !d.meso && d.value == item_id)
            .copied()
    }

    /// Who took a drop off the map each time it was removed.
    pub fn removals(&self, object_id: i32) -> Vec<Option<i32>> {
        self.removed
            .iter()
            .filter(|(oid, _)| *oid == object_id)
            .map(|(_, by)| *by)
            .collect()
    }

    /// Object ids of living mobs of `mob_id`.
    pub fn alive(&self, mob_id: i32) -> Vec<i32> {
        self.mobs
            .iter()
            .filter(|(oid, id)| *id == mob_id && !self.killed.contains(oid))
            .map(|(oid, _)| *oid)
            .collect()
    }
}
