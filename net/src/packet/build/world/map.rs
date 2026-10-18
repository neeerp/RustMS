use crate::codec::time::filetime;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{CharacterStats, TeleportRocks};
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

/// SET_FIELD for a map change: the client warps to `stats.map_id` at portal
/// `stats.spawn_point`, or at `chase` if given (v83 FUN_00776020 reads it
/// after the HP).
pub fn build_warp_to_map(
    stats: &CharacterStats,
    channel_id: u8,
    chase: Option<(i16, i16)>,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetField as i16)?;
    packet.write_int(i32::from(channel_id))?;
    packet.write_int(0)?;
    packet.write_byte(0)?;
    packet.write_int(stats.map_id)?;
    packet.write_byte(stats.spawn_point)?;
    packet.write_short(stats.hp)?;
    match chase {
        Some((x, y)) => {
            packet.write_byte(1)?;
            packet.write_int(i32::from(x))?;
            packet.write_int(i32::from(y))?;
        }
        None => {
            packet.write_byte(0)?;
        }
    }
    packet.write_long(filetime(SystemTime::now()))?;
    Ok(packet)
}

/// MAP_TRANSFER_RESULT (0x2A, v83 FUN_00a25268) after a teleport rock list
/// changed: the whole list, empty slots and all, which the client keeps and
/// shows in an open rock window.
pub fn build_rock_list(
    deleted: bool,
    vip: bool,
    rocks: &TeleportRocks,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MapTransferResult as i16)?;
    packet.write_byte(if deleted { 2 } else { 3 })?;
    packet.write_byte(u8::from(vip))?;
    for map_id in rocks.slots(vip) {
        packet.write_int(map_id)?;
    }
    Ok(packet)
}

/// Why a teleport rock request was refused: MAP_TRANSFER_RESULT's notices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RockRefusal {
    /// "%s is currently difficult to locate, so the teleport will not take
    /// place.", with the name the player typed.
    NotFound = 6,
    /// "You cannot go to that place."
    CannotGo = 8,
    /// "It's the map you're currently on."
    SameMap = 9,
    /// "This map is not available to enter for the list."
    NotForList = 10,
    /// "Users below level 7 are not allowed to go out from Maple Island."
    MapleIsland = 11,
}

pub fn build_rock_refusal(why: RockRefusal, vip: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MapTransferResult as i16)?;
    packet.write_byte(why as u8)?;
    packet.write_byte(u8::from(vip))?;
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
    fn a_rock_list_fills_every_slot() {
        let mut rocks = TeleportRocks::default();
        rocks.keep(false, 100_000_000);
        let packet = build_rock_list(false, false, &rocks).unwrap();
        let mut expected = vec![0x2A, 0, 3, 0];
        expected.extend(100_000_000i32.to_le_bytes());
        for _ in 0..4 {
            expected.extend(999_999_999i32.to_le_bytes());
        }
        assert_eq!(packet.bytes, expected);
        let packet = build_rock_list(true, true, &rocks).unwrap();
        assert_eq!(packet.bytes[..4], [0x2A, 0, 2, 1]);
        assert_eq!(packet.bytes.len(), 4 + 10 * 4);
        assert_eq!(
            build_rock_refusal(RockRefusal::SameMap, true)
                .unwrap()
                .bytes,
            [0x2A, 0, 9, 1]
        );
    }

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
        let packet = build_warp_to_map(&sample_stats(), 1, None).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SetField as i16);
        expect.int(1); // zero-based channel
        expect.int(0).byte(0); // "change map" rather than "enter world"
        expect.int(100000000).byte(2).short(300);
        expect.byte(0); // no explicit spawn position
        expect.skip(8); // FILETIME now
        expect.end();
    }

    #[test]
    fn a_warp_may_place_the_player_anywhere() {
        let packet = build_warp_to_map(&sample_stats(), 1, Some((-120, 275))).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::SetField as i16)
            .int(1)
            .int(0)
            .byte(0);
        expect.int(100000000).byte(2).short(300);
        expect.byte(1).int(-120).int(275);
        expect.skip(8);
        expect.end();
    }
}
