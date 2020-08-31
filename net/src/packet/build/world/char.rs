use crate::{error::NetworkError, packet::op::SendOpcode};
use db::character::Character;
use packet::{io::write::PktWrite, Packet};

use std::time::{SystemTime, UNIX_EPOCH};

// TODO: This is just a barebones implementation.
pub fn _build_update_buddy_list() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();

    let op = SendOpcode::SetField as i16;
    packet.write_short(op)?;

    packet.write_byte(7)?;
    packet.write_byte(0)?;

    Ok(packet)
}

// TODO: This is just a barebones implementation.
pub fn _build_load_family(_character: &Character) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();

    let op = SendOpcode::SetField as i16;
    packet.write_short(op)?;

    packet.write_int(0)?;

    Ok(packet)
}

pub fn _build_family_info() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();

    let op = SendOpcode::FamilyInfo as i16;
    packet.write_short(op)?;

    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_short(0)?;
    packet.write_short(2)?;
    packet.write_short(0)?;
    packet.write_int(0)?;
    packet.write_str_with_length("")?;
    packet.write_str_with_length("")?;
    packet.write_int(0)?;

    Ok(packet)
}

pub fn build_char_info(character: &Character) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();

    let op = SendOpcode::SetField as i16;
    packet.write_short(op)?;

    let channel = 0;
    packet.write_int(channel)?;

    packet.write_byte(1)?;
    packet.write_byte(1)?;
    packet.write_short(0)?;

    // These are random... No idea what they are though.
    packet.write_int(1)?;
    packet.write_int(2)?;
    packet.write_int(3)?;

    write_char(&mut packet, &character)?;

    let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;
    packet.write_long(time)?;

    Ok(packet)
}

fn write_char(packet: &mut Packet, character: &Character) -> Result<(), NetworkError> {
    packet.write_long(-1)?;
    packet.write_byte(0)?;

    write_char_meta(packet, character)?;

    let bl_capacity = 25;
    packet.write_byte(bl_capacity)?;

    packet.write_byte(0)?;

    packet.write_int(character.meso)?;

    write_inventory(packet, character)?;
    write_skills(packet, character)?;
    write_quests(packet, character)?;
    write_minigames(packet, character)?;
    write_rings(packet, character)?;
    write_teleport(packet, character)?;
    write_codex(packet, character)?;
    write_new_year_cards(packet, character)?;
    write_area_info(packet, character)?;

    packet.write_byte(0)?;

    Ok(())
}

/// Write the equiped items and the inventory of the player
fn write_inventory(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    // Inventory slot Capacities
    packet.write_bytes(&vec![0u8; 5])?;

    // Time?
    packet.write_long(0)?;

    // Equiped items go here

    // Start of equiped cash items
    packet.write_short(0)?;

    // Start of equiped inventory
    packet.write_short(0)?;

    // Start of USE
    packet.write_int(0)?;

    // Start of SETUP
    packet.write_byte(0)?;

    // Start of ETC
    packet.write_byte(0)?;

    // Start of CASH
    packet.write_byte(0)?;

    Ok(())
}

fn write_skills(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    // Start of skills
    packet.write_byte(0)?;

    // No skills!
    packet.write_short(0)?;

    // No no cooldowns!
    packet.write_short(0)?;

    Ok(())
}

fn write_quests(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    let started_quests = 0;
    packet.write_short(started_quests)?;

    let completed_quests = 0;
    packet.write_short(completed_quests)?;

    Ok(())
}

fn write_minigames(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    // This ones required but kinda useless...
    packet.write_short(0)?;
    Ok(())
}

fn write_rings(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    let num_crush_rings = 0;
    let num_friendship_rings = 0;
    packet.write_short(num_crush_rings)?;
    packet.write_short(num_friendship_rings)?;

    // Not married
    packet.write_short(0)?;

    Ok(())
}

fn write_teleport(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    // Regular tele rock locations
    for _ in 0..5 {
        packet.write_int(0)?;
    }

    // VIP tele rock locations
    for _ in 0..10 {
        packet.write_int(0)?;
    }

    Ok(())
}

fn write_codex(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    let codex_cover = 1;
    let num_cards = 0;

    packet.write_int(codex_cover)?;
    packet.write_byte(0)?;
    packet.write_short(num_cards)?;

    Ok(())
}

// I have literally no idea what these are...
fn write_new_year_cards(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    let num_cards = 0;
    packet.write_short(num_cards)?;
    Ok(())
}

fn write_area_info(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    let num_areas = 0;
    packet.write_short(num_areas)?;
    Ok(())
}

fn write_char_meta(packet: &mut Packet, character: &Character) -> Result<(), NetworkError> {
    packet.write_int(character.id)?;
    packet.write_str(&character.name)?;
    packet.write_bytes(&vec![0u8; 13 - character.name.len()])?;
    packet.write_byte(character.gender as u8)?;
    packet.write_byte(character.skin as u8)?;
    packet.write_int(character.face)?;
    packet.write_int(character.hair)?;

    // Pets... Not implemented yet
    packet.write_long(0)?;
    packet.write_long(0)?;
    packet.write_long(0)?;

    packet.write_byte(character.level as u8)?;
    packet.write_short(character.job)?;

    packet.write_short(character.stre)?;
    packet.write_short(character.dex)?;
    packet.write_short(character.int)?;
    packet.write_short(character.luk)?;
    packet.write_short(character.hp)?;
    packet.write_short(character.maxhp)?;
    packet.write_short(character.mp)?;
    packet.write_short(character.maxmp)?;
    packet.write_short(character.ap)?;

    // SP
    packet.write_short(0)?;

    packet.write_int(character.exp)?;
    packet.write_short(character.fame)?;

    // Gach xp?
    packet.write_int(0)?;

    // TODO: Map and initial spawn: Gotta do these here.
    let map_id = 1000000;
    packet.write_int(map_id)?;
    packet.write_byte(0)?;

    packet.write_int(0)?;

    Ok(())
}
