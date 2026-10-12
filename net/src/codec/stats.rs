use crate::error::NetworkError;
use game::CharacterStats;
use packet::{io::write::PktWrite, Packet};

/// Fixed width of the name field in the stat block.
const NAME_FIELD_LEN: usize = 13;

/// Write the stat block (`addCharStats` in HeavenMS).
pub fn encode_character_stats(
    packet: &mut Packet,
    stats: &CharacterStats,
) -> Result<(), NetworkError> {
    packet.write_int(stats.id)?;
    write_padded_name(packet, &stats.name)?;
    packet.write_byte(stats.gender)?;
    packet.write_byte(stats.skin)?;
    packet.write_int(stats.face)?;
    packet.write_int(stats.hair)?;

    // Pet unique ids for the three pet slots.
    for _ in 0..3 {
        packet.write_long(0)?;
    }

    packet.write_byte(stats.level)?;
    packet.write_short(stats.job)?;
    packet.write_short(stats.str)?;
    packet.write_short(stats.dex)?;
    packet.write_short(stats.int)?;
    packet.write_short(stats.luk)?;
    packet.write_short(stats.hp)?;
    packet.write_short(stats.max_hp)?;
    packet.write_short(stats.mp)?;
    packet.write_short(stats.max_mp)?;
    packet.write_short(stats.ap)?;
    packet.write_short(stats.sp)?;
    packet.write_int(stats.exp)?;
    packet.write_short(stats.fame)?;
    packet.write_int(0)?; // gachapon exp
    packet.write_int(stats.map_id)?;
    packet.write_byte(stats.spawn_point)?;
    packet.write_int(0)?;
    Ok(())
}

/// Names are a fixed 13-byte, NUL-padded field. Longer names are rejected
/// rather than silently truncated or underflowing the padding.
fn write_padded_name(packet: &mut Packet, name: &str) -> Result<(), NetworkError> {
    let bytes = name.as_bytes();
    if bytes.len() > NAME_FIELD_LEN {
        return Err(NetworkError::PacketHandlerError(
            "character name exceeds 13 bytes",
        ));
    }
    packet.write_bytes(bytes)?;
    packet.write_bytes(&[0u8; NAME_FIELD_LEN][..NAME_FIELD_LEN - bytes.len()])?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn sample_stats() -> CharacterStats {
    CharacterStats {
        id: 0x0102_0304,
        name: "Tester".to_string(),
        gender: 1,
        skin: 2,
        face: 20000,
        hair: 30030,
        level: 10,
        job: 100,
        str: 35,
        dex: 4,
        int: 4,
        luk: 4,
        hp: 300,
        max_hp: 310,
        mp: 50,
        max_mp: 55,
        ap: 3,
        sp: 1,
        exp: 1234,
        fame: 7,
        map_id: 100000000,
        spawn_point: 2,
    }
}

/// A player with `sample_stats` and nothing else, for handler tests.
#[cfg(test)]
pub(crate) fn sample_player() -> game::Player {
    game::Player::new(
        1,
        0,
        sample_stats(),
        0,
        game::Inventory::default(),
        game::SkillBook::default(),
        game::Keymap::default(),
        game::QuestLog::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn stat_block_matches_reference_layout() {
        let stats = sample_stats();
        let mut packet = Packet::new_empty();
        encode_character_stats(&mut packet, &stats).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.int(stats.id);
        expect.bytes(b"Tester\0\0\0\0\0\0\0");
        expect.byte(1).byte(2).int(20000).int(30030);
        expect.long(0).long(0).long(0);
        expect.byte(10).short(100);
        expect.short(35).short(4).short(4).short(4);
        expect.short(300).short(310).short(50).short(55);
        expect.short(3).short(1).int(1234).short(7).int(0);
        expect.int(100000000).byte(2).int(0);
        expect.end();
    }

    #[test]
    fn overlong_names_are_rejected() {
        let mut stats = sample_stats();
        stats.name = "ABCDEFGHIJKLMN".to_string();
        assert!(encode_character_stats(&mut Packet::new_empty(), &stats).is_err());
    }
}
