use crate::codec::time::expiration;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::SkillRecord;
use packet::{io::write::PktWrite, Packet};

/// UPDATE_SKILLS for one skill. It also unlocks the client UI.
pub fn build_update_skill(skill: &SkillRecord) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::UpdateSkills as i16)?;
    packet.write_byte(1)?;
    packet.write_short(1)?;
    packet.write_int(skill.skill_id)?;
    packet.write_int(skill.level)?;
    packet.write_int(skill.master_level)?;
    packet.write_long(expiration(skill.expiration))?;
    packet.write_byte(4)?;
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
}
