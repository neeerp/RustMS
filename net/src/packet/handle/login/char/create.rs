use super::check_name::check_name;
use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::load::{self, asset_path};
use crate::packet::build::login;
use crate::packet::build::login::char::{CreateFailure, NameCheck};
use db::character::NewCharacter;
use game::inventory::equip_slot;
use game::{InventoryType, Item};
use game_data::make_char_info::MakeCharInfo;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::sync::OnceLock;

pub struct CreateCharacterHandler;

impl LoginHandler for CreateCharacterHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;

        let session = ctx.session()?;
        let name = reader.read_str_with_length()?;
        // The creation packet carries a class-family flag, not a job ID.
        let class_family = reader.read_int()?;
        let job = starting_job(class_family)?;
        let face = reader.read_int()?;
        let hair = reader.read_int()?;
        let hair_color = reader.read_int()?;
        let skin = reader.read_int()?;
        let top = reader.read_int()?;
        let bottom = reader.read_int()?;
        let shoes = reader.read_int()?;
        let weapon = reader.read_int()?;
        let gender = reader.read_byte()?;

        // The client checks all of this first, so only a forged packet or
        // a name taken meanwhile gets here. It waits for an answer either way.
        let fail = |reason| {
            Ok(HandlerResult::reply(login::char::build_create_failure(
                reason,
            )?))
        };
        if check_name(&name, session.account_id)? != NameCheck::Available {
            return fail(CreateFailure::NameRefused);
        }
        let slots = db::account::get_account_by_id(session.account_id)?.character_slots;
        let chosen = [face, hair, hair_color, skin, top, bottom, shoes, weapon];
        if db::character::count_characters(session.account_id)? >= i64::from(slots)
            || !creation_choices()
                .map_err(NetworkError::PacketHandlerError)?
                .allows(class_family, gender, chosen)
        {
            return fail(CreateFailure::Failed);
        }
        let Ok(starting_items) = starting_equipment(top, bottom, shoes, weapon) else {
            return fail(CreateFailure::Failed);
        };

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
            // Explorers start outside the Training Camp, with Heena and Sera.
            // The island's exits lead one-way from there, so starting further
            // along skips its first quests.
            map_id: if job == 0 { 0 } else { 1_010_000 },
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

/// The looks and starting equipment the client offers (`MakeCharInfo.img`).
fn creation_choices() -> Result<&'static MakeCharInfo, &'static str> {
    static CHOICES: OnceLock<Result<MakeCharInfo, String>> = OnceLock::new();
    load::once(&CHOICES, || {
        MakeCharInfo::load(asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"))
    })
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
    fn starting_equipment_must_match_its_slot() {
        let items = starting_equipment(1040002, 1060002, 1072001, 1302000).unwrap();
        assert_eq!(items.len(), 4);
        assert!(starting_equipment(1060002, 1040002, 1072001, 1302000).is_err());
    }
}
