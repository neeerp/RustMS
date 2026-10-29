use crate::{error::NetworkError, packet::op::SendOpcode};
use game::inventory::equip_slot;
use game::{InventoryType, Player};
use packet::{io::write::PktWrite, Packet};

/// The medal (title) quests start here.
const MEDAL_QUESTS: i16 = 29000;

/// CHAR_INFO: what the character info window shows about `player`, whose
/// guild is in the alliance `alliance` (empty for none), with their cash
/// shop wish list `wishes` (commodity SNs).
pub fn build_character_info(
    player: &Player,
    alliance: &str,
    wishes: &[i32],
) -> Result<Packet, NetworkError> {
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
    packet.write_str_with_length(alliance)?;
    packet.write_byte(0)?; // medal info
    for (index, item) in
        (0..player.pets.len()).filter_map(|index| Some((index, player.summoned_pet(index)?)))
    {
        let Some(pet) = &item.pet else { continue };
        packet.write_byte(1)?;
        packet.write_int(item.item_id)?;
        packet.write_str_with_length(&pet.name)?;
        packet.write_byte(pet.level)?;
        packet.write_short(pet.closeness)?;
        packet.write_byte(pet.fullness)?;
        packet.write_short(0)?;
        let clothes = equip_slot::PET_CLOTHES
            .get(index)
            .and_then(|&slot| player.pet_equip(slot));
        packet.write_int(clothes.map_or(0, |item| item.item_id))?;
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
    packet.write_byte(wishes.len() as u8)?;
    for &sn in wishes {
        packet.write_int(sn)?;
    }
    // The Monster Book (v83 FUN_00684798): its level, the different normal
    // and special cards, their total, and the cover as its card's mob, whose
    // `Mob/<id>.img` name the client shows.
    let book = &player.monster_book;
    let (normal, special) = (book.count(false), book.count(true));
    packet.write_int(book.book_level())?;
    packet.write_int(normal)?;
    packet.write_int(special)?;
    packet.write_int(normal + special)?;
    packet.write_int(crate::monster_book::mob_of(book.cover()).unwrap_or(0))?;
    let medal = player
        .inventory
        .equipped_ids()
        .find(|&(slot, _)| slot == equip_slot::MEDAL)
        .map_or(0, |(_, id)| id);
    packet.write_int(medal)?;
    // The medal quests done (from 29000, as Cosmic counts them), whose
    // `viewMedalItem` titles the client lists.
    let mut medals: Vec<i16> = player
        .quests
        .completed()
        .map(|(quest, _)| quest)
        .filter(|&quest| quest >= MEDAL_QUESTS)
        .collect();
    medals.sort_unstable();
    packet.write_short(medals.len() as i16)?;
    for quest in medals {
        packet.write_short(quest)?;
    }
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
        let packet = build_character_info(&player, "", &[]).unwrap();
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
        // A book of no cards is at level 1.
        expect.int(1).int(0).int(0).int(0).int(0);
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
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::PET_CLOTHES[0] - 100,
            Item::equipment(1802000, EquipStats::default()),
        );
        let packet = build_character_info(&player, "", &[]).unwrap();
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
        expect.short(0).int(1802000);
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
        let packet = build_character_info(&player, "", &[]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CharInfo as i16).int(0x0102_0304);
        expect.byte(10).short(100).short(7).byte(0);
        expect.string("").string("").byte(0);
        expect.byte(0); // end of pets
        expect.byte(1).int(5).int(200).int(40);
        expect.byte(0); // wish list
    }

    #[test]
    fn shows_the_guild_and_alliance_names() {
        let mut player = crate::codec::stats::sample_player();
        player.guild = Some(GuildTag {
            id: 4,
            name: "Testers".into(),
            emblem: Emblem::default(),
            alliance_id: 0,
        });
        let packet = build_character_info(&player, "Union", &[]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CharInfo as i16)
            .int(0x0102_0304)
            .byte(10)
            .short(100)
            .short(7)
            .byte(0)
            .string("Testers")
            .string("Union");
    }

    #[test]
    fn lists_the_wishes() {
        let player = crate::codec::stats::sample_player();
        let packet = build_character_info(&player, "", &[50200005, 20000053]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CharInfo as i16).int(0x0102_0304);
        expect.byte(10).short(100).short(7).byte(0);
        expect.string("").string("").byte(0).byte(0).byte(0);
        expect.byte(2).int(50200005).int(20000053);
    }

    #[test]
    fn shows_the_book_and_the_medal_quests() {
        use game::quest::{QuestLog, QuestRecord, QuestStatus};
        use std::time::UNIX_EPOCH;

        let mut player = crate::codec::stats::sample_player();
        player.monster_book =
            game::MonsterBook::new(2380001, [(2380000, 5), (2380001, 1), (2388000, 2)]);
        let done = |quest_id| QuestRecord {
            quest_id,
            status: QuestStatus::Completed { at: UNIX_EPOCH },
        };
        player.quests = QuestLog::from_records([
            done(29005),
            done(1000),
            done(29001),
            QuestRecord {
                quest_id: 29002,
                status: QuestStatus::Started {
                    kills: vec![],
                    info: None,
                },
            },
        ]);
        let packet = build_character_info(&player, "", &[]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CharInfo as i16).int(0x0102_0304);
        expect.byte(10).short(100).short(7).byte(0);
        expect.string("").string("").byte(0).byte(0).byte(0).byte(0);
        let cover = crate::monster_book::mob_of(2380001).unwrap_or(0);
        expect.int(1).int(2).int(1).int(3).int(cover);
        expect.int(0).short(2).short(29001).short(29005).end();
    }
}
