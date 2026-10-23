use super::{expiry, ClientActor};
use crate::{db::spawn_db, error::RuntimeError};
use db::cash_shop::NewCashItem;
use db::name_change::{Applied, NewRequest, Requested, Standing};
use game::InventoryType;
use game_data::commodities::Commodity;
use net::packet::build::world::cash_shop::{self as cs, CashShopFailure};
use net::packet::build::world::inventory::build_inventory_operation;
use net::packet::build::world::name_change::{
    build_name_change_cancelled, build_name_change_rules, build_new_name_check,
    build_no_name_change, NameChangeRules,
};
use net::packet::build::world::stat::build_stat_update;
use packet::Packet;
use std::time::SystemTime;
use tracing::{info, warn};

impl ClientActor {
    /// Op 0x10 in the Cash Shop: whether the character may ask for a new
    /// name. The handler checked the birthday code and the level.
    pub(super) async fn name_change_rules(&mut self) -> Result<Vec<Packet>, RuntimeError> {
        let character_id = self.in_game()?.player.stats.id;
        let standing =
            spawn_db(move || db::name_change::standing(character_id, SystemTime::now())).await?;
        let rules = match standing {
            Standing::Free => NameChangeRules::Allowed,
            Standing::Pending => NameChangeRules::Pending,
            Standing::Cooldown => NameChangeRules::Cooldown,
        };
        Ok(vec![build_name_change_rules(rules)?])
    }

    /// Op 0x15 in the Cash Shop: whether no one has `name`, which the
    /// handler checked against the name rules.
    pub(super) async fn check_new_name(
        &mut self,
        name: String,
    ) -> Result<Vec<Packet>, RuntimeError> {
        let checked = name.clone();
        let claimed = spawn_db(move || db::name_change::claimed(&checked)).await?;
        Ok(vec![build_new_name_check(&name, !claimed)?])
    }

    /// CASHSHOP_OPERATION 0x2E: pay for Name Change from the NX Prepaid (the
    /// only balance v83 offers for it) and record the request; the receipt
    /// goes to the Cash Inventory.
    pub(super) async fn request_name_change(
        &mut self,
        commodity: Commodity,
        new_name: String,
    ) -> Result<Vec<Packet>, RuntimeError> {
        let player = &self.in_game()?.player;
        let (account_id, character_id) = (player.account_id, player.stats.id);
        let old_name = player.stats.name.clone();
        let receipt = NewCashItem {
            account_id,
            item_id: commodity.item_id,
            sn: commodity.sn,
            quantity: commodity.count,
            expires_at: expiry(commodity.item_id, commodity.period),
        };
        let (sn, price) = (commodity.sn, commodity.price);
        let name = new_name.clone();
        let requested = spawn_db(move || {
            db::name_change::request(
                &NewRequest {
                    account_id,
                    character_id,
                    sn,
                    price,
                    receipt,
                    old_name: &old_name,
                    new_name: &name,
                },
                SystemTime::now(),
            )
        })
        .await?;
        Ok(match requested {
            Requested::Done(receipt, balance) => {
                info!(
                    self.client_id,
                    character_id, new_name, price, "Bought a Name Change"
                );
                vec![
                    cs::build_bought_name_change(&receipt)?,
                    cs::build_cash_balance(balance.credit, balance.points, balance.prepaid)?,
                ]
            }
            Requested::Short => vec![cs::build_cash_shop_failure(CashShopFailure::NotEnoughCash)?],
            Requested::Refused => vec![cs::build_cash_shop_failure(CashShopFailure::Unknown)?],
        })
    }

    /// The receipt in Cash `slot` was used: drop the character's request it
    /// stands for, with no refund (the dialog says so), and the receipt with
    /// it. Another character's receipt cancels nothing.
    pub(super) async fn cancel_name_change(&mut self, slot: i16) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let character_id = player.stats.id;
        let cash_id = player
            .inventory
            .get(InventoryType::Cash, slot)
            .and_then(|item| item.cash_serial);
        let cancelled = match cash_id {
            Some(cash_id) => {
                spawn_db(move || db::name_change::cancel(character_id, cash_id)).await?
            }
            None => false,
        };
        if !cancelled {
            return self.send(&mut build_no_name_change()?).await;
        }
        info!(self.client_id, character_id, "Cancelled a Name Change");
        let taken = self
            .in_game_mut()?
            .player
            .inventory_mut()
            .take(InventoryType::Cash, slot, 1);
        let mut packets = match taken {
            Ok(changes) => vec![
                build_inventory_operation(true, &changes)?,
                build_name_change_cancelled(true)?,
            ],
            Err(_) => vec![
                build_stat_update(true, &[])?,
                build_name_change_cancelled(true)?,
            ],
        };
        self.send_all(&mut packets).await
    }

    /// At logout, once the player is saved: make their waiting Name Change.
    pub(super) async fn apply_name_change(&mut self) {
        let Ok(game) = self.in_game() else {
            return;
        };
        let character_id = game.player.stats.id;
        match spawn_db(move || db::name_change::apply(character_id, SystemTime::now())).await {
            Ok(Some(applied)) => log_applied(character_id, &applied),
            Ok(None) => {}
            Err(error) => warn!(self.client_id, %error, "Failed to apply a Name Change"),
        }
    }
}

pub(crate) fn log_applied(character_id: i32, applied: &Applied) {
    match applied {
        Applied::Renamed { old_name, new_name } => {
            info!(
                character_id,
                old_name, new_name, "Changed a character's name"
            )
        }
        Applied::Refunded { new_name } => {
            info!(
                character_id,
                new_name, "Name Change refunded: the name was taken"
            )
        }
    }
}
