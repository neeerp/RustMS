//! Hired merchants (`game::hired_merchant`): each character's one merchant,
//! what it lists and what it sold. Every change that moves items or mesos
//! between a merchant and a character saves the character in the same
//! transaction, so nothing is ever in both places or neither.

use crate::player::{equipment_stats_row, write_player};
use crate::schema::{
    characters, hired_merchant_equipment_stats, hired_merchant_items, hired_merchant_sales,
    hired_merchants,
};
use diesel::prelude::*;
use game::hired_merchant::{HiredMerchant, MerchantState, Sale, Stand, Stock};
use game::mini_room::Listing;
use game::{EquipStats, Item, Player};
use std::time::SystemTime;

#[derive(Queryable)]
#[diesel(table_name = hired_merchants)]
struct MerchantRow {
    owner_id: i32,
    channel_id: i16,
    map_id: i32,
    x: i16,
    y: i16,
    foothold: i16,
    item_id: i32,
    title: String,
    state: i16,
    opened_at: Option<SystemTime>,
    closed_at: Option<SystemTime>,
    meso: i32,
}

#[derive(Insertable)]
#[diesel(table_name = hired_merchants)]
struct MerchantInsert<'a> {
    owner_id: i32,
    channel_id: i16,
    map_id: i32,
    x: i16,
    y: i16,
    foothold: i16,
    item_id: i32,
    title: &'a str,
}

#[derive(Queryable)]
#[diesel(table_name = hired_merchant_items)]
struct ItemRow {
    id: i64,
    owner_id: i32,
    bundles: i16,
    price: i32,
    item_id: i32,
    quantity: i16,
    owner: String,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

#[derive(Insertable)]
#[diesel(table_name = hired_merchant_items)]
struct ItemInsert<'a> {
    owner_id: i32,
    bundles: i16,
    price: i32,
    item_id: i32,
    quantity: i16,
    owner: &'a str,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

#[derive(Queryable)]
#[diesel(table_name = hired_merchant_sales)]
struct SaleRow {
    _id: i64,
    owner_id: i32,
    item_id: i32,
    quantity: i16,
    mesos: i32,
    buyer: String,
    _sold_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = hired_merchant_sales)]
struct SaleInsert<'a> {
    owner_id: i32,
    item_id: i32,
    quantity: i16,
    mesos: i32,
    buyer: &'a str,
    sold_at: SystemTime,
}

equipment_stats_row!(
    MerchantEquipmentStatsRow,
    hired_merchant_equipment_stats,
    merchant_item_id
);

/// Where a character's merchant is and how it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub state: MerchantState,
    pub channel_id: u8,
    pub map_id: i32,
}

/// `owner`'s merchant, if they have one: where it is and how it stands.
pub fn find(owner: i32) -> crate::Result<Option<Placement>> {
    let row: Option<(i16, i16, i32)> = hired_merchants::table
        .find(owner)
        .select((
            hired_merchants::state,
            hired_merchants::channel_id,
            hired_merchants::map_id,
        ))
        .first(&mut crate::connection()?)
        .optional()?;
    Ok(row.map(|(state, channel_id, map_id)| Placement {
        state: MerchantState::from_i16(state).unwrap_or(MerchantState::Closed),
        channel_id: channel_id as u8,
        map_id,
    }))
}

/// `owner`'s merchant with all it holds.
pub fn load(owner: i32) -> crate::Result<Option<HiredMerchant>> {
    let mut conn = crate::connection()?;
    let rows: Vec<(MerchantRow, String)> = hired_merchants::table
        .inner_join(characters::table)
        .filter(hired_merchants::owner_id.eq(owner))
        .select((hired_merchants::all_columns, characters::name))
        .load(&mut conn)?;
    Ok(assemble(&mut conn, rows)?.pop())
}

/// The open merchants on `map_id` in channel `channel_id`. A world server
/// serves one world, and its fields are keyed by channel alone.
pub fn load_map(channel_id: u8, map_id: i32) -> crate::Result<Vec<HiredMerchant>> {
    let mut conn = crate::connection()?;
    let rows: Vec<(MerchantRow, String)> = hired_merchants::table
        .inner_join(characters::table)
        .filter(hired_merchants::channel_id.eq(i16::from(channel_id)))
        .filter(hired_merchants::map_id.eq(map_id))
        .filter(hired_merchants::state.eq(MerchantState::Open as i16))
        .order(hired_merchants::opened_at.asc())
        .select((hired_merchants::all_columns, characters::name))
        .load(&mut conn)?;
    Ok(assemble(&mut conn, rows)?)
}

/// Merchants with their stock in the order listed and their sales in the
/// order made.
fn assemble(
    conn: &mut PgConnection,
    rows: Vec<(MerchantRow, String)>,
) -> QueryResult<Vec<HiredMerchant>> {
    let owners: Vec<i32> = rows.iter().map(|(row, _)| row.owner_id).collect();
    let items: Vec<(ItemRow, Option<MerchantEquipmentStatsRow>)> = hired_merchant_items::table
        .left_join(hired_merchant_equipment_stats::table)
        .filter(hired_merchant_items::owner_id.eq_any(&owners))
        .order(hired_merchant_items::id.asc())
        .load(conn)?;
    let sales: Vec<SaleRow> = hired_merchant_sales::table
        .filter(hired_merchant_sales::owner_id.eq_any(&owners))
        .order(hired_merchant_sales::id.asc())
        .load(conn)?;
    let mut merchants: Vec<HiredMerchant> = rows
        .into_iter()
        .map(|(row, owner_name)| HiredMerchant {
            owner_id: row.owner_id,
            owner_name,
            stand: Stand {
                channel_id: row.channel_id as u8,
                map_id: row.map_id,
                x: row.x,
                y: row.y,
                foothold: row.foothold,
            },
            item_id: row.item_id,
            title: row.title,
            state: MerchantState::from_i16(row.state).unwrap_or(MerchantState::Closed),
            opened_at: row.opened_at,
            closed_at: row.closed_at,
            meso: row.meso,
            stock: vec![],
            sales: vec![],
        })
        .collect();
    for (row, equip) in items {
        if let Some(merchant) = merchants.iter_mut().find(|m| m.owner_id == row.owner_id) {
            merchant.stock.push(Stock {
                id: row.id,
                listing: Listing {
                    item: Item {
                        item_id: row.item_id,
                        quantity: row.quantity,
                        owner: row.owner,
                        flags: row.flags,
                        expiration: row.expires_at,
                        cash_serial: row.cash_serial,
                        equip: equip.map(MerchantEquipmentStatsRow::stats),
                        pet: None,
                    },
                    bundles: row.bundles,
                    price: row.price,
                },
            });
        }
    }
    for row in sales {
        if let Some(merchant) = merchants.iter_mut().find(|m| m.owner_id == row.owner_id) {
            merchant.sales.push(Sale {
                item_id: row.item_id,
                quantity: row.quantity,
                mesos: row.mesos,
                buyer: row.buyer,
            });
        }
    }
    Ok(merchants)
}

/// Set up a merchant for `owner` at `stand`, opened with permit `item_id`
/// and named `title`. False if they already have one.
pub fn create(owner: i32, stand: &Stand, item_id: i32, title: &str) -> crate::Result<bool> {
    let inserted = diesel::insert_into(hired_merchants::table)
        .values(&MerchantInsert {
            owner_id: owner,
            channel_id: i16::from(stand.channel_id),
            map_id: stand.map_id,
            x: stand.x,
            y: stand.y,
            foothold: stand.foothold,
            item_id,
            title,
        })
        .on_conflict_do_nothing()
        .execute(&mut crate::connection()?)?;
    Ok(inserted == 1)
}

/// List `listing` in `player`'s merchant and save them, who gave it up,
/// in one transaction. The new row's id.
pub fn put(player: &Player, listing: &Listing) -> crate::Result<i64> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let item = &listing.item;
        let id: i64 = diesel::insert_into(hired_merchant_items::table)
            .values(&ItemInsert {
                owner_id: player.id(),
                bundles: listing.bundles,
                price: listing.price,
                item_id: item.item_id,
                quantity: item.quantity,
                owner: &item.owner,
                flags: item.flags,
                expires_at: item.expiration,
                cash_serial: item.cash_serial,
            })
            .returning(hired_merchant_items::id)
            .get_result(conn)?;
        if let Some(stats) = &item.equip {
            diesel::insert_into(hired_merchant_equipment_stats::table)
                .values(&MerchantEquipmentStatsRow::new(id, stats))
                .execute(conn)?;
        }
        write_player(conn, player)?;
        Ok(id)
    })?)
}

/// Take row `stock`, with `bundles` left, out of `player`'s merchant and
/// save them, who took those back, in one transaction. Whether it was
/// still there with as many; if not, nothing is saved.
pub fn take_back(player: &Player, stock: i64, bundles: i16) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let removed = diesel::delete(
            hired_merchant_items::table
                .filter(hired_merchant_items::id.eq(stock))
                .filter(hired_merchant_items::owner_id.eq(player.id()))
                .filter(hired_merchant_items::bundles.eq(bundles)),
        )
        .execute(conn)?;
        if removed == 0 {
            return Ok(false);
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}

/// Open `player`'s merchant at `now` and save them, in one transaction.
/// Whether it was being set up.
pub fn open(player: &Player, now: SystemTime) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let opened = diesel::update(
            hired_merchants::table
                .find(player.id())
                .filter(hired_merchants::state.eq(MerchantState::Setup as i16)),
        )
        .set((
            hired_merchants::state.eq(MerchantState::Open as i16),
            hired_merchants::opened_at.eq(now),
        ))
        .execute(conn)?;
        if opened == 0 {
            return Ok(false);
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}

/// A purchase from a merchant: `bundles` of row `stock`, which come to
/// `sale`, earning the merchant `net` after tax.
#[derive(Clone, Debug)]
pub struct Purchase<'a> {
    pub owner: i32,
    pub stock: i64,
    pub bundles: i16,
    pub net: i32,
    pub sale: &'a Sale,
}

/// Sell `purchase` from an open merchant and save `buyer`, who has the
/// items and paid, in one transaction. Whether the merchant was open with
/// the bundles in stock and room for the mesos; if not, nothing changes.
pub fn sell(purchase: &Purchase, buyer: &Player, now: SystemTime) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn
        .transaction(|conn| -> QueryResult<_> {
            let paid = diesel::update(
                hired_merchants::table
                    .find(purchase.owner)
                    .filter(hired_merchants::state.eq(MerchantState::Open as i16))
                    .filter(hired_merchants::meso.le(i32::MAX - purchase.net)),
            )
            .set(hired_merchants::meso.eq(hired_merchants::meso + purchase.net))
            .execute(conn)?;
            let taken = diesel::update(
                hired_merchant_items::table
                    .filter(hired_merchant_items::id.eq(purchase.stock))
                    .filter(hired_merchant_items::owner_id.eq(purchase.owner))
                    .filter(hired_merchant_items::bundles.ge(purchase.bundles)),
            )
            .set(hired_merchant_items::bundles.eq(hired_merchant_items::bundles - purchase.bundles))
            .execute(conn)?;
            if paid == 0 || taken == 0 {
                return Err(diesel::result::Error::RollbackTransaction);
            }
            diesel::insert_into(hired_merchant_sales::table)
                .values(&SaleInsert {
                    owner_id: purchase.owner,
                    item_id: purchase.sale.item_id,
                    quantity: purchase.sale.quantity,
                    mesos: purchase.sale.mesos,
                    buyer: &purchase.sale.buyer,
                    sold_at: now,
                })
                .execute(conn)?;
            write_player(conn, buyer)?;
            Ok(true)
        })
        .or_else(|error| match error {
            diesel::result::Error::RollbackTransaction => Ok(false),
            error => Err(error),
        })?)
}

/// Take `meso` of what `player`'s merchant made and save them, who have it
/// now, in one transaction. Whether the merchant had that much.
pub fn take_meso(player: &Player, meso: i32) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let taken = diesel::update(
            hired_merchants::table
                .find(player.id())
                .filter(hired_merchants::meso.ge(meso)),
        )
        .set(hired_merchants::meso.eq(hired_merchants::meso - meso))
        .execute(conn)?;
        if taken == 0 {
            return Ok(false);
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}

/// Strike the sold-out rows off `owner`'s merchant.
pub fn organize(owner: i32) -> crate::Result<usize> {
    Ok(diesel::delete(
        hired_merchant_items::table
            .filter(hired_merchant_items::owner_id.eq(owner))
            .filter(hired_merchant_items::bundles.eq(0)),
    )
    .execute(&mut crate::connection()?)?)
}

/// What the owner took from their merchant when they closed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retrieved {
    pub items: bool,
    pub meso: bool,
}

/// Close `player`'s merchant at `now` and save them, with what they took
/// from it, in one transaction. A merchant they emptied goes; one with
/// items or mesos left stays closed, for Fredrick. Whether it still held
/// what `seen` holds: a purchase written since, which a stalled buyer can
/// still do, leaves it as it is.
pub fn close(
    player: &Player,
    seen: &HiredMerchant,
    retrieved: Retrieved,
    now: SystemTime,
) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    let owner = player.id();
    let mut expected: Vec<(i64, i16)> = seen
        .stock
        .iter()
        .map(|stock| (stock.id, stock.listing.bundles))
        .collect();
    expected.sort_unstable();
    Ok(conn.transaction(|conn| -> QueryResult<bool> {
        // A purchase takes this row first, so it waits for the close.
        let meso: Option<i32> = hired_merchants::table
            .find(owner)
            .select(hired_merchants::meso)
            .for_update()
            .first(conn)
            .optional()?;
        let held: Vec<(i64, i16)> = hired_merchant_items::table
            .filter(hired_merchant_items::owner_id.eq(owner))
            .select((hired_merchant_items::id, hired_merchant_items::bundles))
            .order(hired_merchant_items::id)
            .load(conn)?;
        if meso != Some(seen.meso) || held != expected {
            return Ok(false);
        }
        if retrieved.items {
            diesel::delete(
                hired_merchant_items::table.filter(hired_merchant_items::owner_id.eq(owner)),
            )
            .execute(conn)?;
        }
        if retrieved.meso {
            diesel::update(hired_merchants::table.find(owner))
                .set(hired_merchants::meso.eq(0))
                .execute(conn)?;
        }
        let items_left: i64 = hired_merchant_items::table
            .filter(hired_merchant_items::owner_id.eq(owner))
            .filter(hired_merchant_items::bundles.gt(0))
            .count()
            .get_result(conn)?;
        let meso_left: i32 = hired_merchants::table
            .find(owner)
            .select(hired_merchants::meso)
            .first(conn)?;
        if items_left == 0 && meso_left == 0 {
            // Sales and stock rows cascade.
            diesel::delete(hired_merchants::table.find(owner)).execute(conn)?;
        } else {
            diesel::update(hired_merchants::table.find(owner))
                .set((
                    hired_merchants::state.eq(MerchantState::Closed as i16),
                    hired_merchants::closed_at.eq(now),
                ))
                .execute(conn)?;
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}

/// Close `owner`'s open merchant whose time ran out at `now`, keeping what
/// it holds for Fredrick; one with nothing left goes. Whether it was open.
pub fn expire(owner: i32, now: SystemTime) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let closed = diesel::update(
            hired_merchants::table
                .find(owner)
                .filter(hired_merchants::state.eq(MerchantState::Open as i16)),
        )
        .set((
            hired_merchants::state.eq(MerchantState::Closed as i16),
            hired_merchants::closed_at.eq(now),
        ))
        .execute(conn)?;
        if closed == 0 {
            return Ok(false);
        }
        let items_left: i64 = hired_merchant_items::table
            .filter(hired_merchant_items::owner_id.eq(owner))
            .filter(hired_merchant_items::bundles.gt(0))
            .count()
            .get_result(conn)?;
        let meso_left: i32 = hired_merchants::table
            .find(owner)
            .select(hired_merchants::meso)
            .first(conn)?;
        if items_left == 0 && meso_left == 0 {
            diesel::delete(hired_merchants::table.find(owner)).execute(conn)?;
        }
        Ok(true)
    })?)
}

/// Drop `owner`'s merchant still being set up with nothing in it, as when
/// they leave before listing anything. Whether there was one.
pub fn abandon(owner: i32) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let listed: i64 = hired_merchant_items::table
            .filter(hired_merchant_items::owner_id.eq(owner))
            .count()
            .get_result(conn)?;
        if listed > 0 {
            return Ok(false);
        }
        let removed = diesel::delete(
            hired_merchants::table
                .find(owner)
                .filter(hired_merchants::state.eq(MerchantState::Setup as i16)),
        )
        .execute(conn)?;
        Ok(removed == 1)
    })?)
}
