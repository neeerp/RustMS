use super::check_name::name_is_available;
use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::packet::build::login;
use db::character::NewCharacter;
use game::inventory::equip_slot;
use game::{InventoryType, Item};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct CreateCharacterHandler;

impl LoginHandler for CreateCharacterHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;

        let session = ctx.session()?;
        let name = reader.read_str_with_length()?;
        // The creation packet carries a class-family flag, not a job ID.
        let job = starting_job(reader.read_int()?)?;
        let face = reader.read_int()?;
        let hair = reader.read_int()?;
        let hair_color = reader.read_int()?;
        let skin = reader.read_int()?;
        let top = reader.read_int()?;
        let bottom = reader.read_int()?;
        let shoes = reader.read_int()?;
        let weapon = reader.read_int()?;
        let gender = reader.read_byte()?;

        if !name_is_available(&name)? {
            return Err(NetworkError::PacketHandlerError(
                "Character name is invalid or taken",
            ));
        }
        validate_appearance(face, hair, hair_color, skin, gender)?;
        let starting_items = starting_equipment(top, bottom, shoes, weapon)?;

        let character = NewCharacter {
            accountid: session.account_id,
            world: session.selected_world_id.unwrap_or(0),
            name: &name,
            job,
            face,
            // The client renders hair as style + color digit.
            hair: hair + hair_color,
            hair_color,
            skin,
            gender: i16::from(gender),
            // The Explorer tutorial exits lead one-way toward Amherst. Starting
            // in Amherst skips the existing Heena/Nina quests entirely.
            map_id: if job == 0 { 10_000 } else { 1_000_000 },
        };

        let (stats, look) = character.create(starting_items)?;
        Ok(HandlerResult::reply(login::char::build_char_packet(
            &stats, &look,
        )?))
    }
}

fn starting_job(class_family: i32) -> Result<i16, NetworkError> {
    match class_family {
        0 => Ok(1000), // Cygnus Noblesse
        1 => Ok(0),    // Explorer Beginner
        2 => Ok(2000), // Aran Legend
        _ => Err(NetworkError::PacketHandlerError(
            "Unsupported character class",
        )),
    }
}

/// Character names: 4 to 12 ASCII letters or digits.
pub(super) fn name_is_valid(name: &str) -> bool {
    (4..=12).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Bounds checks on creation choices. The exact allowed sets live in
/// Etc.wz/MakeCharInfo, which the server does not load yet.
fn validate_appearance(
    face: i32,
    hair: i32,
    hair_color: i32,
    skin: i32,
    gender: u8,
) -> Result<(), NetworkError> {
    let valid = (20000..22000).contains(&face)
        && (30000..32000).contains(&hair)
        && hair % 10 == 0
        && (0..8).contains(&hair_color)
        && (0..12).contains(&skin)
        && gender <= 1;
    if valid {
        Ok(())
    } else {
        Err(NetworkError::PacketHandlerError(
            "Invalid character appearance",
        ))
    }
}

/// The chosen starting clothes and weapon, equipped. Each id must belong to
/// the slot it was offered for. Equipment gets its base stats from
/// Character.nx when the item data is available.
fn starting_equipment(
    top: i32,
    bottom: i32,
    shoes: i32,
    weapon: i32,
) -> Result<Vec<(InventoryType, i16, Item)>, NetworkError> {
    let mut items = Vec::new();
    for (item_id, slot) in [
        (top, equip_slot::TOP),
        (bottom, equip_slot::BOTTOM),
        (shoes, equip_slot::SHOES),
        (weapon, equip_slot::WEAPON),
    ] {
        // Overalls leave the bottom slot empty.
        if item_id == 0 && slot == equip_slot::BOTTOM {
            continue;
        }
        if equip_slot::for_item(item_id) != Some(slot) {
            return Err(NetworkError::PacketHandlerError(
                "Invalid starting equipment",
            ));
        }
        let stats = crate::items::info(item_id)
            .and_then(|info| info.equip)
            .map(|equip| crate::items::base_stats(&equip))
            .unwrap_or_default();
        items.push((
            InventoryType::Equipped,
            slot,
            Item::equipment(item_id, stats),
        ));
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_flags_are_converted_to_actual_starting_jobs() {
        assert_eq!(starting_job(1).unwrap(), 0);
        assert_eq!(starting_job(0).unwrap(), 1000);
        assert_eq!(starting_job(2).unwrap(), 2000);
        assert!(starting_job(-1).is_err());
        assert!(starting_job(100).is_err());
    }

    #[test]
    fn names_must_be_short_and_alphanumeric() {
        assert!(name_is_valid("Maple123"));
        assert!(!name_is_valid("abc"));
        assert!(!name_is_valid("thirteenchars"));
        assert!(!name_is_valid("bad name"));
        assert!(!name_is_valid("ünï"));
    }

    #[test]
    fn starting_equipment_must_match_its_slot() {
        let items = starting_equipment(1040002, 1060002, 1072001, 1302000).unwrap();
        assert_eq!(items.len(), 4);
        assert!(starting_equipment(1060002, 1040002, 1072001, 1302000).is_err());
        // Overall with no bottom.
        assert_eq!(
            starting_equipment(1052122, 0, 1072001, 1302000)
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn appearance_is_bounds_checked() {
        assert!(validate_appearance(20000, 30030, 0, 0, 0).is_ok());
        assert!(validate_appearance(20000, 30031, 0, 0, 0).is_err());
        assert!(validate_appearance(1, 30030, 0, 0, 0).is_err());
        assert!(validate_appearance(20000, 30030, 9, 0, 0).is_err());
    }
}
