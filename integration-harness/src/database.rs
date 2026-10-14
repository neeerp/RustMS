//! The harness stack's database, for state a test can't reach through the
//! game, such as an account's NX.

use crate::HarnessError;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};

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
