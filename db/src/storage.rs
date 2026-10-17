//! An account's storage: its mesos and the items its characters stored.

use crate::player::{equipment_stats_row, write_player};
use crate::schema::{accounts, storage_equipment_stats, storage_items};
use diesel::prelude::*;
use game::{EquipStats, Item, Player, Storage};
use std::time::SystemTime;

#[derive(Queryable)]
#[diesel(table_name = storage_items)]
struct StorageItemRow {
    _id: i64,
    _account_id: i32,
    _position: i16,
    item_id: i32,
    quantity: i16,
    owner: String,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

#[derive(Insertable)]
#[diesel(table_name = storage_items)]
struct NewStorageItem<'a> {
    account_id: i32,
    position: i16,
    item_id: i32,
    quantity: i16,
    owner: &'a str,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

equipment_stats_row!(
    StorageEquipmentStatsRow,
    storage_equipment_stats,
    storage_item_id
);

/// Load the account's storage, its items in their order.
pub fn load(account_id: i32) -> crate::Result<Storage> {
    let mut conn = crate::connection()?;
    let (slots, meso): (i16, i32) = accounts::table
        .find(account_id)
        .select((accounts::storage_slots, accounts::storage_meso))
        .first(&mut conn)?;
    let rows: Vec<(StorageItemRow, Option<StorageEquipmentStatsRow>)> = storage_items::table
        .left_join(storage_equipment_stats::table)
        .filter(storage_items::account_id.eq(account_id))
        .order(storage_items::position)
        .load(&mut conn)?;
    let items = rows
        .into_iter()
        .map(|(row, equip)| Item {
            item_id: row.item_id,
            quantity: row.quantity,
            owner: row.owner,
            flags: row.flags,
            expiration: row.expires_at,
            cash_serial: row.cash_serial,
            equip: equip.map(StorageEquipmentStatsRow::stats),
        })
        .collect();
    Ok(Storage::new(
        u8::try_from(slots).unwrap_or(game::storage::DEFAULT_STORAGE_SLOTS),
        meso,
        items,
    ))
}

/// Write the account's storage mesos and items, and `player` if given (the
/// character who moved something in or out), in one transaction, so nothing
/// is ever in both places or neither.
pub fn save(account_id: i32, storage: &Storage, player: Option<&Player>) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> QueryResult<()> {
        diesel::update(accounts::table.find(account_id))
            .set(accounts::storage_meso.eq(storage.meso))
            .execute(conn)?;
        // Equipment stats cascade with their item rows.
        diesel::delete(storage_items::table.filter(storage_items::account_id.eq(account_id)))
            .execute(conn)?;
        for (position, item) in storage.all().iter().enumerate() {
            insert_item(conn, account_id, position as i16, item)?;
        }
        match player {
            Some(player) => write_player(conn, player),
            None => Ok(()),
        }
    })?;
    Ok(())
}

fn insert_item(
    conn: &mut PgConnection,
    account_id: i32,
    position: i16,
    item: &Item,
) -> QueryResult<()> {
    let id: i64 = diesel::insert_into(storage_items::table)
        .values(&NewStorageItem {
            account_id,
            position,
            item_id: item.item_id,
            quantity: item.quantity,
            owner: &item.owner,
            flags: item.flags,
            expires_at: item.expiration,
            cash_serial: item.cash_serial,
        })
        .returning(storage_items::id)
        .get_result(conn)?;
    if let Some(stats) = &item.equip {
        diesel::insert_into(storage_equipment_stats::table)
            .values(&StorageEquipmentStatsRow::new(id, stats))
            .execute(conn)?;
    }
    Ok(())
}
