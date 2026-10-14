use crate::codec::{encode_character_data, time::filetime};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::Player;
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

/// SET_FIELD for world entry, carrying the full character data block.
pub fn build_char_info(player: &Player, channel_id: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetField as i16)?;
    packet.write_int(i32::from(channel_id))?;
    packet.write_byte(1)?;
    packet.write_byte(1)?;
    packet.write_short(0)?;

    // Seeds for the client's damage RNG.
    for _ in 0..3 {
        packet.write_int(rand::random::<i32>())?;
    }

    encode_character_data(&mut packet, player)?;
    packet.write_long(filetime(SystemTime::now()))?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_stats;
    use crate::codec::testing::Expect;
    use crate::codec::time::{DEFAULT_TIME, ZERO_TIME};
    use game::inventory::equip_slot;
    use game::{
        EquipStats, Inventory, InventoryType, Item, Keymap, MonsterBook, QuestLog, QuestRecord,
        QuestStatus, SkillBook, SkillRecord,
    };
    use std::time::{Duration, UNIX_EPOCH};

    /// FILETIME of the unix epoch plus 1000 seconds.
    const FILETIME_1000S: i64 = 116_444_746_000_000_000;

    fn player() -> Player {
        let mut inventory = Inventory::default();
        inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(
                1302000,
                EquipStats {
                    upgrade_slots: 7,
                    level: 1,
                    str: 3,
                    watk: 17,
                    vicious: 2,
                    item_level: 1,
                    ..EquipStats::default()
                },
            ),
        );
        let mut cash_cap = Item::equipment(1002186, EquipStats::default());
        cash_cap.cash_serial = Some(77);
        inventory.insert(InventoryType::Equipped, equip_slot::CAP - 100, cash_cap);
        inventory.insert(InventoryType::Use, 1, Item::stack(2000000, 12));
        inventory.insert(InventoryType::Use, 2, Item::stack(2070000, 500));

        let skills = SkillBook::from_records([
            SkillRecord {
                skill_id: 1001004,
                level: 3,
                master_level: 0,
                expiration: None,
            },
            SkillRecord {
                skill_id: 1121008,
                level: 5,
                master_level: 30,
                expiration: Some(UNIX_EPOCH + Duration::from_secs(1_000)),
            },
        ]);
        let quests = QuestLog::from_records([
            QuestRecord {
                quest_id: 1021,
                status: QuestStatus::Started { kills: Vec::new() },
            },
            QuestRecord {
                quest_id: 2000,
                status: QuestStatus::Completed {
                    at: UNIX_EPOCH + Duration::from_secs(1_000),
                },
            },
        ]);

        let mut player = Player::new(
            9,
            0,
            sample_stats(),
            12345,
            inventory,
            skills,
            Keymap::defaults(),
            quests,
        );
        player.buddy_capacity = 30;
        player.monster_book = MonsterBook::new(2380001, [(2380000, 5), (2380001, 2)]);
        player
    }

    #[test]
    fn set_field_for_world_entry_matches_reference() {
        let player = player();
        let packet = build_char_info(&player, 2).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SetField as i16);
        expect.int(2).byte(1).byte(1).short(0);
        expect.skip(12); // three random damage seeds

        // addCharacterInfo
        expect.long(-1).byte(0);
        expect.stats(&player.stats);
        expect.byte(30); // buddy capacity
        expect.byte(0); // no linked name
        expect.int(12345);

        // addInventoryInfo: slot limits, then the equip timestamp
        expect.byte(24).byte(24).byte(24).byte(24).byte(96);
        expect.long(ZERO_TIME);
        // equipped regular: the weapon at position 11
        expect
            .short(11)
            .byte(1)
            .int(1302000)
            .byte(0)
            .long(DEFAULT_TIME); // no expiration
        expect.byte(7).byte(1);
        expect.short(3).short(0).short(0).short(0); // str dex int luk
        expect.short(0).short(0); // hp mp
        expect.short(17).short(0); // watk matk
        expect.short(0).short(0).short(0).short(0); // wdef mdef acc avoid
        expect.short(0).short(0).short(0); // hands speed jump
        expect.string("").short(0);
        expect.byte(0).byte(1).int(0).int(2).long(0);
        expect.long(ZERO_TIME).int(-1);
        expect.short(0); // end of regular equipment
                         // equipped cash: the cash cap at position 101 -> 1
        expect
            .short(1)
            .byte(1)
            .int(1002186)
            .byte(1)
            .long(77)
            .long(DEFAULT_TIME);
        expect.byte(0).byte(0);
        for _ in 0..15 {
            expect.short(0);
        }
        expect.string("").short(0);
        expect.bytes(&[0x40; 10]);
        expect.long(ZERO_TIME).int(-1);
        expect.short(0); // end of cash equipment
        expect.int(0); // equip bag is empty
                       // use bag: potion, then throwing stars with the recharge block
        expect
            .byte(1)
            .byte(2)
            .int(2000000)
            .byte(0)
            .long(DEFAULT_TIME);
        expect.short(12).string("").short(0);
        expect
            .byte(2)
            .byte(2)
            .int(2070000)
            .byte(0)
            .long(DEFAULT_TIME);
        expect.short(500).string("").short(0);
        expect.int(2).bytes(&[0x54, 0, 0, 0x34]);
        expect.byte(0); // end of use
        expect.byte(0); // end of setup
        expect.byte(0); // end of etc
        expect.byte(0); // end of cash (doubles as the skills-section marker)

        // addSkillInfo
        expect.short(2);
        expect.int(1001004).int(3).long(DEFAULT_TIME);
        expect.int(1121008).int(5).long(FILETIME_1000S).int(30); // 4th job: master level
        expect.short(0); // cooldowns

        // addQuestInfo
        expect.short(1).short(1021).string("");
        expect.short(1).short(2000).long(FILETIME_1000S);

        expect.short(0); // minigames
        expect.short(0).short(0).short(0); // crush, friendship, marriage rings
        for _ in 0..15 {
            expect.int(999_999_999); // teleport rocks (5 regular + 10 VIP)
        }
        // monster book: the cover card, then each card's number and count
        expect.int(2380001).byte(0).short(2);
        expect.short(0).byte(5).short(1).byte(2);
        expect.short(0); // new year cards
        expect.short(0); // area info
        expect.short(0);

        expect.skip(8); // FILETIME now
        expect.end();
    }
}
