//! Crush and friendship rings (report b061): one row per ring of a pair,
//! keyed by the ring's cash serial.

use crate::schema::{characters, rings};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use game::inventory::{Inventory, InventoryType};
use game::ring::{CoupleRing, RingKind};

fn load(conn: &mut PgConnection, ids: &[i64]) -> QueryResult<Vec<CoupleRing>> {
    let rows: Vec<(i64, i64, i32, String, i32)> = rings::table
        .inner_join(characters::table)
        .filter(rings::id.eq_any(ids))
        .select((
            rings::id,
            rings::partner_ring_id,
            rings::partner_id,
            characters::name,
            rings::item_id,
        ))
        .order(rings::id)
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(
            |(id, partner_ring_id, partner_id, partner_name, item_id)| CoupleRing {
                id,
                partner_ring_id,
                partner_id,
                partner_name,
                item_id,
            },
        )
        .collect())
}

/// The pairs of the crush and friendship rings in `inventory`, worn or not.
pub(crate) fn carried(
    conn: &mut PgConnection,
    inventory: &Inventory,
) -> QueryResult<Vec<CoupleRing>> {
    let ids: Vec<i64> = [InventoryType::Equip, InventoryType::Equipped]
        .into_iter()
        .flat_map(|kind| inventory.items(kind))
        .filter(|(_, item)| {
            matches!(
                game::ring::kind(item.item_id),
                Some(RingKind::Crush | RingKind::Friendship)
            )
        })
        .filter_map(|(_, item)| item.cash_serial)
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    load(conn, &ids)
}

/// The ring with cash serial `id`, if it is one of a pair.
pub fn find(id: i64) -> crate::Result<Option<CoupleRing>> {
    let mut conn = crate::connection()?;
    Ok(load(&mut conn, &[id])?.pop())
}

/// Record a pair: ring `id` of `owner_id`, and ring `partner_ring_id` of
/// `partner_id`.
pub(crate) fn insert_pair(
    conn: &mut PgConnection,
    (id, owner_id): (i64, i32),
    (partner_ring_id, partner_id): (i64, i32),
    item_id: i32,
) -> QueryResult<()> {
    diesel::insert_into(rings::table)
        .values(&[
            (
                rings::id.eq(id),
                rings::partner_ring_id.eq(partner_ring_id),
                rings::partner_id.eq(partner_id),
                rings::item_id.eq(item_id),
            ),
            (
                rings::id.eq(partner_ring_id),
                rings::partner_ring_id.eq(id),
                rings::partner_id.eq(owner_id),
                rings::item_id.eq(item_id),
            ),
        ])
        .execute(conn)?;
    Ok(())
}
