//! Hunting as HeavenClient does it: walking, melee attacks, picking up
//! drops, and using items, plus a record of what a test has seen on its
//! field.

use crate::packets::build_player_move;
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::collections::BTreeSet;
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

/// CANCEL_BUFF: right-clicking a skill's buff icon.
pub fn cancel_buff(skill_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CancelBuff as i16).unwrap();
    p.write_int(skill_id).unwrap();
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
            self.move_responses.push(r.read_int().unwrap());
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
