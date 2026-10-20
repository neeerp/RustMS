use crate::{error::NetworkError, packet::op::SendOpcode};
use game::SkillMacros;
use packet::{io::write::PktWrite, Packet};

/// MACRO_SYS_DATA_INIT (124, v83 FUN_0050f9b4): the character's skill
/// macros, sent on every entry to the game, even with none, since the client
/// keeps the last list it was given.
pub fn build_skill_macros(macros: &SkillMacros) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MacroSysDataInit as i16)?;
    packet.write_byte(macros.macros().len() as u8)?;
    for m in macros.macros() {
        packet.write_str_with_length(&m.name)?;
        packet.write_byte(u8::from(m.silent))?;
        for skill in m.skills {
            packet.write_int(skill)?;
        }
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::SkillMacro;

    #[test]
    fn none_is_a_zero_count() {
        let packet = build_skill_macros(&SkillMacros::default()).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::MacroSysDataInit as i16)
            .byte(0)
            .end();
    }

    #[test]
    fn each_macro_is_its_name_silence_and_three_skills() {
        let macros = SkillMacros::new(vec![
            SkillMacro {
                name: "buffs".into(),
                silent: true,
                skills: [1001003, 1101006, 0],
            },
            SkillMacro {
                name: "hit".into(),
                silent: false,
                skills: [0, 0, 1001004],
            },
        ]);
        let packet = build_skill_macros(&macros).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::MacroSysDataInit as i16)
            .byte(2)
            .string("buffs")
            .byte(1)
            .int(1001003)
            .int(1101006)
            .int(0)
            .string("hit")
            .byte(0)
            .int(0)
            .int(0)
            .int(1001004)
            .end();
    }
}
