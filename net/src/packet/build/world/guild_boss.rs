//! Ergoth's room in Sharenian (990000900, field type 8, report b380): the
//! chandelier hanging over the altar and the pulley that drops it.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// The chandelier (the map's `healer`) hangs at height `y`; the client
/// moves it there at once (v83 `CField_GuildBoss::OnPacket`, 0x558b0f).
pub fn build_guild_boss_healer_move(y: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GuildBossHealerMove as i16)?;
    p.write_short(y)?;
    Ok(p)
}

/// The pulley's state: 0 when it may be pulled, anything else while the
/// chandelier is away, which keeps the client from asking again.
pub fn build_guild_boss_pulley_state(state: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GuildBossPulleyState as i16)?;
    p.write_byte(state)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn the_chandelier_moves_and_the_pulley_locks() {
        let packet = build_guild_boss_healer_move(-420).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x11F).short(-420);
        expect.end();
        let packet = build_guild_boss_pulley_state(1).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x120).byte(1);
        expect.end();
    }
}
