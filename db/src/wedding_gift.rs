//! Wedding gifts: items guests gave a groom or a bride from their wishlist,
//! waiting until the receiver takes them from Pila (report b085).

use crate::player::{equipment_stats_row, write_player};
use crate::schema::{wedding_gift_equipment_stats, wedding_gifts};
use diesel::prelude::*;
use game::{EquipStats, Item, Player};
use std::time::SystemTime;

/// A gift waiting for its receiver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gift {
    pub id: i64,
    pub item: Item,
}

#[derive(Queryable)]
#[diesel(table_name = wedding_gifts)]
struct GiftRow {
    id: i64,
    _receiver_id: i32,
    _giver_id: i32,
    _given_at: SystemTime,
    item_id: i32,
    quantity: i16,
    owner: String,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

#[derive(Insertable)]
#[diesel(table_name = wedding_gifts)]
struct GiftInsert<'a> {
    receiver_id: i32,
    giver_id: i32,
    given_at: SystemTime,
    item_id: i32,
    quantity: i16,
    owner: &'a str,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

equipment_stats_row!(GiftEquipmentStatsRow, wedding_gift_equipment_stats, gift_id);

fn gift((row, equip): (GiftRow, Option<GiftEquipmentStatsRow>)) -> Gift {
    Gift {
        id: row.id,
        item: Item {
            item_id: row.item_id,
            quantity: row.quantity,
            owner: row.owner,
            flags: row.flags,
            expiration: row.expires_at,
            cash_serial: row.cash_serial,
            equip: equip.map(GiftEquipmentStatsRow::stats),
            pet: None,
        },
    }
}

/// `receiver`'s gifts, oldest first.
pub fn list(receiver: i32) -> crate::Result<Vec<Gift>> {
    let rows: Vec<(GiftRow, Option<GiftEquipmentStatsRow>)> = wedding_gifts::table
        .left_join(wedding_gift_equipment_stats::table)
        .filter(wedding_gifts::receiver_id.eq(receiver))
        .order(wedding_gifts::id.asc())
        .load(&mut crate::connection()?)?;
    Ok(rows.into_iter().map(gift).collect())
}

/// Whether `receiver` has gifts waiting.
pub fn any(receiver: i32) -> crate::Result<bool> {
    let first: Option<i64> = wedding_gifts::table
        .filter(wedding_gifts::receiver_id.eq(receiver))
        .select(wedding_gifts::id)
        .first(&mut crate::connection()?)
        .optional()?;
    Ok(first.is_some())
}

/// Leave `item` for `receiver` and save `giver`, who gave it up, in one
/// transaction. The gift's id.
pub fn give(receiver: i32, item: &Item, giver: &Player, now: SystemTime) -> crate::Result<i64> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let id: i64 = diesel::insert_into(wedding_gifts::table)
            .values(&GiftInsert {
                receiver_id: receiver,
                giver_id: giver.id(),
                given_at: now,
                item_id: item.item_id,
                quantity: item.quantity,
                owner: &item.owner,
                flags: item.flags,
                expires_at: item.expiration,
                cash_serial: item.cash_serial,
            })
            .returning(wedding_gifts::id)
            .get_result(conn)?;
        if let Some(stats) = &item.equip {
            diesel::insert_into(wedding_gift_equipment_stats::table)
                .values(&GiftEquipmentStatsRow::new(id, stats))
                .execute(conn)?;
        }
        write_player(conn, giver)?;
        Ok(id)
    })?)
}

/// Remove `receiver`'s gift `id` and save `player`, who took it, in one
/// transaction. Whether it was still there; if not, nothing is saved.
pub fn take(receiver: i32, id: i64, player: &Player) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let removed = diesel::delete(
            wedding_gifts::table
                .filter(wedding_gifts::id.eq(id))
                .filter(wedding_gifts::receiver_id.eq(receiver)),
        )
        .execute(conn)?;
        if removed == 0 {
            return Ok(false);
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}
