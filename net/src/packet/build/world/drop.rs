use crate::codec::time::expiration;
use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// How a drop appears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropAppearance {
    /// Falls from the mob that died (`dropfrom` → `dropto`), `delay` ms
    /// from now (the client hides it until then).
    Falling { from: (i16, i16), delay: i16 },
    /// Already on the ground (shown to someone entering the map).
    Lying,
    /// Thrown from `from`, then fades away instead of landing (an item that
    /// can't be dropped).
    Vanishing { from: (i16, i16) },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DropView {
    pub object_id: i32,
    /// An item id, or the meso amount.
    pub item_or_meso: i32,
    pub meso: bool,
    /// Who may pick it up first.
    pub owner: i32,
    /// 0: owner first for a while, 2: anyone.
    pub ownership: u8,
    pub x: i16,
    pub y: i16,
    pub dropper: i32,
    /// A player threw it away; pets leave it alone.
    pub player_drop: bool,
}

/// DROP_ITEM_FROM_MAPOBJECT (`dropItemFromMapObject`).
pub fn build_drop(drop: &DropView, appearance: DropAppearance) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::DropItemFromMapObject as i16)?;
    p.write_byte(match appearance {
        DropAppearance::Falling { .. } => 1,
        DropAppearance::Lying => 2,
        DropAppearance::Vanishing { .. } => 3,
    })?;
    p.write_int(drop.object_id)?;
    p.write_byte(drop.meso as u8)?;
    p.write_int(drop.item_or_meso)?;
    p.write_int(drop.owner)?;
    p.write_byte(drop.ownership)?;
    p.write_short(drop.x)?;
    p.write_short(drop.y)?;
    p.write_int(drop.dropper)?;
    let falls = match appearance {
        DropAppearance::Falling { from, delay } => Some((from, delay)),
        DropAppearance::Vanishing { from } => Some((from, 0)),
        DropAppearance::Lying => None,
    };
    if let Some((from, delay)) = falls {
        p.write_short(from.0)?;
        p.write_short(from.1)?;
        p.write_short(delay)?;
    }
    if !drop.meso {
        p.write_long(expiration(None))?;
    }
    p.write_byte(u8::from(!drop.player_drop))?;
    Ok(p)
}

/// REMOVE_ITEM_FROM_MAP mode 5: picked up by the pet at `pet` in the list of
/// the player `owner`, to whom it flies (v83 FUN_00506590).
pub fn build_pet_loot_drop(object_id: i32, owner: i32, pet: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveItemFromMap as i16)?;
    p.write_byte(5)?;
    p.write_int(object_id)?;
    p.write_int(owner)?;
    p.write_byte(pet)?;
    Ok(p)
}

/// REMOVE_ITEM_FROM_MAP: 0 expired, 2 picked up by `looter`.
pub fn build_remove_drop(object_id: i32, looter: Option<i32>) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveItemFromMap as i16)?;
    match looter {
        Some(character_id) => {
            p.write_byte(2)?;
            p.write_int(object_id)?;
            p.write_int(character_id)?;
        }
        None => {
            p.write_byte(0)?;
            p.write_int(object_id)?;
        }
    }
    Ok(p)
}

/// REMOVE_ITEM_FROM_MAP mode 4: Meso Explosion set it off, and it goes
/// off `delay` ms from now (v83 FUN_00506590, 0x5065e6).
pub fn build_explode_drop(object_id: i32, delay: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveItemFromMap as i16)?;
    p.write_byte(4)?;
    p.write_int(object_id)?;
    p.write_short(delay)?;
    Ok(p)
}

/// SHOW_STATUS_INFO 0/1: "You have gained mesos (+n)".
pub fn build_meso_gain(amount: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(0)?;
    p.write_short(1)?;
    p.write_int(amount)?;
    p.write_short(0)?;
    Ok(p)
}

/// SHOW_STATUS_INFO 5: "You have gained mesos (+n)" or "You have lost mesos
/// (-n)" in the chat log, as for quest rewards.
pub fn build_meso_gain_in_chat(amount: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(5)?;
    p.write_int(amount)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    fn shell() -> DropView {
        DropView {
            object_id: 3_000_000_000u32 as i32,
            item_or_meso: 4000019,
            meso: false,
            owner: 7,
            ownership: 0,
            x: 150,
            y: -25,
            dropper: 2_000_000_001,
            player_drop: false,
        }
    }

    #[test]
    fn falling_item_layout() {
        let falling = DropAppearance::Falling {
            from: (140, -25),
            delay: 300,
        };
        let packet = build_drop(&shell(), falling).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::DropItemFromMapObject as i16)
            .byte(1);
        expect
            .int(shell().object_id)
            .byte(0)
            .int(4000019)
            .int(7)
            .byte(0);
        expect.short(150).short(-25).int(2_000_000_001);
        expect.short(140).short(-25).short(300);
        expect.long(expiration(None)).byte(1);
        expect.end();
    }

    #[test]
    fn lying_meso_layout() {
        let mesos = DropView {
            item_or_meso: 5,
            meso: true,
            ownership: 2,
            ..shell()
        };
        let packet = build_drop(&mesos, DropAppearance::Lying).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::DropItemFromMapObject as i16)
            .byte(2);
        expect.int(mesos.object_id).byte(1).int(5).int(7).byte(2);
        expect.short(150).short(-25).int(2_000_000_001).byte(1);
        expect.end();
    }

    #[test]
    fn vanishing_player_drop_layout() {
        let thrown = DropView {
            ownership: 2,
            dropper: 0,
            player_drop: true,
            ..shell()
        };
        let packet = build_drop(&thrown, DropAppearance::Vanishing { from: (150, -25) }).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::DropItemFromMapObject as i16)
            .byte(3);
        expect
            .int(thrown.object_id)
            .byte(0)
            .int(4000019)
            .int(7)
            .byte(2);
        expect.short(150).short(-25).int(0);
        expect.short(150).short(-25).short(0);
        expect.long(expiration(None)).byte(0);
        expect.end();
    }

    #[test]
    fn removal_and_meso_gain() {
        let picked = build_remove_drop(9, Some(7)).unwrap();
        let mut expect = Expect::new(&picked.bytes);
        expect
            .short(SendOpcode::RemoveItemFromMap as i16)
            .byte(2)
            .int(9)
            .int(7);
        expect.end();
        let expired = build_remove_drop(9, None).unwrap();
        let mut expect = Expect::new(&expired.bytes);
        expect
            .short(SendOpcode::RemoveItemFromMap as i16)
            .byte(0)
            .int(9);
        expect.end();
        let gain = build_meso_gain(5).unwrap();
        let mut expect = Expect::new(&gain.bytes);
        expect
            .short(SendOpcode::ShowStatusInfo as i16)
            .byte(0)
            .short(1)
            .int(5)
            .short(0);
        expect.end();
        let spent = build_meso_gain_in_chat(-150).unwrap();
        let mut expect = Expect::new(&spent.bytes);
        expect
            .short(SendOpcode::ShowStatusInfo as i16)
            .byte(5)
            .int(-150);
        expect.end();
    }
}
