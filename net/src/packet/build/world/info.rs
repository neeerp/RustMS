use crate::{error::NetworkError, packet::op::SendOpcode};
use game::inventory::equip_slot;
use game::{InventoryType, Player};
use packet::{io::write::PktWrite, Packet};

/// CHAR_INFO: what the character info window shows about `player`. There are
/// no alliances, wish lists, or monster books yet, so those are empty.
pub fn build_character_info(player: &Player) -> Result<Packet, NetworkError> {
    let stats = &player.stats;
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CharInfo as i16)?;
    packet.write_int(stats.id)?;
    packet.write_byte(stats.level)?;
    packet.write_short(stats.job)?;
    packet.write_short(stats.fame)?;
    packet.write_byte(0)?; // married
    let guild = player.guild.as_ref();
    packet.write_str_with_length(guild.map_or("", |guild| &guild.name))?;
    packet.write_str_with_length("")?; // alliance
    packet.write_byte(0)?; // medal info
    for item in player.summoned_pets() {
        let Some(pet) = &item.pet else { continue };
        packet.write_byte(1)?;
        packet.write_int(item.item_id)?;
        packet.write_str_with_length(&pet.name)?;
        packet.write_byte(pet.level)?;
        packet.write_short(pet.closeness)?;
        packet.write_byte(pet.fullness)?;
        packet.write_short(0)?;
        packet.write_int(0)?; // its equipment: pets wear none yet
    }
    packet.write_byte(0)?; // end of pets

    // The mount, for one worn (v83 0x903ce7 reads its stats after any
    // nonzero byte).
    if player
        .inventory
        .get(InventoryType::Equipped, equip_slot::MOUNT)
        .is_some()
    {
        packet.write_byte(1)?;
        packet.write_int(player.mount.level)?;
        packet.write_int(player.mount.exp)?;
        packet.write_int(player.mount.tiredness)?;
    } else {
        packet.write_byte(0)?;
    }
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
    use game::guild::{Emblem, GuildTag};
    use game::{EquipStats, Item};

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

    #[test]
    fn shows_the_pets_out() {
        let mut player = crate::codec::stats::sample_player();
        let pet = |serial, name: &str| Item {
            cash_serial: Some(serial),
            pet: Some(game::PetData {
                level: 3,
                closeness: 40,
                fullness: 80,
                ..game::PetData::new(name)
            }),
            ..Item::stack(5000066, 1)
        };
        player
            .inventory
            .insert(InventoryType::Cash, 1, pet(77, "Tigger"));
        player
            .inventory
            .insert(InventoryType::Cash, 2, pet(78, "Stays home"));
        player.pets = vec![77];
        let packet = build_character_info(&player).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CharInfo as i16).int(0x0102_0304);
        expect.byte(10).short(100).short(7).byte(0);
        expect.string("").string("").byte(0);
        expect
            .byte(1)
            .int(5000066)
            .string("Tigger")
            .byte(3)
            .short(40)
            .byte(80);
        expect.short(0).int(0);
        expect.byte(0); // end of pets
        expect.byte(0).byte(0);
    }

    #[test]
    fn shows_the_mount_worn() {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::MOUNT,
            Item::equipment(1902000, EquipStats::default()),
        );
        player.mount = game::Mount {
            level: 5,
            exp: 200,
            tiredness: 40,
        };
        let packet = build_character_info(&player).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CharInfo as i16).int(0x0102_0304);
        expect.byte(10).short(100).short(7).byte(0);
        expect.string("").string("").byte(0);
        expect.byte(0); // end of pets
        expect.byte(1).int(5).int(200).int(40);
        expect.byte(0); // wish list
    }

    #[test]
    fn shows_the_guild_name() {
        let mut player = crate::codec::stats::sample_player();
        player.guild = Some(GuildTag {
            id: 4,
            name: "Testers".into(),
            emblem: Emblem::default(),
        });
        let packet = build_character_info(&player).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CharInfo as i16)
            .int(0x0102_0304)
            .byte(10)
            .short(100)
            .short(7)
            .byte(0)
            .string("Testers")
            .string("");
    }
}
