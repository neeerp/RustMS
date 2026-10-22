//! Cash Shop coupons (report b072): codes an account redeems once, in the
//! shop's Coupon dialog, for items, NX and mesos.

use crate::cash_shop::{Balance, CashItem, NewCashItem};
use crate::player::write_player;
use crate::schema::{accounts, cash_items, coupon_rewards, coupons};
use diesel::prelude::*;
use game::Player;
use std::time::SystemTime;

/// What a coupon gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reward {
    Item { item_id: i32, quantity: i32 },
    Credit(i32),
    MaplePoints(i32),
    Prepaid(i32),
    Mesos(i32),
}

impl Reward {
    fn row(self) -> (i16, i32, i32) {
        match self {
            Reward::Item { item_id, quantity } => (0, item_id, quantity),
            Reward::Credit(amount) => (1, 0, amount),
            Reward::MaplePoints(amount) => (2, 0, amount),
            Reward::Prepaid(amount) => (3, 0, amount),
            Reward::Mesos(amount) => (4, 0, amount),
        }
    }

    fn from_row((kind, item_id, quantity): (i16, i32, i32)) -> Option<Self> {
        Some(match kind {
            0 => Reward::Item { item_id, quantity },
            1 => Reward::Credit(quantity),
            2 => Reward::MaplePoints(quantity),
            3 => Reward::Prepaid(quantity),
            4 => Reward::Mesos(quantity),
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coupon {
    pub id: i32,
    pub expires_at: Option<SystemTime>,
    pub redeemed: bool,
    pub rewards: Vec<Reward>,
}

impl Coupon {
    pub fn expired(&self, now: SystemTime) -> bool {
        self.expires_at.is_some_and(|at| at <= now)
    }
}

/// How a redemption went.
#[derive(Debug, PartialEq, Eq)]
pub enum Redemption {
    /// The coupon is the account's: the Cash Inventory items it gave, and
    /// the balances after.
    Redeemed {
        items: Vec<CashItem>,
        balance: Balance,
    },
    /// Someone redeemed it first.
    Used,
    Expired,
}

/// The coupon with `code`, which must be in upper case.
pub fn find(code: &str) -> crate::Result<Option<Coupon>> {
    let mut conn = crate::connection()?;
    let Some((id, expires_at, redeemed_at)) = coupons::table
        .filter(coupons::code.eq(code))
        .select((coupons::id, coupons::expires_at, coupons::redeemed_at))
        .first::<(i32, Option<SystemTime>, Option<SystemTime>)>(&mut conn)
        .optional()?
    else {
        return Ok(None);
    };
    let rows: Vec<(i16, i32, i32)> = coupon_rewards::table
        .filter(coupon_rewards::coupon_id.eq(id))
        .order(coupon_rewards::id)
        .select((
            coupon_rewards::kind,
            coupon_rewards::item_id,
            coupon_rewards::quantity,
        ))
        .load(&mut conn)?;
    Ok(Some(Coupon {
        id,
        expires_at,
        redeemed: redeemed_at.is_some(),
        rewards: rows.into_iter().filter_map(Reward::from_row).collect(),
    }))
}

/// Make a coupon with `code`, which must be in upper case; its id.
pub fn create(
    code: &str,
    expires_at: Option<SystemTime>,
    rewards: &[Reward],
) -> crate::Result<i32> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<i32> {
        let id = diesel::insert_into(coupons::table)
            .values((coupons::code.eq(code), coupons::expires_at.eq(expires_at)))
            .returning(coupons::id)
            .get_result(conn)?;
        let rows: Vec<_> = rewards
            .iter()
            .map(|reward| {
                let (kind, item_id, quantity) = reward.row();
                (
                    coupon_rewards::coupon_id.eq(id),
                    coupon_rewards::kind.eq(kind),
                    coupon_rewards::item_id.eq(item_id),
                    coupon_rewards::quantity.eq(quantity),
                )
            })
            .collect();
        if !rows.is_empty() {
            diesel::insert_into(coupon_rewards::table)
                .values(&rows)
                .execute(conn)?;
        }
        Ok(id)
    })?)
}

/// Redeem coupon `coupon_id` for the account: mark it used, add `add` to
/// the balances, put `items` in the Cash Inventory and save `player`, who
/// holds the rest of what it gave, in one transaction.
pub fn redeem(
    coupon_id: i32,
    now: SystemTime,
    add: Balance,
    items: &[NewCashItem],
    player: &Player,
) -> crate::Result<Redemption> {
    let account_id = player.account_id;
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<Redemption> {
        let (expires_at, redeemed_at) = coupons::table
            .find(coupon_id)
            .select((coupons::expires_at, coupons::redeemed_at))
            .for_update()
            .first::<(Option<SystemTime>, Option<SystemTime>)>(conn)?;
        if redeemed_at.is_some() {
            return Ok(Redemption::Used);
        }
        if expires_at.is_some_and(|at| at <= now) {
            return Ok(Redemption::Expired);
        }
        diesel::update(coupons::table.find(coupon_id))
            .set((
                coupons::redeemed_by.eq(account_id),
                coupons::redeemed_at.eq(now),
            ))
            .execute(conn)?;
        diesel::update(accounts::table.find(account_id))
            .set((
                accounts::nx_credit.eq(accounts::nx_credit + add.credit),
                accounts::maple_points.eq(accounts::maple_points + add.points),
                accounts::nx_prepaid.eq(accounts::nx_prepaid + add.prepaid),
            ))
            .execute(conn)?;
        let items = if items.is_empty() {
            Vec::new()
        } else {
            diesel::insert_into(cash_items::table)
                .values(items)
                .get_results::<CashItem>(conn)?
        };
        write_player(conn, player)?;
        let balance = accounts::table
            .find(account_id)
            .select((
                accounts::nx_credit,
                accounts::maple_points,
                accounts::nx_prepaid,
            ))
            .first::<Balance>(conn)?;
        Ok(Redemption::Redeemed { items, balance })
    })?)
}
