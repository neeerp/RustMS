//! MAKER_RESULT (recv 217, v83 FUN_0095dad3): what the Item Maker made, and
//! the effect it plays (report c006 §2). The window waits for an answer to
//! every ITEM_MAKE, refusals included.

use crate::{error::NetworkError, packet::op::SendOpcode};
use game::InventoryType;
use packet::{io::write::PktWrite, Packet};

/// Why the Maker made nothing. The client reads nothing after the code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MakerRefusal {
    /// "Production failed due to an unknown error."
    Failed,
    /// "You don't have enough room in your Inventory.", naming the tab.
    NoRoom(InventoryType),
}

/// What a make from a recipe cost: the ingredients (each id and count),
/// the gems and the stimulator put in, and the mesos.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MakerCost {
    pub lost: Vec<(i32, i32)>,
    pub gems: Vec<i32>,
    pub stimulator: Option<i32>,
    pub meso: i32,
}

/// Made `count` of `item_id` from a recipe, or with `None` the stimulator
/// destroyed it ("The item has been destroyed."). Either way the cost is
/// gone.
pub fn build_maker_made(
    made: Option<(i32, i32)>,
    cost: &MakerCost,
) -> Result<Packet, NetworkError> {
    let mut p = result(i32::from(made.is_none()))?;
    p.write_int(1)?;
    p.write_byte(u8::from(made.is_none()))?;
    if let Some((item_id, count)) = made {
        p.write_int(item_id)?;
        p.write_int(count)?;
    }
    p.write_int(cost.lost.len() as i32)?;
    for &(id, count) in &cost.lost {
        p.write_int(id)?;
        p.write_int(count)?;
    }
    p.write_int(cost.gems.len() as i32)?;
    for &gem in &cost.gems {
        p.write_int(gem)?;
    }
    p.write_byte(u8::from(cost.stimulator.is_some()))?;
    if let Some(stimulator) = cost.stimulator {
        p.write_int(stimulator)?;
    }
    p.write_int(cost.meso)?;
    Ok(p)
}

/// Made the monster crystal `crystal` from 100 of `from`.
pub fn build_maker_crystal(crystal: i32, from: i32) -> Result<Packet, NetworkError> {
    let mut p = result(0)?;
    p.write_int(3)?;
    p.write_int(crystal)?;
    p.write_int(from)?;
    Ok(p)
}

/// Took `item_id` apart into `gained` (each crystal and count) for `meso`.
pub fn build_maker_disassembled(
    item_id: i32,
    gained: &[(i32, i32)],
    meso: i32,
) -> Result<Packet, NetworkError> {
    let mut p = result(0)?;
    p.write_int(4)?;
    p.write_int(item_id)?;
    p.write_int(gained.len() as i32)?;
    for &(id, count) in gained {
        p.write_int(id)?;
        p.write_int(count)?;
    }
    p.write_int(meso)?;
    Ok(p)
}

pub fn build_maker_refusal(why: MakerRefusal) -> Result<Packet, NetworkError> {
    result(match why {
        MakerRefusal::Failed => 2,
        // 4 to 7 for the Equip to the Etc tab.
        MakerRefusal::NoRoom(kind) => 3 + kind as i32,
    })
}

/// SHOW_ITEM_GAIN_INCHAT mode 0x10: the Maker's success or failure glow
/// and sound on the maker.
pub fn build_maker_effect(success: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(0x10)?;
    p.write_int(i32::from(!success))?;
    Ok(p)
}

fn result(code: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MakerResult as i16)?;
    p.write_int(code)?;
    Ok(p)
}
