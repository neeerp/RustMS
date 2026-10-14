use super::item::encode_inventory;
use super::stats::encode_character_stats;
use super::time::{expiration, filetime};
use crate::error::NetworkError;
use game::quest::encode_kills;
use game::Player;
use packet::{io::write::PktWrite, Packet};

/// Teleport rock slots hold this map id when empty.
const EMPTY_TELEPORT_ROCK: i32 = 999_999_999;
const REGULAR_TELEPORT_ROCKS: usize = 5;
const VIP_TELEPORT_ROCKS: usize = 10;

/// Write the full character data block sent on world entry
/// (`addCharacterInfo` in HeavenMS).
pub fn encode_character_data(packet: &mut Packet, player: &Player) -> Result<(), NetworkError> {
    // Flags selecting which sections follow: all of them.
    packet.write_long(-1)?;
    packet.write_byte(0)?;
    encode_character_stats(packet, &player.stats)?;
    packet.write_byte(player.buddy_capacity)?;
    packet.write_byte(0)?; // no linked (blessing of the fairy) name
    packet.write_int(player.meso)?;
    encode_inventory(packet, &player.inventory)?;

    packet.write_short(player.skills.len() as i16)?;
    for skill in player.skills.iter() {
        packet.write_int(skill.skill_id)?;
        packet.write_int(skill.level)?;
        packet.write_long(expiration(skill.expiration))?;
        if skill.is_fourth_job() {
            packet.write_int(skill.master_level)?;
        }
    }
    packet.write_short(0)?; // cooldowns

    let started: Vec<_> = player.quests.started().collect();
    packet.write_short(started.len() as i16)?;
    for quest in started {
        packet.write_short(quest.quest_id)?;
        packet.write_str_with_length(&encode_kills(player.quests.kills(quest.quest_id)))?;
    }
    let completed: Vec<_> = player.quests.completed().collect();
    packet.write_short(completed.len() as i16)?;
    for (quest_id, at) in completed {
        packet.write_short(quest_id)?;
        packet.write_long(filetime(at))?;
    }

    packet.write_short(0)?; // minigame records
    packet.write_short(0)?; // crush rings
    packet.write_short(0)?; // friendship rings
    packet.write_short(0)?; // marriage
    for _ in 0..REGULAR_TELEPORT_ROCKS + VIP_TELEPORT_ROCKS {
        packet.write_int(EMPTY_TELEPORT_ROCK)?;
    }
    packet.write_int(player.monster_book.cover())?;
    packet.write_byte(0)?;
    let cards: Vec<_> = player.monster_book.cards().collect();
    packet.write_short(cards.len() as i16)?;
    for (card, level) in cards {
        // Cards go by their number within 0238.img.
        packet.write_short((card % 10_000) as i16)?;
        packet.write_byte(level)?;
    }
    packet.write_short(0)?; // new year cards
    packet.write_short(0)?; // area info
    packet.write_short(0)?;
    Ok(())
}
