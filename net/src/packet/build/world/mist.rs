use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// What made a mist, its second field (OdinMS-family values; v83's client
/// only tells type 3 apart, and draws by the skill).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MistKind {
    Mob = 0,
    Poison = 1,
    Smoke = 2,
}

/// A mist (an affected area) as players see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MistView {
    pub object_id: i32,
    pub kind: MistKind,
    pub owner_id: i32,
    pub skill_id: i32,
    pub level: u8,
    /// How long after the packet it starts, in 100 ms.
    pub delay: i16,
    /// Left, top, right, bottom.
    pub area: (i32, i32, i32, i32),
}

/// AFFECTED_AREA_CREATED (0x111, v83 FUN_00431a63): the client draws the
/// skill's `tile` clouds over the area until `time` runs out or the mist is
/// removed.
pub fn build_spawn_mist(mist: &MistView) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMist as i16)?;
    p.write_int(mist.object_id)?;
    p.write_int(mist.kind as i32)?;
    p.write_int(mist.owner_id)?;
    p.write_int(mist.skill_id)?;
    p.write_byte(mist.level)?;
    p.write_short(mist.delay)?;
    let (left, top, right, bottom) = mist.area;
    p.write_int(left)?;
    p.write_int(top)?;
    p.write_int(right)?;
    p.write_int(bottom)?;
    p.write_int(0)?;
    Ok(p)
}

/// AFFECTED_AREA_REMOVED (0x112, v83 FUN_0043234d): the clouds fade out.
pub fn build_remove_mist(object_id: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveMist as i16)?;
    p.write_int(object_id)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mist_names_its_skill_and_area() {
        let mist = MistView {
            object_id: 1_100_000_000,
            kind: MistKind::Poison,
            owner_id: 7,
            skill_id: 2111003,
            level: 30,
            delay: 8,
            area: (-200, -150, 200, 150),
        };
        let p = build_spawn_mist(&mist).unwrap();
        let mut want = vec![0x11, 0x01];
        for v in [1_100_000_000, 1, 7, 2111003] {
            want.extend_from_slice(&i32::to_le_bytes(v));
        }
        want.push(30);
        want.extend_from_slice(&8i16.to_le_bytes());
        for v in [-200, -150, 200, 150, 0] {
            want.extend_from_slice(&i32::to_le_bytes(v));
        }
        assert_eq!(p.bytes, want);
        assert_eq!(
            build_remove_mist(5).unwrap().bytes,
            [0x12, 0x01, 5, 0, 0, 0]
        );
    }
}
