//! The Cash Shop: an account's NX balances and Cash Inventory, and each
//! character's wish list.

use crate::player::write_player;
use crate::schema::{accounts, cash_items, cash_sales, characters, wish_lists};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use game::Player;
use std::time::SystemTime;

/// What a purchase is paid with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CashType {
    Credit,
    MaplePoints,
    Prepaid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Queryable)]
pub struct Balance {
    pub credit: i32,
    pub points: i32,
    pub prepaid: i32,
}

/// An item in an account's Cash Inventory. `id` is its cash serial number.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable, Insertable)]
#[diesel(table_name = cash_items)]
pub struct CashItem {
    pub id: i64,
    pub account_id: i32,
    pub item_id: i32,
    pub sn: i32,
    pub quantity: i16,
    pub gift_from: String,
    pub expires_at: Option<SystemTime>,
    /// A gift's message, until the account has been shown the gift.
    pub gift_message: Option<String>,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = cash_items)]
pub struct NewCashItem {
    pub account_id: i32,
    pub item_id: i32,
    pub sn: i32,
    pub quantity: i16,
    pub expires_at: Option<SystemTime>,
}

/// A gift for another account's Cash Inventory.
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = cash_items)]
pub struct NewGift {
    pub account_id: i32,
    pub item_id: i32,
    pub sn: i32,
    pub quantity: i16,
    pub expires_at: Option<SystemTime>,
    pub gift_from: String,
    pub gift_message: Option<String>,
}

/// The character a gift is for.
#[derive(Clone, Debug, PartialEq, Eq, Queryable)]
pub struct GiftReceiver {
    pub id: i32,
    pub account_id: i32,
    pub name: String,
    pub gender: i16,
}

pub fn balance(account_id: i32) -> crate::Result<Balance> {
    Ok(balance_of(&mut *crate::connection()?, account_id)?)
}

/// Add `amount` (which may be negative) to the account's `cash_type`
/// balance, keeping it within 0..=i32::MAX, and return the balances.
/// `None` if there is no such account.
pub fn grant(account_id: i32, cash_type: CashType, amount: i32) -> crate::Result<Option<Balance>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| add_cash(conn, account_id, cash_type, amount))?)
}

/// `grant` within a transaction.
pub(crate) fn add_cash(
    conn: &mut PgConnection,
    account_id: i32,
    cash_type: CashType,
    amount: i32,
) -> diesel::QueryResult<Option<Balance>> {
    let account = accounts::table.find(account_id);
    let Some(before) = account
        .select((
            accounts::nx_credit,
            accounts::maple_points,
            accounts::nx_prepaid,
        ))
        .for_update()
        .first::<Balance>(conn)
        .optional()?
    else {
        return Ok(None);
    };
    let add =
        |value: i32| (i64::from(value) + i64::from(amount)).clamp(0, i64::from(i32::MAX)) as i32;
    match cash_type {
        CashType::Credit => diesel::update(account)
            .set(accounts::nx_credit.eq(add(before.credit)))
            .execute(conn)?,
        CashType::MaplePoints => diesel::update(account)
            .set(accounts::maple_points.eq(add(before.points)))
            .execute(conn)?,
        CashType::Prepaid => diesel::update(account)
            .set(accounts::nx_prepaid.eq(add(before.prepaid)))
            .execute(conn)?,
    };
    Ok(Some(balance_of(conn, account_id)?))
}

/// The account's Cash Inventory, and the gifts in it that it hasn't been
/// shown yet, which from now on count as shown.
pub fn open_cash_inventory(account_id: i32) -> crate::Result<(Vec<CashItem>, Vec<CashItem>)> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        let items: Vec<CashItem> = cash_items::table
            .filter(cash_items::account_id.eq(account_id))
            .order(cash_items::id)
            .load(conn)?;
        let gifts: Vec<CashItem> = items
            .iter()
            .filter(|item| item.gift_message.is_some())
            .cloned()
            .collect();
        let ids: Vec<i64> = gifts.iter().map(|gift| gift.id).collect();
        diesel::update(cash_items::table.filter(cash_items::id.eq_any(&ids)))
            .set(cash_items::gift_message.eq(None::<String>))
            .execute(conn)?;
        Ok((items, gifts))
    })?)
}

/// Delete those of the account's Cash Inventory items `ids` whose time ran
/// out by `now`, returning which.
pub fn expire(account_id: i32, ids: &[i64], now: SystemTime) -> crate::Result<Vec<i64>> {
    let mut conn = crate::connection()?;
    Ok(diesel::delete(
        cash_items::table
            .filter(cash_items::account_id.eq(account_id))
            .filter(cash_items::id.eq_any(ids))
            .filter(cash_items::expires_at.le(now)),
    )
    .returning(cash_items::id)
    .get_results(&mut conn)?)
}

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// The character named `name`, ignoring case, that a gift would go to.
pub fn gift_receiver(name: &str) -> crate::Result<Option<GiftReceiver>> {
    Ok(characters::table
        .filter(lower(characters::name).eq(lower(name)))
        .select((
            characters::id,
            characters::accountid,
            characters::name,
            characters::gender,
        ))
        .first(&mut crate::connection()?)
        .optional()?)
}

/// A new cash serial number from the Cash Inventory's own, for a cash item
/// that comes to a character from elsewhere (a quest's pet), so that it may
/// be put in the Cash Inventory later.
pub fn new_serial() -> crate::Result<i64> {
    let next = diesel::dsl::sql::<diesel::sql_types::BigInt>("nextval('cash_items_id_seq')");
    Ok(diesel::select(next).get_result(&mut crate::connection()?)?)
}

/// The account's Cash Inventory item with cash serial `id`.
pub fn cash_item(account_id: i32, id: i64) -> crate::Result<Option<CashItem>> {
    Ok(cash_items::table
        .find(id)
        .filter(cash_items::account_id.eq(account_id))
        .first(&mut crate::connection()?)
        .optional()?)
}

/// Move a Cash Inventory item to a character: delete it and save `player`,
/// who now carries it, in one transaction. False if it was already gone.
pub fn take_out(account_id: i32, id: i64, player: &Player) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<bool> {
        let taken = diesel::delete(
            cash_items::table
                .find(id)
                .filter(cash_items::account_id.eq(account_id)),
        )
        .execute(conn)?;
        if taken == 0 {
            return Ok(false);
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}

/// Move a character's cash item back to the Cash Inventory: insert it and
/// save `player`, who no longer carries it, in one transaction.
pub fn put_back(item: &CashItem, player: &Player) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<()> {
        diesel::insert_into(cash_items::table)
            .values(item)
            .execute(conn)?;
        write_player(conn, player)
    })?;
    Ok(())
}

/// Pay `price` from the account's `cash_type` balance and put `items`, all
/// that commodity `sn` sells, in its Cash Inventory, in one transaction.
/// `None` if the balance is short.
pub fn buy(
    account_id: i32,
    cash_type: CashType,
    sn: i32,
    price: i32,
    items: &[NewCashItem],
) -> crate::Result<Option<(Vec<CashItem>, Balance)>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        if !pay(conn, account_id, cash_type, price)? {
            return Ok(None);
        }
        let bought = diesel::insert_into(cash_items::table)
            .values(items)
            .get_results::<CashItem>(conn)?;
        record_sale(conn, sn)?;
        Ok(Some((bought, balance_of(conn, account_id)?)))
    })?)
}

/// Pay `price` from the sender's NX Credit, or its NX Prepaid if the credit
/// is short, for the commodity `sn`, and put `gifts`, its item or a
/// package's items, in the receiver's Cash Inventory, in one transaction.
/// `None` if neither balance covers the price.
pub fn gift(
    sender: i32,
    (sn, price): (i32, i32),
    gifts: &[NewGift],
) -> crate::Result<Option<Balance>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        if !pay(conn, sender, CashType::Credit, price)?
            && !pay(conn, sender, CashType::Prepaid, price)?
        {
            return Ok(None);
        }
        diesel::insert_into(cash_items::table)
            .values(gifts)
            .execute(conn)?;
        record_sale(conn, sn)?;
        Ok(Some(balance_of(conn, sender)?))
    })?)
}

/// Pay `price` from the buyer's `cash_type` balance for a crush or
/// friendship ring: put `ring` in the buyer's Cash Inventory and `gift`, its
/// partner, in the receiver's, and record the pair, in one transaction.
/// `None` if the balance is short.
pub fn buy_ring(
    cash_type: CashType,
    price: i32,
    (ring, buyer_id): (&NewCashItem, i32),
    (gift, receiver_id): (&NewGift, i32),
) -> crate::Result<Option<(CashItem, Balance)>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        if !pay(conn, ring.account_id, cash_type, price)? {
            return Ok(None);
        }
        let bought = diesel::insert_into(cash_items::table)
            .values(ring)
            .get_result::<CashItem>(conn)?;
        let sent = diesel::insert_into(cash_items::table)
            .values(gift)
            .returning(cash_items::id)
            .get_result::<i64>(conn)?;
        crate::ring::insert_pair(
            conn,
            (bought.id, buyer_id),
            (sent, receiver_id),
            ring.item_id,
        )?;
        record_sale(conn, ring.sn)?;
        Ok(Some((bought, balance_of(conn, ring.account_id)?)))
    })?)
}

/// Pay `price` from the account's `cash_type` balance and save `player`,
/// whose bags have grown or whose second pendant slot is open longer, in one
/// transaction. `None` if the balance is short.
pub fn expand_inventory(
    cash_type: CashType,
    price: i32,
    player: &Player,
) -> crate::Result<Option<Balance>> {
    let account_id = player.account_id;
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        if !pay(conn, account_id, cash_type, price)? {
            return Ok(None);
        }
        write_player(conn, player)?;
        Ok(Some(balance_of(conn, account_id)?))
    })?)
}

/// Pay `price` from the account's `cash_type` balance and give its storage
/// `slots` slots, in one transaction. `None` if the balance is short.
pub fn expand_storage(
    account_id: i32,
    cash_type: CashType,
    price: i32,
    slots: i16,
) -> crate::Result<Option<Balance>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        if !pay(conn, account_id, cash_type, price)? {
            return Ok(None);
        }
        diesel::update(accounts::table.find(account_id))
            .set(accounts::storage_slots.eq(slots))
            .execute(conn)?;
        Ok(Some(balance_of(conn, account_id)?))
    })?)
}

/// What buying a character slot came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotPurchase {
    /// The account now has `slots` character slots, and `balance` left.
    Bought { slots: i16, balance: Balance },
    /// The account already had the most slots; nothing was paid.
    Full,
    /// The balance was short; nothing changed.
    Short,
}

/// Pay `price` for commodity `sn` from the account's `cash_type` balance and
/// give the account one more character slot, up to `max`, in one
/// transaction.
pub fn buy_character_slot(
    account_id: i32,
    cash_type: CashType,
    sn: i32,
    price: i32,
    max: i16,
) -> crate::Result<SlotPurchase> {
    let mut conn = crate::connection()?;
    let bought = conn.transaction(|conn| -> diesel::QueryResult<_> {
        let account = accounts::table.find(account_id);
        let slots = diesel::update(account.filter(accounts::character_slots.lt(max)))
            .set(accounts::character_slots.eq(accounts::character_slots + 1))
            .returning(accounts::character_slots)
            .get_result::<i16>(conn)
            .optional()?;
        let Some(slots) = slots else {
            return Ok(SlotPurchase::Full);
        };
        if !pay(conn, account_id, cash_type, price)? {
            return Err(diesel::result::Error::RollbackTransaction);
        }
        record_sale(conn, sn)?;
        let balance = balance_of(conn, account_id)?;
        Ok(SlotPurchase::Bought { slots, balance })
    });
    match bought {
        Err(diesel::result::Error::RollbackTransaction) => Ok(SlotPurchase::Short),
        bought => Ok(bought?),
    }
}

/// The serial numbers of the commodities sold most, best first.
pub fn best_sellers(limit: i64) -> crate::Result<Vec<i32>> {
    Ok(cash_sales::table
        .order((cash_sales::sold.desc(), cash_sales::sn))
        .select(cash_sales::sn)
        .limit(limit)
        .load(&mut crate::connection()?)?)
}

/// Count one more sale of commodity `sn`.
pub(crate) fn record_sale(conn: &mut PgConnection, sn: i32) -> diesel::QueryResult<()> {
    diesel::insert_into(cash_sales::table)
        .values((cash_sales::sn.eq(sn), cash_sales::sold.eq(1)))
        .on_conflict(cash_sales::sn)
        .do_update()
        .set(cash_sales::sold.eq(cash_sales::sold + 1))
        .execute(conn)?;
    Ok(())
}

/// Take `price` from the account's `cash_type` balance; false if it is short.
pub(crate) fn pay(
    conn: &mut PgConnection,
    account_id: i32,
    cash_type: CashType,
    price: i32,
) -> diesel::QueryResult<bool> {
    let account = accounts::table.find(account_id);
    let paid = match cash_type {
        CashType::Credit => diesel::update(account.filter(accounts::nx_credit.ge(price)))
            .set(accounts::nx_credit.eq(accounts::nx_credit - price))
            .execute(conn)?,
        CashType::MaplePoints => diesel::update(account.filter(accounts::maple_points.ge(price)))
            .set(accounts::maple_points.eq(accounts::maple_points - price))
            .execute(conn)?,
        CashType::Prepaid => diesel::update(account.filter(accounts::nx_prepaid.ge(price)))
            .set(accounts::nx_prepaid.eq(accounts::nx_prepaid - price))
            .execute(conn)?,
    };
    Ok(paid > 0)
}

pub(crate) fn balance_of(conn: &mut PgConnection, account_id: i32) -> diesel::QueryResult<Balance> {
    accounts::table
        .find(account_id)
        .select((
            accounts::nx_credit,
            accounts::maple_points,
            accounts::nx_prepaid,
        ))
        .first::<Balance>(conn)
}

/// The character's wish list, in slot order.
pub fn wish_list(character_id: i32) -> crate::Result<Vec<i32>> {
    Ok(wish_lists::table
        .filter(wish_lists::character_id.eq(character_id))
        .order(wish_lists::slot)
        .select(wish_lists::sn)
        .load(&mut crate::connection()?)?)
}

/// Replace the character's wish list.
pub fn set_wish_list(character_id: i32, sns: &[i32]) -> crate::Result<()> {
    let rows: Vec<_> = sns
        .iter()
        .enumerate()
        .map(|(slot, &sn)| {
            (
                wish_lists::character_id.eq(character_id),
                wish_lists::slot.eq(slot as i16),
                wish_lists::sn.eq(sn),
            )
        })
        .collect();
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<()> {
        diesel::delete(wish_lists::table.filter(wish_lists::character_id.eq(character_id)))
            .execute(conn)?;
        if !rows.is_empty() {
            diesel::insert_into(wish_lists::table)
                .values(&rows)
                .execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}
