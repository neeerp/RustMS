//! Server policy switches that are not part of the game rules themselves.

use game::Player;
use std::sync::OnceLock;

/// `RUSTMS_ALLOW_DEBUG_WARPS=1` lets any player warp by map id. Only meant
/// for local development and the integration harness.
pub fn debug_warps_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED
        .get_or_init(|| std::env::var("RUSTMS_ALLOW_DEBUG_WARPS").is_ok_and(|value| value == "1"))
}

/// The GM level from which the client is told the account is a GM's (row
/// 205): it then sends the GM's '/' lines, and refuses trades and stores.
pub const CLIENT_GM_LEVEL: i16 = 3;

/// Whether an account of `gm_level` logs in with the client's GM grade.
pub fn client_gm(gm_level: i16) -> bool {
    gm_level >= CLIENT_GM_LEVEL
}

/// The GM level from which a GM may send parcels by Duey. Below it a GM
/// can't, so that the items and mesos GMs make stay with them (Cosmic
/// `MINIMUM_GM_LEVEL_TO_USE_DUEY`).
pub const PARCEL_GM_LEVEL: i16 = 3;

/// Whether an account of `gm_level` may send parcels: a player's or an
/// admin's.
pub fn may_send_parcels(gm_level: i16) -> bool {
    gm_level <= 0 || gm_level >= PARCEL_GM_LEVEL
}

/// Chat commands (`net::gm_commands`) are on unless
/// `RUSTMS_GM_COMMANDS=0`.
pub fn gm_commands_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("RUSTMS_GM_COMMANDS").map_or(true, |value| value != "0"))
}

/// Whether `player` may warp straight to an arbitrary map.
pub fn may_warp_anywhere(player: &Player) -> bool {
    player.gm_level > 0 || debug_warps_enabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn players_and_admins_send_parcels_and_lesser_gms_dont() {
        assert!(may_send_parcels(0));
        assert!(!may_send_parcels(1));
        assert!(!may_send_parcels(2));
        assert!(may_send_parcels(3));
    }
}
