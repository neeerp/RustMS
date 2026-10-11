//! Server policy switches that are not part of the game rules themselves.

use game::Player;
use std::sync::OnceLock;

/// `RUSTMS_ALLOW_DEBUG_WARPS=1` lets any player warp by map id. Only meant
/// for local development and the integration harness.
pub fn debug_warps_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("RUSTMS_ALLOW_DEBUG_WARPS").is_ok_and(|value| value == "1")
    })
}

/// Whether `player` may warp straight to an arbitrary map.
pub fn may_warp_anywhere(player: &Player) -> bool {
    player.gm_level > 0 || debug_warps_enabled()
}
