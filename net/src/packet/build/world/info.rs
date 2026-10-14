use crate::{error::NetworkError, packet::op::SendOpcode};
use game::inventory::equip_slot;
use game::Player;
use packet::{io::write::PktWrite, Packet};

/// CHAR_INFO: what the character info window shows about `player`. There are
/// no guilds, pets, mounts, wish lists, or monster books yet, so those are
/// empty.
pub fn build_character_info(player: &Player) -> Result<Packet, NetworkError> {
    let stats = &player.stats;
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CharInfo as i16)?;
    packet.write_int(stats.id)?;
    packet.write_byte(stats.level)?;
    packet.write_short(stats.job)?;
    packet.write_short(stats.fame)?;
    packet.write_byte(0)?; // married
    packet.write_str_with_length("")?; // guild
    packet.write_str_with_length("")?; // alliance
    packet.write_byte(0)?; // medal info
    packet.write_byte(0)?; // end of pets
    packet.write_byte(0)?; // mount
    packet.write_byte(0)?; // wish list size
    for _ in 0..5 {
        // Monster book level, normal, special, and total cards, and cover.
        packet.write_int(0)?;
    }
    let medal = player
        .inventory
        .equipped_ids()
        .find(|&(slot, _)| slot == equip_slot::MEDAL)
        .map_or(0, |(_, id)| id);
    packet.write_int(medal)?;
    packet.write_short(0)?; // completed medal quests
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::{EquipStats, InventoryType, Item};

    #[test]
    fn matches_the_v83_layout() {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::MEDAL,
            Item::equipment(1142000, EquipStats::default()),
        );
        let packet = build_character_info(&player).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CharInfo as i16)
            .int(0x0102_0304)
            .byte(10)
            .short(100)
            .short(7)
            .byte(0)
            .string("")
            .string("")
            .byte(0)
            .byte(0)
            .byte(0)
            .byte(0);
        for _ in 0..5 {
            expect.int(0);
        }
        expect.int(1142000).short(0).end();
    }
}
