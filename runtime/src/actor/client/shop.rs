use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::{inventory, shop, stat};
use net::packet::handle::world::{take_miu_miu, MU_MU};
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

    /// The Miu Miu in Cash `slot`: Mu Mu's shop opens and one Miu Miu
    /// goes, but not while the player trades or keeps a shop.
    pub(super) async fn miu_miu(&mut self, slot: i16) -> Result<(), RuntimeError> {
        // As the client refuses while any dialog is open.
        let busy = self.trade.is_some()
            || self.personal_shop.is_some()
            || self.mini_game.is_some()
            || self.conversation.is_some()
            || self.script.is_some()
            || self.shop.is_some()
            || self.storage.is_some()
            || self.duey.is_some();
        let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
        let opened = match busy {
            true => None,
            false => {
                Shop::open(MU_MU, player).and_then(|shop| Some((shop, take_miu_miu(player, slot)?)))
            }
        };
        let Some((shop, changes)) = opened else {
            let mut unlock = stat::build_stat_update(true, &[])
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            return self.send(&mut unlock).await;
        };
        let mut packets = vec![
            inventory::build_inventory_operation(true, &changes),
            shop::build_open_shop(&shop),
        ];
        for packet in &mut packets {
            let packet = packet
                .as_mut()
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            self.send(packet).await?;
        }
        self.shop = Some(shop);
        Ok(())
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
