//! Pets (v83 CPet, report 159). Every packet starts with the owner's
//! character id, then the pet's position in the owner's pet list.

use crate::{error::NetworkError, packet::op::SendOpcode};
use game::FieldPet;
use packet::{io::write::PktWrite, Packet};

/// Why a pet went back, as its owner reads it (the others just see it go).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PetGone {
    Dismissed = 0,
    /// "The pet went back home because it's hungry." (StringPool 0x17e)
    Hungry = 1,
    /// "The pet's magical time has run out, ..." (StringPool 0x17f)
    Expired = 2,
    /// "You cannot use pet here."
    NotHere = 3,
}

/// A pet as the field shows it (`CPet::Init`, FUN_00703314), with whether
/// its label and quote rings are worn.
pub fn write_pet(packet: &mut Packet, pet: &FieldPet) -> Result<(), NetworkError> {
    packet.write_int(pet.item_id)?;
    packet.write_str_with_length(&pet.name)?;
    packet.write_long(pet.serial)?;
    packet.write_short(pet.x)?;
    packet.write_short(pet.y)?;
    packet.write_byte(pet.stance)?;
    packet.write_short(pet.foothold)?;
    packet.write_byte(u8::from(pet.name_tag))?;
    packet.write_byte(u8::from(pet.chat_balloon))?;
    Ok(())
}

/// SPAWN_PET (168): `pet` joins the list at `index`, moving those after it
/// up one.
pub fn build_spawn_pet(owner_id: i32, index: u8, pet: &FieldPet) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnPet as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_byte(1)?;
    p.write_byte(0)?; // insert rather than replace
    write_pet(&mut p, pet)?;
    Ok(p)
}

/// SPAWN_PET (168) for a pet leaving: those after `index` move down one.
pub fn build_remove_pet(owner_id: i32, index: u8, why: PetGone) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnPet as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_byte(0)?;
    p.write_byte(why as u8)?;
    Ok(p)
}

/// MOVE_PET (170): the path its owner's client sent, from `start`.
pub fn build_move_pet(
    owner_id: i32,
    index: u8,
    start: (i16, i16),
    movement: &[u8],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MovePet as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_short(start.0)?;
    p.write_short(start.1)?;
    p.write_bytes(movement)?;
    Ok(p)
}

/// PET_CHAT (171, FUN_0070476e): the pet at `index` acts and speaks as its
/// owner's client showed it. `action` 9 and up are the pet's own actions.
/// A `styled` line shows in the pet's own chat balloon.
pub fn build_pet_chat(
    owner_id: i32,
    index: u8,
    kind: u8,
    action: u8,
    text: &str,
    styled: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PetChat as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_byte(kind)?;
    p.write_byte(action)?;
    p.write_str_with_length(text)?;
    p.write_byte(u8::from(styled))?;
    Ok(p)
}

/// PET_NAMECHANGE (172, FUN_00704801): the pet at `index` is now called
/// `name`, and shows its kind's name tag if `name_tag` (a label ring is
/// worn for it). Its owner's client needs it too, as pet commands may
/// start with the name.
pub fn build_pet_name(
    owner_id: i32,
    index: u8,
    name: &str,
    name_tag: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PetNameChange as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_str_with_length(name)?;
    p.write_byte(u8::from(name_tag))?;
    Ok(p)
}

/// PET_COMMAND (174, FUN_007048ab) mode 0: whether the pet at `index`
/// obeyed `command`. Only its owner's client reacts: it picks the action
/// and line and sends them back as PET_CHAT for the others. A `quoted` pet
/// (one wearing a quote ring) answers in its kind's chat balloon.
pub fn build_pet_command(
    owner_id: i32,
    index: u8,
    command: u8,
    obeyed: bool,
    quoted: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PetCommand as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_byte(0)?;
    p.write_byte(command)?;
    p.write_byte(u8::from(obeyed))?;
    p.write_byte(u8::from(quoted))?;
    Ok(p)
}

/// PET_COMMAND (174) mode 1: whether the pet at `index` enjoyed its food.
/// As with commands, its owner's client picks the reaction and sends it
/// back as PET_CHAT, in its kind's balloon if `quoted`.
pub fn build_pet_food(
    owner_id: i32,
    index: u8,
    enjoyed: bool,
    quoted: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PetCommand as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_byte(1)?;
    p.write_byte(u8::from(enjoyed))?;
    p.write_byte(u8::from(quoted))?;
    Ok(p)
}

/// CASH_PET_FOOD_RESULT (76, FUN_00a29049): the pet at `index` ate a cash
/// food, and its owner's client plays its eating sound. `None` shows "Your
/// pet cannot consume this food. Please check again." Neither unlocks the
/// client.
pub fn build_cash_pet_food(index: Option<u8>) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CashPetFoodResult as i16)?;
    match index {
        Some(index) => {
            p.write_byte(0)?;
            p.write_byte(index)?;
        }
        None => {
            p.write_byte(1)?;
        }
    }
    Ok(p)
}

/// User effect 4 (FUN_009377d9): the pet at `index` levelled up, on its
/// owner (`None`) or another player.
pub fn build_pet_level_up(owner_id: Option<i32>, index: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    match owner_id {
        Some(id) => {
            p.write_short(SendOpcode::ShowForeignEffect as i16)?;
            p.write_int(id)?;
        }
        None => p.write_short(SendOpcode::ShowItemGainInChat as i16)?,
    }
    p.write_byte(4)?;
    p.write_byte(0)?;
    p.write_byte(index)?;
    Ok(p)
}

/// PET_EXCEPTION_LIST (173): the items the pet `serial` will not pick up.
pub fn build_pet_exception_list(
    owner_id: i32,
    index: u8,
    serial: i64,
    items: &[i32],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PetExceptionList as i16)?;
    p.write_int(owner_id)?;
    p.write_byte(index)?;
    p.write_long(serial)?;
    p.write_byte(items.len() as u8)?;
    for &item in items {
        p.write_int(item)?;
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    fn tiger() -> FieldPet {
        FieldPet {
            item_id: 5000066,
            name: "Baby Tiger".into(),
            serial: 77,
            x: -100,
            y: 20,
            stance: 4,
            foothold: 12,
            name_tag: true,
            chat_balloon: false,
        }
    }

    #[test]
    fn a_pet_comes_out() {
        let packet = build_spawn_pet(9, 1, &tiger()).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::SpawnPet as i16)
            .int(9)
            .byte(1)
            .byte(1)
            .byte(0);
        expect.int(5000066).string("Baby Tiger").long(77);
        expect
            .short(-100)
            .short(20)
            .byte(4)
            .short(12)
            .byte(1) // a label ring
            .byte(0);
        expect.end();
    }

    #[test]
    fn a_pet_goes_back() {
        let packet = build_remove_pet(9, 2, PetGone::Hungry).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::SpawnPet as i16)
            .int(9)
            .byte(2)
            .byte(0)
            .byte(1);
        expect.end();
    }

    #[test]
    fn a_pet_moves_and_lists_what_it_ignores() {
        let packet = build_move_pet(9, 0, (5, 6), &[1, 2, 3]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MovePet as i16).int(9).byte(0);
        expect.short(5).short(6).bytes(&[1, 2, 3]);
        expect.end();

        let packet = build_pet_exception_list(9, 0, 77, &[]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PetExceptionList as i16)
            .int(9)
            .byte(0)
            .long(77)
            .byte(0);
        expect.end();
    }

    #[test]
    fn a_pet_obeys_and_speaks() {
        let packet = build_pet_command(9, 1, 28, true, true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PetCommand as i16).int(9).byte(1);
        expect.byte(0).byte(28).byte(1).byte(1);
        expect.end();

        let packet = build_pet_chat(9, 1, 0, 12, "Ahh", true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PetChat as i16).int(9).byte(1);
        expect.byte(0).byte(12).string("Ahh").byte(1);
        expect.end();
    }

    #[test]
    fn a_pet_levels_up() {
        let packet = build_pet_level_up(None, 2).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowItemGainInChat as i16);
        expect.byte(4).byte(0).byte(2);
        expect.end();

        let packet = build_pet_level_up(Some(9), 2).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowForeignEffect as i16).int(9);
        expect.byte(4).byte(0).byte(2);
        expect.end();
    }

    #[test]
    fn a_pet_is_renamed() {
        let packet = build_pet_name(9, 0, "Tigger", true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PetNameChange as i16)
            .int(9)
            .byte(0);
        expect.string("Tigger").byte(1);
        expect.end();
    }
}
