use crate::codec::time::expiration;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::SkillRecord;
use packet::{io::write::PktWrite, Packet};

/// UPDATE_SKILLS for one skill. It also unlocks the client UI.
pub fn build_update_skill(skill: &SkillRecord) -> Result<Packet, NetworkError> {
    build_update_skills(std::slice::from_ref(skill))
}

/// UPDATE_SKILLS for several skills at once (v83 FUN_00a1e48c reads a
/// count). It also unlocks the client UI.
pub fn build_update_skills(skills: &[SkillRecord]) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::UpdateSkills as i16)?;
    packet.write_byte(1)?;
    packet.write_short(skills.len() as i16)?;
    for skill in skills {
        packet.write_int(skill.skill_id)?;
        packet.write_int(skill.level)?;
        packet.write_int(skill.master_level)?;
        packet.write_long(expiration(skill.expiration))?;
    }
    packet.write_byte(4)?;
    Ok(packet)
}

/// How reading a skill or mastery book went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookResult {
    pub character_id: i32,
    /// A mastery book (229xxxx) rather than a skill book (228xxxx).
    pub mastery: bool,
    pub skill_id: i32,
    pub master_level: i32,
    /// The reader could use it: it was spent.
    pub used: bool,
    pub success: bool,
}

/// SKILL_LEARN_ITEM_RESULT (0x33, v83 FUN_00a1e5af): the map sees the book's
/// glow on the reader when it was used; the reader also gets the chat line
/// and their next request.
pub fn build_skill_book_result(result: &BookResult) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SkillBookResult as i16)?;
    packet.write_int(result.character_id)?;
    packet.write_byte(u8::from(result.mastery))?;
    packet.write_int(result.skill_id)?;
    packet.write_int(result.master_level)?;
    packet.write_byte(u8::from(result.used))?;
    packet.write_byte(u8::from(result.success))?;
    Ok(packet)
}

/// SKILL_EFFECT: another player started preparing a skill. The client
/// takes `action_and_facing` and `speed` as that player sent them.
pub fn build_skill_prepare(
    character_id: i32,
    skill_id: i32,
    level: u8,
    action_and_facing: u16,
    speed: u8,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SkillEffect as i16)?;
    packet.write_int(character_id)?;
    packet.write_int(skill_id)?;
    packet.write_byte(level)?;
    packet.write_short(action_and_facing as i16)?;
    packet.write_byte(speed)?;
    Ok(packet)
}

/// CANCEL_SKILL_EFFECT: another player let go of a held skill.
pub fn build_cancel_skill_prepare(
    character_id: i32,
    skill_id: i32,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CancelSkillEffect as i16)?;
    packet.write_int(character_id)?;
    packet.write_int(skill_id)?;
    Ok(packet)
}

/// THROW_GRENADE: another player threw a grenade from where they stood,
/// held for `charge` ms (0x983bc1).
pub fn build_throw_grenade(
    character_id: i32,
    x: i32,
    y: i32,
    charge: i32,
    skill_id: i32,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ThrowGrenade as i16)?;
    packet.write_int(character_id)?;
    packet.write_int(x)?;
    packet.write_int(y)?;
    packet.write_int(charge)?;
    packet.write_int(skill_id)?;
    Ok(packet)
}

/// ADMIN_RESULT 0x10: whether the GM is now hidden. The client keeps the
/// byte (FUN_005352e9); others stop seeing the GM through the field.
pub fn build_hidden(hidden: bool) -> Result<Packet, NetworkError> {
    build_admin_result(0x10, u8::from(hidden))
}

/// ADMIN_RESULT 0x90: how a GM's slash command `mode` went, in one byte
/// (FUN_005352e9).
pub fn build_admin_result(mode: u8, value: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::AdminResult as i16)?;
    packet.write_byte(mode)?;
    packet.write_byte(value)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::codec::time::DEFAULT_TIME;

    #[test]
    fn one_skill_with_its_level_master_level_and_expiration() {
        let packet = build_update_skill(&SkillRecord {
            skill_id: 2001002,
            level: 3,
            master_level: 0,
            expiration: None,
        })
        .unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::UpdateSkills as i16)
            .byte(1)
            .short(1);
        expect.int(2001002).int(3).int(0).long(DEFAULT_TIME).byte(4);
        expect.end();
    }

    #[test]
    fn hiding_tells_the_gm_with_admin_result_0x10() {
        let packet = build_hidden(true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::AdminResult as i16)
            .byte(0x10)
            .byte(1);
        expect.end();
    }

    #[test]
    fn a_book_result_names_the_reader_skill_and_outcome() {
        let packet = build_skill_book_result(&BookResult {
            character_id: 7,
            mastery: true,
            skill_id: 1121001,
            master_level: 20,
            used: true,
            success: false,
        })
        .unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x33).int(7).byte(1).int(1121001).int(20);
        expect.byte(1).byte(0).end();
    }

    #[test]
    fn a_prepare_passes_on_the_action_and_facing_and_speed() {
        let packet = build_skill_prepare(7, 3121004, 20, 0x8016, 4).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SkillEffect as i16);
        expect
            .int(7)
            .int(3121004)
            .byte(20)
            .short(0x8016_u16 as i16)
            .byte(4);
        expect.end();
    }

    #[test]
    fn a_cancel_names_the_player_and_skill() {
        let packet = build_cancel_skill_prepare(7, 5221004).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CancelSkillEffect as i16);
        expect.int(7).int(5221004);
        expect.end();
    }

    #[test]
    fn a_throw_names_the_player_where_they_stood_and_the_charge() {
        let packet = build_throw_grenade(7, 120, -45, 640, 5201002).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ThrowGrenade as i16);
        expect.int(7).int(120).int(-45).int(640).int(5201002);
        expect.end();
    }
}
