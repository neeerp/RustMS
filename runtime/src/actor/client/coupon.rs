use super::{expiry, ClientActor};
use crate::{db::spawn_db, error::RuntimeError};
use db::cash_shop::NewCashItem;
use db::coupon::Redemption;
use net::coupon::{Grant, STRIKES};
use net::packet::build::world::cash_shop::{self as cs, CashShopFailure};
use packet::Packet;
use std::time::SystemTime;
use tracing::{info, warn};

impl ClientActor {
    /// Redeem the coupon the player typed in the Cash Shop. Every answer
    /// ends with the balances, which is what lets the client take another
    /// request.
    pub(super) async fn redeem_coupon(
        &mut self,
        code: String,
    ) -> Result<Vec<Packet>, RuntimeError> {
        let account_id = self.in_game()?.player.account_id;
        let found = match net::coupon::normalize(&code) {
            Some(code) => spawn_db(move || db::coupon::find(&code)).await?,
            None => None,
        };
        let mut packets = match found {
            None => {
                self.coupon_strikes += 1;
                let failure = if self.coupon_strikes >= STRIKES {
                    self.coupon_strikes = 0;
                    CashShopFailure::CouponStrikes
                } else {
                    CashShopFailure::CouponNotFound
                };
                vec![cs::build_cash_shop_failure(failure)?]
            }
            Some(coupon) if coupon.redeemed => {
                vec![cs::build_cash_shop_failure(CashShopFailure::CouponUsed)?]
            }
            Some(coupon) if coupon.expired(SystemTime::now()) => {
                vec![cs::build_cash_shop_failure(CashShopFailure::CouponExpired)?]
            }
            Some(coupon) => self.redeem(account_id, coupon).await?,
        };
        let balance = spawn_db(move || db::cash_shop::balance(account_id)).await?;
        packets.push(cs::build_cash_balance(
            balance.credit,
            balance.points,
            balance.prepaid,
        )?);
        Ok(packets)
    }

    async fn redeem(
        &mut self,
        account_id: i32,
        coupon: db::coupon::Coupon,
    ) -> Result<Vec<Packet>, RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let grant = match net::coupon::grant(player, &coupon.rewards)? {
            Ok(grant) => grant,
            Err(failure) => return Ok(vec![cs::build_cash_shop_failure(failure)?]),
        };
        let Grant {
            cash_items,
            items,
            mesos,
            add,
            packets,
        } = grant;
        let new_items: Vec<_> = cash_items
            .iter()
            .map(|&(item_id, sn, quantity)| NewCashItem {
                account_id,
                item_id,
                sn,
                quantity,
                expires_at: expiry(
                    item_id,
                    net::cash_shop::commodity(sn).map_or(0, |commodity| commodity.period),
                ),
            })
            .collect();
        let snapshot = player.clone();
        let coupon_id = coupon.id;
        let redeemed = spawn_db(move || {
            db::coupon::redeem(coupon_id, SystemTime::now(), add, &new_items, &snapshot)
        })
        .await;
        let failure = match redeemed {
            Ok(Redemption::Redeemed {
                items: cash_items, ..
            }) => {
                self.in_game_mut()?.player.mark_saved();
                self.coupon_strikes = 0;
                info!(self.client_id, coupon_id, "Redeemed a coupon");
                let mut packets = packets;
                packets.push(cs::build_coupon_redeemed(
                    &cash_items,
                    add.points,
                    &items,
                    mesos,
                )?);
                return Ok(packets);
            }
            Ok(Redemption::Used) => CashShopFailure::CouponUsed,
            Ok(Redemption::Expired) => CashShopFailure::CouponExpired,
            Err(error) => {
                warn!(self.client_id, coupon_id, %error, "Couldn't redeem a coupon");
                CashShopFailure::Unknown
            }
        };
        self.in_game_mut()?.player = before;
        Ok(vec![cs::build_cash_shop_failure(failure)?])
    }
}
