//! The harness stack's database, for state a test can't reach through the
//! game, such as an account's NX.

use crate::HarnessError;
use diesel::prelude::*;
use diesel::sql_types::{Integer, SmallInt, Text};

const DEFAULT_DATABASE_URL: &str = "postgres://maplestory:pass@127.0.0.1:15432/maplestory_test";

/// `HARNESS_DATABASE_URL`, which `harnessctl test` sets.
pub fn database_url() -> String {
    std::env::var("HARNESS_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
}

/// Give the account named `user_name` `credit` NX Credit.
pub async fn set_nx_credit(user_name: &str, credit: i32) -> Result<(), HarnessError> {
    let user_name = user_name.to_string();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "set NX credit",
        message,
    };
    let updated = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query("UPDATE accounts SET nx_credit = $1 WHERE user_name = $2")
            .bind::<Integer, _>(credit)
            .bind::<Text, _>(user_name)
            .execute(&mut conn)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    if updated == 0 {
        return Err(fixture("no such account".to_string()));
    }
    Ok(())
}

/// Put `item_id` in slot `slot` of the logged-out character `character_id`'s
/// inventory `inventory_type`.
pub async fn grant_item(
    character_id: i32,
    inventory_type: i16,
    slot: i16,
    item_id: i32,
) -> Result<(), HarnessError> {
    let fixture = |message: String| HarnessError::Fixture {
        phase: "grant item",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "INSERT INTO inventory_items (character_id, inventory_type, slot, item_id, quantity, owner, flags) \
             VALUES ($1, $2, $3, $4, 1, '', 0)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(inventory_type)
        .bind::<SmallInt, _>(slot)
        .bind::<Integer, _>(item_id)
        .execute(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(())
}
