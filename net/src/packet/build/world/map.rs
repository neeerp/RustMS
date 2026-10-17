use crate::codec::time::filetime;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::CharacterStats;
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

/// SET_FIELD for a map change: the client warps to `stats.map_id` at portal
/// `stats.spawn_point`.
pub fn build_warp_to_map(stats: &CharacterStats, channel_id: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetField as i16)?;
    packet.write_int(i32::from(channel_id))?;
    packet.write_int(0)?;
    packet.write_byte(0)?;
    packet.write_int(stats.map_id)?;
    packet.write_byte(stats.spawn_point)?;
    packet.write_short(stats.hp)?;
    packet.write_byte(0)?; // no explicit spawn position follows
    packet.write_long(filetime(SystemTime::now()))?;
    Ok(packet)
}

/// Why a map change was refused (v83 FUN_0053185c's chat lines and notices).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockedMap {
    /// "The portal is closed for now."
    PortalClosed = 1,
    /// "You cannot go to that place."
    CannotGo = 2,
    /// "Unable to approach due to the force of the ground."
    ForceOfTheGround = 3,
    /// A notice: "You cannot teleport to or on this map."
    NoTeleport = 4,
    /// "This map can only be entered by party members."
    PartyOnly = 6,
    /// A notice: "The cash shop is currently not available."
    CashShopUnavailable = 7,
}

/// Why a channel, cash shop or trade shop move was refused (v83
/// FUN_00531a08's chat lines).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockedServer {
    /// "You cannot move that channel. Please try again later."
    Channel = 1,
    /// "You cannot go into the cash shop. Please try again later."
    CashShop = 2,
    /// "The Item-Trading Shop is currently unavailable."
    TradeShop = 3,
    /// "You cannot go into the trade shop, due to the limitation of user count."
    TradeShopFull = 4,
    /// "You do not meet the minimum level requirement to access the Trade Shop."
    TradeShopLevel = 5,
}

/// BLOCKED_MAP (0x83): the map change is refused and the client may ask
/// again.
pub fn build_blocked_map(reason: BlockedMap) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::BlockedMap as i16)?;
    p.write_byte(reason as u8)?;
    Ok(p)
}

/// BLOCKED_SERVER (0x84): the move is refused and the client may ask again.
pub fn build_blocked_server(reason: BlockedServer) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::BlockedServer as i16)?;
    p.write_byte(reason as u8)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_stats;
    use crate::codec::testing::Expect;

    #[test]
    fn a_refusal_names_its_reason() {
        assert_eq!(
            build_blocked_map(BlockedMap::PortalClosed).unwrap().bytes,
            [0x83, 0, 1]
        );
        assert_eq!(
            build_blocked_server(BlockedServer::Channel).unwrap().bytes,
            [0x84, 0, 1]
        );
    }

    #[test]
    fn warp_to_map_matches_reference() {
        let packet = build_warp_to_map(&sample_stats(), 1).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SetField as i16);
        expect.int(1); // zero-based channel
        expect.int(0).byte(0); // "change map" rather than "enter world"
        expect.int(100000000).byte(2).short(300);
        expect.byte(0); // no explicit spawn position
        expect.skip(8); // FILETIME now
        expect.end();
    }
}
