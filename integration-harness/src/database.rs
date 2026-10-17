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
    grant_stack(character_id, inventory_type, slot, item_id, 1).await
}

/// Put `quantity` of `item_id` in slot `slot` of the logged-out character
/// `character_id`'s inventory `inventory_type`.
pub async fn grant_stack(
    character_id: i32,
    inventory_type: i16,
    slot: i16,
    item_id: i32,
    quantity: i16,
) -> Result<(), HarnessError> {
    change_one_row("grant item", move |conn| {
        diesel::sql_query(
            "INSERT INTO inventory_items (character_id, inventory_type, slot, item_id, quantity, owner, flags) \
             VALUES ($1, $2, $3, $4, $5, '', 0)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(inventory_type)
        .bind::<SmallInt, _>(slot)
        .bind::<Integer, _>(item_id)
        .bind::<SmallInt, _>(quantity)
        .execute(conn)
    })
    .await
}

/// Turn the item in slot `slot` of the logged-out character `character_id`'s
/// inventory `inventory_type` into `item_id`, keeping its stats (e.g. a
/// beginner's sword into a claw).
pub async fn replace_item(
    character_id: i32,
    inventory_type: i16,
    slot: i16,
    item_id: i32,
) -> Result<(), HarnessError> {
    change_one_row("replace item", move |conn| {
        diesel::sql_query(
            "UPDATE inventory_items SET item_id = $4 \
             WHERE character_id = $1 AND inventory_type = $2 AND slot = $3",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(inventory_type)
        .bind::<SmallInt, _>(slot)
        .bind::<Integer, _>(item_id)
        .execute(conn)
    })
    .await
}

/// Make the logged-out character `character_id` a level `level` `job`
/// with `mp` MP of `mp`.
pub async fn set_job(character_id: i32, job: i16, level: i16, mp: i16) -> Result<(), HarnessError> {
    change_one_row("set job", move |conn| {
        diesel::sql_query(
            "UPDATE characters SET job = $2, level = $3, mp = $4, maxmp = $4 WHERE id = $1",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(job)
        .bind::<SmallInt, _>(level)
        .bind::<SmallInt, _>(mp)
        .execute(conn)
    })
    .await
}

/// Set the logged-out character `character_id`'s HP and max HP; 0 HP
/// leaves them dead.
pub async fn set_hp(character_id: i32, hp: i16, max_hp: i16) -> Result<(), HarnessError> {
    change_one_row("set hp", move |conn| {
        diesel::sql_query("UPDATE characters SET hp = $2, maxhp = $3 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(hp)
            .bind::<SmallInt, _>(max_hp)
            .execute(conn)
    })
    .await
}

/// Set the logged-out character `character_id`'s STR, e.g. to hit hard.
pub async fn set_str(character_id: i32, str: i16) -> Result<(), HarnessError> {
    change_one_row("set str", move |conn| {
        diesel::sql_query("UPDATE characters SET stre = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(str)
            .execute(conn)
    })
    .await
}

/// Give the logged-out character `character_id` `meso` mesos.
pub async fn set_meso(character_id: i32, meso: i32) -> Result<(), HarnessError> {
    change_one_row("set meso", move |conn| {
        diesel::sql_query("UPDATE characters SET meso = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<Integer, _>(meso)
            .execute(conn)
    })
    .await
}

/// Teach the logged-out character `character_id` skill `skill_id` at
/// `level`.
pub async fn grant_skill(character_id: i32, skill_id: i32, level: i32) -> Result<(), HarnessError> {
    change_one_row("grant skill", move |conn| {
        diesel::sql_query(
            "INSERT INTO skills (character_id, skill_id, level, master_level) VALUES ($1, $2, $3, 0)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(skill_id)
        .bind::<Integer, _>(level)
        .execute(conn)
    })
    .await
}

/// Make the fame `giver_id` gave look `days` days older, as though that
/// many days had passed since.
pub async fn backdate_fame(giver_id: i32, days: i32) -> Result<(), HarnessError> {
    change_one_row("backdate fame", move |conn| {
        diesel::sql_query(
            "UPDATE fame_log SET given_at = given_at - make_interval(days => $2) WHERE giver_id = $1",
        )
        .bind::<Integer, _>(giver_id)
        .bind::<Integer, _>(days)
        .execute(conn)
    })
    .await
}

async fn change_one_row(
    phase: &'static str,
    query: impl FnOnce(&mut PgConnection) -> QueryResult<usize> + Send + 'static,
) -> Result<(), HarnessError> {
    let fixture = |message: String| HarnessError::Fixture { phase, message };
    let rows = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        query(&mut conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    if rows != 1 {
        return Err(fixture(format!("{rows} rows changed")));
    }
    Ok(())
}
