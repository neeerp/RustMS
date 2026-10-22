//! Booked weddings (report b066): one per couple, until the wedding or a
//! no-show ends it.

use crate::schema::{marriages, wedding_reservations};
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use game::marriage::ENGAGED;
use game::wedding::{next_slot, Reservation, Venue, SLOT};
use std::time::{Duration, SystemTime};

type Row = (i32, i16, bool, i16, SystemTime);

fn columns() -> (
    wedding_reservations::marriage_id,
    wedding_reservations::venue,
    wedding_reservations::premium,
    wedding_reservations::channel,
    wedding_reservations::starts_at,
) {
    (
        wedding_reservations::marriage_id,
        wedding_reservations::venue,
        wedding_reservations::premium,
        wedding_reservations::channel,
        wedding_reservations::starts_at,
    )
}

fn reservation_of(row: Row) -> Option<Reservation> {
    let (marriage_id, venue, premium, channel, starts_at) = row;
    Some(Reservation {
        marriage_id,
        venue: Venue::from_i16(venue)?,
        premium,
        channel: u8::try_from(channel).ok()?,
        starts_at,
    })
}

/// Lock the couple `marriage_id`'s row for the transaction; its status, if
/// the couple is still there. Booking and breaking off both take it first.
pub(crate) fn lock_couple(conn: &mut PgConnection, marriage_id: i32) -> QueryResult<Option<i16>> {
    marriages::table
        .find(marriage_id)
        .select(marriages::status)
        .for_update()
        .first(conn)
        .optional()
}

/// The couple's wedding, unless they let it pass by `now`, which ends it.
pub(crate) fn current_in(
    conn: &mut PgConnection,
    marriage_id: i32,
    now: SystemTime,
) -> QueryResult<Option<Reservation>> {
    let row = wedding_reservations::table
        .find(marriage_id)
        .select(columns())
        .first::<Row>(conn)
        .optional()?;
    match row.and_then(reservation_of) {
        Some(reservation) if Reservation::over(&reservation, now) => {
            diesel::delete(wedding_reservations::table.find(marriage_id)).execute(conn)?;
            Ok(None)
        }
        reservation => Ok(reservation),
    }
}

/// The couple `marriage_id`'s booked wedding, if it has not passed by
/// `now`.
pub fn current(marriage_id: i32, now: SystemTime) -> crate::Result<Option<Reservation>> {
    let mut conn = crate::connection()?;
    Ok(current_in(&mut conn, marriage_id, now)?)
}

/// What came of a booking.
#[derive(Debug, PartialEq, Eq)]
pub enum Booking {
    Booked(Reservation),
    /// The couple has a wedding booked already.
    AlreadyBooked(Reservation),
    /// The couple broke off, or married, meanwhile.
    NotEngaged,
}

/// Book the couple `marriage_id` a wedding at `venue` on `channel`: the
/// next free hour at least `lead` from `now` (`game::wedding::next_slot`).
/// Two couples taking the same hour at once: the later takes the next.
pub fn book(
    marriage_id: i32,
    venue: Venue,
    premium: bool,
    channel: u8,
    now: SystemTime,
    lead: Duration,
) -> crate::Result<Booking> {
    let mut conn = crate::connection()?;
    let mut attempts = 0;
    loop {
        let booked = conn.transaction(|conn| -> QueryResult<Booking> {
            if lock_couple(conn, marriage_id)? != Some(ENGAGED) {
                return Ok(Booking::NotEngaged);
            }
            if let Some(reservation) = current_in(conn, marriage_id, now)? {
                return Ok(Booking::AlreadyBooked(reservation));
            }
            let last = wedding_reservations::table
                .filter(wedding_reservations::channel.eq(i16::from(channel)))
                .filter(wedding_reservations::venue.eq(venue as i16))
                .select(max(wedding_reservations::starts_at))
                .first::<Option<SystemTime>>(conn)?;
            let reservation = Reservation {
                marriage_id,
                venue,
                premium,
                channel,
                starts_at: next_slot(now, lead, SLOT, last),
            };
            diesel::insert_into(wedding_reservations::table)
                .values((
                    wedding_reservations::marriage_id.eq(marriage_id),
                    wedding_reservations::venue.eq(venue as i16),
                    wedding_reservations::premium.eq(premium),
                    wedding_reservations::channel.eq(i16::from(channel)),
                    wedding_reservations::starts_at.eq(reservation.starts_at),
                ))
                .execute(conn)?;
            Ok(Booking::Booked(reservation))
        });
        match booked {
            Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _))
                if attempts < 3 =>
            {
                attempts += 1;
            }
            other => return Ok(other?),
        }
    }
}

/// Drop the couple's booking; whether there was one.
pub fn cancel(marriage_id: i32) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    let deleted =
        diesel::delete(wedding_reservations::table.find(marriage_id)).execute(&mut conn)?;
    Ok(deleted > 0)
}

/// Drop every booking whose couple let it pass by `now`; how many went.
pub fn drop_over(now: SystemTime) -> crate::Result<usize> {
    let mut conn = crate::connection()?;
    let cutoff = now - game::wedding::NO_SHOW;
    Ok(diesel::delete(
        wedding_reservations::table.filter(wedding_reservations::starts_at.le(cutoff)),
    )
    .execute(&mut conn)?)
}
