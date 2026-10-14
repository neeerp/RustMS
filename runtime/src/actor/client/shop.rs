use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::{shop, stat};
use net::shop::{Shop, ShopRequest};

impl ClientActor {
    /// Open `npc_id`'s shop if it keeps one; the shop stays open until the
    /// player leaves it, talks to another NPC, or changes maps.
    pub(super) async fn open_shop(&mut self, npc_id: i32) -> Result<bool, RuntimeError> {
        let Some(shop) = Shop::open(npc_id, &self.in_game()?.player) else {
            return Ok(false);
        };
        let mut packet =
            shop::build_open_shop(&shop).map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut packet).await?;
        self.shop = Some(shop);
        Ok(true)
    }

    pub(super) async fn shop_action(&mut self, request: ShopRequest) -> Result<(), RuntimeError> {
        if request == ShopRequest::Leave {
            self.shop = None;
            return Ok(());
        }
        let packets = match self.shop.as_ref() {
            Some(open) => {
                let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
                net::shop::respond(open, player, request)
            }
            None => stat::build_stat_update(true, &[]).map(|packet| vec![packet]),
        }
        .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        for mut packet in packets {
            self.send(&mut packet).await?;
        }
        Ok(())
    }
}
