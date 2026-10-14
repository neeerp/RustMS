use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build;
use packet::Packet;

/// 0x28, `ENTER_CASHSHOP`: the SHOP button or the Cash Shop key. The client
/// leaves again with an empty `CHANGE_MAP`.
pub struct EnterCashShopHandler;

impl WorldHandler for EnterCashShopHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if ctx.player.stats.hp <= 0 {
            let enable_actions = build::world::stat::build_stat_update(true, &[])?;
            return Ok(HandlerResult::reply(enable_actions));
        }
        Ok(HandlerResult::empty().with(WorldAction::EnterCashShop))
    }
}
