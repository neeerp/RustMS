use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// FIELD_EFFECT mode 3: play `Map.nx/Effect.img/<path>` on the screen, e.g.
/// a map name banner (`maplemap/enter/10000`).
pub fn build_field_effect(path: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldEffect as i16)?;
    p.write_byte(3)?;
    p.write_str_with_length(path)?;
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT mode 0x17: show a UI image (`UI/tutorial.img/25`)
/// anchored to the player, as the tutorial hints do.
pub fn build_avatar_effect(path: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(0x17)?;
    p.write_str_with_length(path)?;
    p.write_int(1)?;
    Ok(p)
}

/// An effect played on a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharEffect {
    LevelUp = 0,
    QuestClear = 9,
}

/// SHOW_ITEM_GAIN_INCHAT: `effect` on this player.
pub fn build_own_effect(effect: CharEffect) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(effect as u8)?;
    Ok(p)
}

/// SHOW_FOREIGN_EFFECT: `effect` on another player.
pub fn build_foreign_effect(character_id: i32, effect: CharEffect) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowForeignEffect as i16)?;
    p.write_int(character_id)?;
    p.write_byte(effect as u8)?;
    Ok(p)
}

/// PLAYER_HINT: an instruction balloon above the player.
pub fn build_hint(text: &str, width: i16, height: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PlayerHint as i16)?;
    p.write_str_with_length(text)?;
    p.write_short(width)?;
    p.write_short(height)?;
    p.write_byte(1)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn field_effect_is_mode_three_with_a_path() {
        let packet = build_field_effect("maplemap/enter/10000").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::FieldEffect as i16).byte(3);
        expect.string("maplemap/enter/10000");
        expect.end();
    }

    #[test]
    fn avatar_effect_carries_the_ui_path() {
        let packet = build_avatar_effect("UI/tutorial.img/25").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ShowItemGainInChat as i16)
            .byte(0x17);
        expect.string("UI/tutorial.img/25").int(1);
        expect.end();
    }

    #[test]
    fn own_effect_is_just_the_effect() {
        let packet = build_own_effect(CharEffect::QuestClear).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowItemGainInChat as i16).byte(9);
        expect.end();
    }

    #[test]
    fn foreign_effect_names_the_player() {
        let packet = build_foreign_effect(7, CharEffect::LevelUp).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ShowForeignEffect as i16)
            .int(7)
            .byte(0);
        expect.end();
    }

    #[test]
    fn hint_has_text_size_and_trailer() {
        let packet = build_hint("Once you leave", 150, 5).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PlayerHint as i16)
            .string("Once you leave");
        expect.short(150).short(5).byte(1);
        expect.end();
    }
}
