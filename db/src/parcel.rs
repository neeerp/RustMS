//! Parcels: mesos and an item left with Duey for a character, who claims
//! or deletes them in the Duey window. A parcel expires `LIFETIME` after
//! it was sent; the receiver's next look at their parcels removes it.

use crate::player::{equipment_stats_row, write_player};
use crate::schema::{characters, parcel_equipment_stats, parcels};
use diesel::prelude::*;
use game::{EquipStats, Item, Player};
use std::time::{Duration, SystemTime};

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// How long a parcel waits before it expires.
pub const LIFETIME: Duration = Duration::from_secs(30 * 24 * 60 * 60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parcel {
    pub id: i64,
    pub sender: String,
    pub mesos: i32,
    pub message: Option<String>,
    pub quick: bool,
    pub sent_at: SystemTime,
    pub item: Option<Item>,
}

impl Parcel {
    /// When it expires.
    pub fn expires_at(&self) -> SystemTime {
        self.sent_at + LIFETIME
    }
}

/// A parcel to send.
#[derive(Clone, Debug)]
pub struct NewParcel<'a> {
    pub receiver_id: i32,
    pub sender: &'a str,
    pub mesos: i32,
    pub message: Option<&'a str>,
    pub quick: bool,
    pub sent_at: SystemTime,
    pub item: Option<&'a Item>,
}

#[derive(Queryable)]
#[diesel(table_name = parcels)]
struct ParcelRow {
    id: i64,
    _receiver_id: i32,
    sender: String,
    mesos: i32,
    message: Option<String>,
    quick: bool,
    sent_at: SystemTime,
    _announced: bool,
    item_id: Option<i32>,
    quantity: i16,
    owner: String,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

#[derive(Insertable)]
#[diesel(table_name = parcels)]
struct ParcelInsert<'a> {
    receiver_id: i32,
    sender: &'a str,
    mesos: i32,
    message: Option<&'a str>,
    quick: bool,
    sent_at: SystemTime,
    item_id: Option<i32>,
    quantity: i16,
    owner: &'a str,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

equipment_stats_row!(ParcelEquipmentStatsRow, parcel_equipment_stats, parcel_id);

fn parcel((row, equip): (ParcelRow, Option<ParcelEquipmentStatsRow>)) -> Parcel {
    let item = row.item_id.map(|item_id| Item {
        item_id,
        quantity: row.quantity,
        owner: row.owner,
        flags: row.flags,
        expiration: row.expires_at,
        cash_serial: row.cash_serial,
        equip: equip.map(ParcelEquipmentStatsRow::stats),
        pet: None,
    });
    Parcel {
        id: row.id,
        sender: row.sender,
        mesos: row.mesos,
        message: row.message,
        quick: row.quick,
        sent_at: row.sent_at,
        item,
    }
}

/// The oldest a parcel sent before `now` may be and not have expired.
fn cutoff(now: SystemTime) -> SystemTime {
    now.checked_sub(LIFETIME).unwrap_or(SystemTime::UNIX_EPOCH)
}

/// `receiver`'s parcels, oldest first, and those that expired, which are
/// gone once listed.
pub fn open(receiver: i32, now: SystemTime) -> crate::Result<(Vec<Parcel>, Vec<Parcel>)> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let rows: Vec<(ParcelRow, Option<ParcelEquipmentStatsRow>)> = parcels::table
            .left_join(parcel_equipment_stats::table)
            .filter(parcels::receiver_id.eq(receiver))
            .order(parcels::id)
            .load(conn)?;
        let (expired, waiting): (Vec<_>, Vec<_>) = rows
            .into_iter()
            .map(parcel)
            .partition(|parcel| parcel.sent_at < cutoff(now));
        let ids: Vec<i64> = expired.iter().map(|parcel| parcel.id).collect();
        // Equipment stats cascade with their parcels.
        diesel::delete(parcels::table.filter(parcels::id.eq_any(&ids))).execute(conn)?;
        Ok((waiting, expired))
    })?)
}

/// `receiver`'s parcel `id`, unless it expired.
pub fn get(receiver: i32, id: i64, now: SystemTime) -> crate::Result<Option<Parcel>> {
    let row: Option<(ParcelRow, Option<ParcelEquipmentStatsRow>)> = parcels::table
        .left_join(parcel_equipment_stats::table)
        .filter(parcels::id.eq(id))
        .filter(parcels::receiver_id.eq(receiver))
        .filter(parcels::sent_at.ge(cutoff(now)))
        .first(&mut crate::connection()?)
        .optional()?;
    Ok(row.map(parcel))
}

/// Who a parcel would go to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipient {
    pub id: i32,
    pub name: String,
    pub account_id: i32,
}

/// The character named `name`, ignoring case.
pub fn recipient(name: &str) -> crate::Result<Option<Recipient>> {
    let found: Option<(i32, String, i32)> = characters::table
        .filter(lower(characters::name).eq(lower(name)))
        .select((characters::id, characters::name, characters::accountid))
        .first(&mut crate::connection()?)
        .optional()?;
    Ok(found.map(|(id, name, account_id)| Recipient {
        id,
        name,
        account_id,
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sent {
    Sent,
    /// The receiver has `limit` parcels waiting.
    InboxFull,
    /// A parcel waiting for the receiver has this one-of-a-kind item.
    OnlyOneWaiting,
}

/// Leave `new` for its receiver and save `sender` (who paid for it and
/// gave up its item) in one transaction, unless the receiver already has
/// `limit` parcels waiting, or the item is one of a kind (`only`) and a
/// waiting parcel has one.
pub fn send(new: &NewParcel, only: bool, limit: i64, sender: &Player) -> crate::Result<Sent> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        // Two parcels sent at once can't both take the last place.
        characters::table
            .find(new.receiver_id)
            .select(characters::id)
            .for_update()
            .first::<i32>(conn)?;
        let waiting = parcels::table
            .filter(parcels::receiver_id.eq(new.receiver_id))
            .filter(parcels::sent_at.ge(cutoff(new.sent_at)));
        let count: i64 = waiting.clone().count().get_result(conn)?;
        if count >= limit {
            return Ok(Sent::InboxFull);
        }
        if let Some(item) = new.item.filter(|_| only) {
            let held: i64 = waiting
                .filter(parcels::item_id.eq(item.item_id))
                .count()
                .get_result(conn)?;
            if held > 0 {
                return Ok(Sent::OnlyOneWaiting);
            }
        }
        let item = new.item;
        let id: i64 = diesel::insert_into(parcels::table)
            .values(&ParcelInsert {
                receiver_id: new.receiver_id,
                sender: new.sender,
                mesos: new.mesos,
                message: new.message,
                quick: new.quick,
                sent_at: new.sent_at,
                item_id: item.map(|item| item.item_id),
                quantity: item.map_or(0, |item| item.quantity),
                owner: item.map_or("", |item| item.owner.as_str()),
                flags: item.map_or(0, |item| item.flags),
                expires_at: item.and_then(|item| item.expiration),
                cash_serial: item.and_then(|item| item.cash_serial),
            })
            .returning(parcels::id)
            .get_result(conn)?;
        if let Some(stats) = item.and_then(|item| item.equip.as_ref()) {
            diesel::insert_into(parcel_equipment_stats::table)
                .values(&ParcelEquipmentStatsRow::new(id, stats))
                .execute(conn)?;
        }
        write_player(conn, sender)?;
        Ok(Sent::Sent)
    })?)
}

/// Remove `receiver`'s parcel `id` and save `player`, who took what it
/// held, in one transaction. Whether it was still there; if not, nothing
/// is saved.
pub fn claim(receiver: i32, id: i64, player: &Player) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let removed = diesel::delete(
            parcels::table
                .filter(parcels::id.eq(id))
                .filter(parcels::receiver_id.eq(receiver)),
        )
        .execute(conn)?;
        if removed == 0 {
            return Ok(false);
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}

/// Throw away `receiver`'s parcel `id`. Whether it was there.
pub fn delete(receiver: i32, id: i64) -> crate::Result<bool> {
    let removed = diesel::delete(
        parcels::table
            .filter(parcels::id.eq(id))
            .filter(parcels::receiver_id.eq(receiver)),
    )
    .execute(&mut crate::connection()?)?;
    Ok(removed > 0)
}

/// Whether `receiver` has parcels they weren't told of, and if so whether
/// one is a quick delivery; all of them count as told from now on.
pub fn announce(receiver: i32, now: SystemTime) -> crate::Result<Option<bool>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let untold = parcels::table
            .filter(parcels::receiver_id.eq(receiver))
            .filter(parcels::announced.eq(false))
            .filter(parcels::sent_at.ge(cutoff(now)));
        let quick: Option<bool> = untold
            .clone()
            .select(parcels::quick)
            .order(parcels::quick.desc())
            .first(conn)
            .optional()?;
        if quick.is_some() {
            diesel::update(untold)
                .set(parcels::announced.eq(true))
                .execute(conn)?;
        }
        Ok(quick)
    })?)
}
