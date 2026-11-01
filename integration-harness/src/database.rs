//! The harness stack's database, for state a test can't reach through the
//! game, such as an account's NX.

use crate::HarnessError;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Integer, Nullable, SmallInt, Text};

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

/// Set the NX Prepaid of the account named `user_name`.
pub async fn set_nx_prepaid(user_name: &str, prepaid: i32) -> Result<(), HarnessError> {
    let user_name = user_name.to_string();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "set NX Prepaid",
        message,
    };
    let updated = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query("UPDATE accounts SET nx_prepaid = $1 WHERE user_name = $2")
            .bind::<Integer, _>(prepaid)
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

/// Give the account named `user_name` `slots` character slots.
pub async fn set_character_slots(user_name: &str, slots: i16) -> Result<(), HarnessError> {
    let user_name = user_name.to_string();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "set character slots",
        message,
    };
    let updated = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query("UPDATE accounts SET character_slots = $1 WHERE user_name = $2")
            .bind::<SmallInt, _>(slots)
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

/// Make the account named `user_name` a GM of `level`, or not with 0.
pub async fn set_gm_level(user_name: &str, level: i16) -> Result<(), HarnessError> {
    let user_name = user_name.to_string();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "set GM level",
        message,
    };
    let updated = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query("UPDATE accounts SET gm_level = $1 WHERE user_name = $2")
            .bind::<SmallInt, _>(level)
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

/// Flag `user_name`'s account as the client's admin account, or not.
pub async fn set_test_admin(user_name: &str, on: bool) -> Result<(), HarnessError> {
    let user_name = user_name.to_string();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "set test admin",
        message,
    };
    let updated = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query("UPDATE accounts SET test_admin = $1 WHERE user_name = $2")
            .bind::<Bool, _>(on)
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

/// Rename the character `character_id` behind the game's back.
pub async fn rename_character(character_id: i32, name: &str) -> Result<(), HarnessError> {
    let name = name.to_string();
    change_one_row("rename character", move |conn| {
        diesel::sql_query("UPDATE characters SET name = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<Text, _>(name)
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

/// Give the logged-out character `character_id` `points` Mu Lung Dojo
/// training points.
pub async fn set_dojo_points(character_id: i32, points: i32) -> Result<(), HarnessError> {
    change_one_row("set Dojo points", move |conn| {
        diesel::sql_query(
            "INSERT INTO dojo_records (character_id, points) VALUES ($1, $2) \
             ON CONFLICT (character_id) DO UPDATE SET points = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(points)
        .execute(conn)
    })
    .await
}

/// Set the logged-out character `character_id`'s buddy list capacity.
pub async fn set_buddy_capacity(character_id: i32, capacity: i16) -> Result<(), HarnessError> {
    change_one_row("set buddy capacity", move |conn| {
        diesel::sql_query(
            "INSERT INTO buddy_capacities (character_id, capacity) VALUES ($1, $2) \
             ON CONFLICT (character_id) DO UPDATE SET capacity = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(capacity)
        .execute(conn)
    })
    .await
}

/// Give the logged-out character `character_id` `points` Battle Arena
/// points.
pub async fn set_ariant_points(character_id: i32, points: i32) -> Result<(), HarnessError> {
    change_one_row("set Ariant points", move |conn| {
        diesel::sql_query(
            "INSERT INTO ariant_records (character_id, points) VALUES ($1, $2) \
             ON CONFLICT (character_id) DO UPDATE SET points = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(points)
        .execute(conn)
    })
    .await
}

/// Mark So Gong's first-time trial beaten for the logged-out character
/// `character_id`, so he takes them on into the Dojo.
pub async fn set_dojo_trial_done(character_id: i32) -> Result<(), HarnessError> {
    change_one_row("set Dojo trial done", move |conn| {
        diesel::sql_query(
            "INSERT INTO dojo_records (character_id, tutorial_done) VALUES ($1, TRUE) \
             ON CONFLICT (character_id) DO UPDATE SET tutorial_done = TRUE",
        )
        .bind::<Integer, _>(character_id)
        .execute(conn)
    })
    .await
}

/// Put the pet `item_id` named `name`, with cash serial `serial`, in slot
/// `slot` of the logged-out character `character_id`'s Cash inventory.
pub async fn grant_pet(
    character_id: i32,
    slot: i16,
    item_id: i32,
    serial: i64,
    name: &str,
) -> Result<(), HarnessError> {
    let name = name.to_string();
    change_one_row("grant pet", move |conn| {
        diesel::sql_query(
            "WITH pet AS (INSERT INTO pets (serial, name) VALUES ($4, $5)) \
             INSERT INTO inventory_items (character_id, inventory_type, slot, item_id, quantity, owner, flags, cash_serial) \
             VALUES ($1, 5, $2, $3, 1, '', 0, $4)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(slot)
        .bind::<Integer, _>(item_id)
        .bind::<BigInt, _>(serial)
        .bind::<Text, _>(name)
        .execute(conn)
    })
    .await
}

/// Dress the logged-out character `character_id` in `item_id` at
/// `position` (negative), with no stats of its own.
pub async fn wear_equip(
    character_id: i32,
    position: i16,
    item_id: i32,
) -> Result<(), HarnessError> {
    change_one_row("wear equip", move |conn| {
        diesel::sql_query(
            "WITH item AS (INSERT INTO inventory_items (character_id, inventory_type, slot, item_id, quantity, owner, flags) \
             VALUES ($1, -1, $2, $3, 1, '', 0) RETURNING id) \
             INSERT INTO equipment_stats (item_row_id) SELECT id FROM item",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(position)
        .bind::<Integer, _>(item_id)
        .execute(conn)
    })
    .await
}

/// Put the equip `item_id` with `upgrade_slots` slots in slot `slot` of the
/// logged-out character `character_id`'s Equip tab.
pub async fn stow_equip(
    character_id: i32,
    slot: i16,
    item_id: i32,
    upgrade_slots: i16,
) -> Result<(), HarnessError> {
    change_one_row("stow equip", move |conn| {
        diesel::sql_query(
            "WITH item AS (INSERT INTO inventory_items (character_id, inventory_type, slot, item_id, quantity, owner, flags) \
             VALUES ($1, 1, $2, $3, 1, '', 0) RETURNING id) \
             INSERT INTO equipment_stats (item_row_id, upgrade_slots) SELECT id, $4 FROM item",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(slot)
        .bind::<Integer, _>(item_id)
        .bind::<SmallInt, _>(upgrade_slots)
        .execute(conn)
    })
    .await
}

/// Give the equip in slot `slot` of the logged-out character
/// `character_id`'s Equip tab `str` STR.
pub async fn set_equip_str(character_id: i32, slot: i16, str: i16) -> Result<(), HarnessError> {
    change_one_row("set equip str", move |conn| {
        diesel::sql_query(
            "UPDATE equipment_stats SET stre = $3 WHERE item_row_id = \
             (SELECT id FROM inventory_items WHERE character_id = $1 AND inventory_type = 1 AND slot = $2)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(slot)
        .bind::<SmallInt, _>(str)
        .execute(conn)
    })
    .await
}

/// The upgrade slots left, scrolls passed (level) and hammers of the equip in
/// slot `slot` of character `character_id`'s Equip tab.
pub async fn equip_upgrades(
    character_id: i32,
    slot: i16,
) -> Result<Option<(i16, i16, i32)>, HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = SmallInt)]
        upgrade_slots: i16,
        #[diesel(sql_type = SmallInt)]
        level: i16,
        #[diesel(sql_type = Integer)]
        vicious: i32,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read an equip",
        message,
    };
    let rows = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT e.upgrade_slots, e.level, e.vicious FROM inventory_items i \
             JOIN equipment_stats e ON e.item_row_id = i.id \
             WHERE i.character_id = $1 AND i.inventory_type = 1 AND i.slot = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(slot)
        .load::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(rows
        .first()
        .map(|row| (row.upgrade_slots, row.level, row.vicious)))
}

/// Dress the logged-out character `character_id` in the cash equip
/// `item_id`, with cash serial `serial`, at `position` (negative), such as
/// a pet's equipment.
pub async fn wear_cash_equip(
    character_id: i32,
    position: i16,
    item_id: i32,
    serial: i64,
) -> Result<(), HarnessError> {
    change_one_row("wear cash equip", move |conn| {
        diesel::sql_query(
            "WITH item AS (INSERT INTO inventory_items (character_id, inventory_type, slot, item_id, quantity, owner, flags, cash_serial) \
             VALUES ($1, -1, $2, $3, 1, '', 0, $4) RETURNING id) \
             INSERT INTO equipment_stats (item_row_id) SELECT id FROM item",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(position)
        .bind::<Integer, _>(item_id)
        .bind::<BigInt, _>(serial)
        .execute(conn)
    })
    .await
}

/// Set the fullness of the pet `serial`, whose owner is logged out.
pub async fn set_pet_fullness(serial: i64, fullness: i16) -> Result<(), HarnessError> {
    change_one_row("set pet fullness", move |conn| {
        diesel::sql_query("UPDATE pets SET fullness = $2 WHERE serial = $1")
            .bind::<BigInt, _>(serial)
            .bind::<SmallInt, _>(fullness)
            .execute(conn)
    })
    .await
}

/// Set the closeness of the pet `serial`, whose owner is logged out.
pub async fn set_pet_closeness(serial: i64, closeness: i16) -> Result<(), HarnessError> {
    change_one_row("set pet closeness", move |conn| {
        diesel::sql_query("UPDATE pets SET closeness = $2 WHERE serial = $1")
            .bind::<BigInt, _>(serial)
            .bind::<SmallInt, _>(closeness)
            .execute(conn)
    })
    .await
}

/// The skill bits and attribute saved for the pet `serial`.
pub async fn pet_skills(serial: i64) -> Result<(i32, i16), HarnessError> {
    #[derive(QueryableByName)]
    struct Skills {
        #[diesel(sql_type = Integer)]
        skill: i32,
        #[diesel(sql_type = SmallInt)]
        attribute: i16,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read pet skills",
        message,
    };
    let skills = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query("SELECT skill, attribute FROM pets WHERE serial = $1")
            .bind::<BigInt, _>(serial)
            .get_result::<Skills>(&mut conn)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok((skills.skill, skills.attribute))
}

/// The cash serial and saved name of the pet `item_id` that `character_id`
/// carries.
pub async fn carried_pet(character_id: i32, item_id: i32) -> Result<(i64, String), HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = BigInt)]
        serial: i64,
        #[diesel(sql_type = Text)]
        name: String,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read a carried pet",
        message,
    };
    let row = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT p.serial, p.name FROM inventory_items i JOIN pets p ON p.serial = i.cash_serial \
             WHERE i.character_id = $1 AND i.item_id = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(item_id)
        .get_result::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok((row.serial, row.name))
}

/// Date the pet `serial`, whose owner is logged out, to dry up `seconds`
/// from now; in the past if negative.
pub async fn date_pet(serial: i64, seconds: i32) -> Result<(), HarnessError> {
    change_one_row("date pet", move |conn| {
        diesel::sql_query(
            "UPDATE inventory_items SET expires_at = now() + make_interval(secs => $2) \
             WHERE cash_serial = $1",
        )
        .bind::<BigInt, _>(serial)
        .bind::<Integer, _>(seconds)
        .execute(conn)
    })
    .await
}

/// Date the logged-out character `character_id`'s `item_id` to expire
/// `seconds` from now; in the past if negative. The whole minutes it had
/// left, if it was dated.
pub async fn date_item(
    character_id: i32,
    item_id: i32,
    seconds: i32,
) -> Result<Option<i64>, HarnessError> {
    #[derive(QueryableByName)]
    struct Left {
        #[diesel(sql_type = Nullable<BigInt>)]
        minutes: Option<i64>,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "date an item",
        message,
    };
    let left = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "WITH old AS (SELECT id, expires_at FROM inventory_items \
             WHERE character_id = $1 AND item_id = $2) \
             UPDATE inventory_items i SET expires_at = now() + make_interval(secs => $3) \
             FROM old WHERE i.id = old.id \
             RETURNING floor(extract(epoch FROM old.expires_at - now()::timestamp) / 60)::bigint AS minutes",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(item_id)
        .bind::<Integer, _>(seconds)
        .get_result::<Left>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(left.minutes)
}

/// End the character `character_id`'s jailing `seconds` from now.
pub async fn date_jail(character_id: i32, seconds: i32) -> Result<(), HarnessError> {
    change_one_row("date jail", move |conn| {
        diesel::sql_query(
            "UPDATE jails SET until = now() + make_interval(secs => $2) WHERE character_id = $1",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(seconds)
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

/// Set the weapon attack of the logged-out character `character_id`'s
/// weapon, e.g. to hit very hard.
pub async fn set_weapon_attack(character_id: i32, watk: i16) -> Result<(), HarnessError> {
    change_one_row("set weapon attack", move |conn| {
        diesel::sql_query(
            "UPDATE equipment_stats SET watk = $2 WHERE item_row_id = \
             (SELECT id FROM inventory_items \
              WHERE character_id = $1 AND inventory_type = -1 AND slot = -11)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(watk)
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

/// Set the logged-out character `character_id`'s DEX, e.g. to hit often.
pub async fn set_dex(character_id: i32, dex: i16) -> Result<(), HarnessError> {
    change_one_row("set dex", move |conn| {
        diesel::sql_query("UPDATE characters SET dex = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(dex)
            .execute(conn)
    })
    .await
}

/// Set the logged-out character `character_id`'s LUK, e.g. to meet a
/// dagger's requirements.
pub async fn set_luk(character_id: i32, luk: i16) -> Result<(), HarnessError> {
    change_one_row("set luk", move |conn| {
        diesel::sql_query("UPDATE characters SET luk = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(luk)
            .execute(conn)
    })
    .await
}

/// Give the logged-out character `character_id` `meso` mesos.
pub async fn set_sp(character_id: i32, sp: i16) -> Result<(), HarnessError> {
    change_one_row("set sp", move |conn| {
        diesel::sql_query("UPDATE characters SET sp = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(sp)
            .execute(conn)
    })
    .await
}

/// Make the logged-out character `character_id` a man (0) or a woman (1).
pub async fn set_gender(character_id: i32, gender: i16) -> Result<(), HarnessError> {
    change_one_row("set gender", move |conn| {
        diesel::sql_query("UPDATE characters SET gender = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(gender)
            .execute(conn)
    })
    .await
}

pub async fn set_meso(character_id: i32, meso: i32) -> Result<(), HarnessError> {
    change_one_row("set meso", move |conn| {
        diesel::sql_query("UPDATE characters SET meso = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<Integer, _>(meso)
            .execute(conn)
    })
    .await
}

/// Set the logged-out character `character_id`'s fame.
pub async fn set_fame(character_id: i32, fame: i16) -> Result<(), HarnessError> {
    change_one_row("set fame", move |conn| {
        diesel::sql_query("UPDATE characters SET fame = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(fame)
            .execute(conn)
    })
    .await
}

/// Put `level` copies of card `card_id` in the logged-out character
/// `character_id`'s Monster Book.
pub async fn add_book_card(
    character_id: i32,
    card_id: i32,
    level: i16,
) -> Result<(), HarnessError> {
    change_one_row("add book card", move |conn| {
        diesel::sql_query(
            "INSERT INTO monster_book_cards (character_id, card_id, level) VALUES ($1, $2, $3)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(card_id)
        .bind::<SmallInt, _>(level)
        .execute(conn)
    })
    .await
}

/// Give the logged-out character `character_id` `exp` EXP into their level.
pub async fn set_exp(character_id: i32, exp: i32) -> Result<(), HarnessError> {
    change_one_row("set exp", move |conn| {
        diesel::sql_query("UPDATE characters SET exp = $2 WHERE id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<Integer, _>(exp)
            .execute(conn)
    })
    .await
}

/// Give `character_id`, who is in a family, `reputation` Rep to spend.
pub async fn set_family_reputation(character_id: i32, reputation: i32) -> Result<(), HarnessError> {
    change_one_row("set family reputation", move |conn| {
        diesel::sql_query("UPDATE family_members SET reputation = $2 WHERE character_id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<Integer, _>(reputation)
            .execute(conn)
    })
    .await
}

/// Make `junior` the junior of `senior`, both logged out and in no family.
pub async fn join_family(senior: i32, junior: i32) -> Result<(), HarnessError> {
    change_one_row("seat the senior", move |conn| {
        diesel::sql_query("INSERT INTO family_members (character_id) VALUES ($1)")
            .bind::<Integer, _>(senior)
            .execute(conn)
    })
    .await?;
    change_one_row("seat the junior", move |conn| {
        diesel::sql_query("INSERT INTO family_members (character_id, senior_id) VALUES ($1, $2)")
            .bind::<Integer, _>(junior)
            .bind::<Integer, _>(senior)
            .execute(conn)
    })
    .await
}

/// Give `character_id`, who is in a family, `reputation` Rep to spend and
/// `total` earned in all.
pub async fn set_family_reputations(
    character_id: i32,
    reputation: i32,
    total: i32,
) -> Result<(), HarnessError> {
    change_one_row("set family reputations", move |conn| {
        diesel::sql_query(
            "UPDATE family_members SET reputation = $2, total_reputation = $3 \
             WHERE character_id = $1",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(reputation)
        .bind::<Integer, _>(total)
        .execute(conn)
    })
    .await
}

/// Start quest `quest_id` for the logged-out character `character_id`.
pub async fn start_quest(character_id: i32, quest_id: i16) -> Result<(), HarnessError> {
    change_one_row("start quest", move |conn| {
        diesel::sql_query(
            "INSERT INTO character_quests (character_id, quest_id, status) VALUES ($1, $2, 1)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(quest_id)
        .execute(conn)
    })
    .await
}

/// Give the logged-out character `character_id` quest `quest_id` in
/// `status` (1 started, 2 completed) with kill `progress`, replacing any
/// record of it already there.
pub async fn set_quest(
    character_id: i32,
    quest_id: i16,
    status: i16,
    progress: &str,
) -> Result<(), HarnessError> {
    let progress = progress.to_string();
    change_one_row("set quest", move |conn| {
        diesel::sql_query(
            "INSERT INTO character_quests (character_id, quest_id, status, completed_at, progress) \
             VALUES ($1, $2, $3, CASE WHEN $3 = 2 THEN now() END, $4) \
             ON CONFLICT (character_id, quest_id) DO UPDATE SET status = EXCLUDED.status, \
             completed_at = EXCLUDED.completed_at, progress = EXCLUDED.progress",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(quest_id)
        .bind::<SmallInt, _>(status)
        .bind::<Text, _>(progress)
        .execute(conn)
    })
    .await
}

/// Mark quests `first` to `last` completed for the logged-out character
/// `character_id`.
pub async fn complete_quests(character_id: i32, first: i16, last: i16) -> Result<(), HarnessError> {
    let fixture = |message: String| HarnessError::Fixture {
        phase: "complete quests",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "INSERT INTO character_quests (character_id, quest_id, status, completed_at, progress) \
             SELECT $1, q::smallint, 2, now(), '' FROM generate_series($2::int, $3::int) AS q",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(first)
        .bind::<SmallInt, _>(last)
        .execute(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(())
}

/// Set the logged-out character `character_id`'s record beside the quest
/// log for `quest_id` to `data` (`k=v;k=v`).
pub async fn set_quest_ex(
    character_id: i32,
    quest_id: i16,
    data: &str,
) -> Result<(), HarnessError> {
    let data = data.to_string();
    change_one_row("set quest record ex", move |conn| {
        diesel::sql_query(
            "INSERT INTO quest_ex_records (character_id, quest_id, data) VALUES ($1, $2, $3) \
             ON CONFLICT (character_id, quest_id) DO UPDATE SET data = EXCLUDED.data",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(quest_id)
        .bind::<Text, _>(data)
        .execute(conn)
    })
    .await
}

/// Record that the logged-out character `character_id` has explored
/// `map_id` toward the medal counted in quest record `record_id`.
pub async fn explore_map(
    character_id: i32,
    record_id: i16,
    map_id: i32,
) -> Result<(), HarnessError> {
    change_one_row("explore map", move |conn| {
        diesel::sql_query(
            "INSERT INTO explored_maps (character_id, record_id, map_id) VALUES ($1, $2, $3)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(record_id)
        .bind::<Integer, _>(map_id)
        .execute(conn)
    })
    .await
}

/// Make the logged-out character `character_id`'s completion of quest
/// `quest_id` look `minutes` minutes older.
pub async fn backdate_quest(
    character_id: i32,
    quest_id: i16,
    minutes: i32,
) -> Result<(), HarnessError> {
    change_one_row("backdate quest", move |conn| {
        diesel::sql_query(
            "UPDATE character_quests SET completed_at = completed_at - make_interval(mins => $3) \
             WHERE character_id = $1 AND quest_id = $2 AND status = 2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(quest_id)
        .bind::<Integer, _>(minutes)
        .execute(conn)
    })
    .await
}

/// The saved record of character `character_id`'s quest `quest_id` (kill
/// counts, info string or end date); None if it has none.
pub async fn quest_progress(
    character_id: i32,
    quest_id: i16,
) -> Result<Option<String>, HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        progress: String,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read a quest",
        message,
    };
    let rows = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT progress FROM character_quests WHERE character_id = $1 AND quest_id = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(quest_id)
        .load::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(rows.into_iter().next().map(|row| row.progress))
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

/// Put the logged-out character `character_id` in the guild named `guild`
/// as a plain member (rank 3), as an accepted invitation would.
pub async fn join_guild(character_id: i32, guild: &str) -> Result<(), HarnessError> {
    let guild = guild.to_owned();
    change_one_row("join guild", move |conn| {
        diesel::sql_query(
            "INSERT INTO guild_members (character_id, guild_id, rank) \
             SELECT $1, id, 3 FROM guilds WHERE name = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Text, _>(guild)
        .execute(conn)
    })
    .await
}

/// Found the guild `guild` in world 0 with `master` as its only member.
pub async fn found_guild(master: i32, guild: &str) -> Result<(), HarnessError> {
    let guild = guild.to_owned();
    change_one_row("found guild", move |conn| {
        diesel::sql_query(
            "WITH g AS (INSERT INTO guilds (world_id, name) VALUES (0, $2) RETURNING id) \
             INSERT INTO guild_members (character_id, guild_id, rank) SELECT $1, id, 1 FROM g",
        )
        .bind::<Integer, _>(master)
        .bind::<Text, _>(guild)
        .execute(conn)
    })
    .await
}

/// Give the guild named `guild` `points` and the emblem `(frame, frame
/// colour, mark, mark colour)`.
pub async fn rank_guild(
    guild: &str,
    points: i32,
    emblem: (i16, i16, i16, i16),
) -> Result<(), HarnessError> {
    let guild = guild.to_owned();
    change_one_row("rank guild", move |conn| {
        diesel::sql_query(
            "UPDATE guilds SET points = $2, mark_bg = $3, mark_bg_color = $4, mark = $5, \
             mark_color = $6 WHERE name = $1",
        )
        .bind::<Text, _>(guild)
        .bind::<Integer, _>(points)
        .bind::<SmallInt, _>(emblem.0)
        .bind::<SmallInt, _>(emblem.1)
        .bind::<SmallInt, _>(emblem.2)
        .bind::<SmallInt, _>(emblem.3)
        .execute(conn)
    })
    .await
}

/// Set the capacity of the guild named `guild`.
pub async fn set_guild_capacity(guild: &str, capacity: i16) -> Result<(), HarnessError> {
    let guild = guild.to_owned();
    change_one_row("set guild capacity", move |conn| {
        diesel::sql_query("UPDATE guilds SET capacity = $2 WHERE name = $1")
            .bind::<Text, _>(guild)
            .bind::<SmallInt, _>(capacity)
            .execute(conn)
    })
    .await
}

/// Give guild member `character_id` `rank`.
pub async fn set_guild_rank(character_id: i32, rank: i16) -> Result<(), HarnessError> {
    change_one_row("set guild rank", move |conn| {
        diesel::sql_query("UPDATE guild_members SET rank = $2 WHERE character_id = $1")
            .bind::<Integer, _>(character_id)
            .bind::<SmallInt, _>(rank)
            .execute(conn)
    })
    .await
}

/// Delete the guild named `guild`, and its members' rows with it, behind
/// the server's back.
pub async fn delete_guild(guild: &str) -> Result<(), HarnessError> {
    let guild = guild.to_owned();
    change_one_row("delete guild", move |conn| {
        diesel::sql_query("DELETE FROM guilds WHERE name = $1")
            .bind::<Text, _>(guild)
            .execute(conn)
    })
    .await
}

/// The capacity of the guild named `guild`, and the characters in it; None
/// if there is no such guild.
pub async fn guild_capacity_and_members(
    guild: &str,
) -> Result<Option<(i16, Vec<i32>)>, HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = SmallInt)]
        capacity: i16,
        #[diesel(sql_type = Nullable<Integer>)]
        character_id: Option<i32>,
    }
    let guild = guild.to_owned();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read a guild",
        message,
    };
    let rows = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT g.capacity, m.character_id FROM guilds g \
             LEFT JOIN guild_members m ON m.guild_id = g.id \
             WHERE g.name = $1 ORDER BY m.character_id",
        )
        .bind::<Text, _>(guild)
        .load::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(rows.first().map(|first| {
        (
            first.capacity,
            rows.iter().filter_map(|row| row.character_id).collect(),
        )
    }))
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

/// Give the logged-out character `character_id`'s mount `level`, `exp`
/// and `tiredness`.
pub async fn set_mount(
    character_id: i32,
    level: i32,
    exp: i32,
    tiredness: i32,
) -> Result<(), HarnessError> {
    change_one_row("set mount", move |conn| {
        diesel::sql_query(
            "INSERT INTO character_mounts (character_id, level, exp, tiredness) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (character_id) DO UPDATE SET level = $2, exp = $3, tiredness = $4",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(level)
        .bind::<Integer, _>(exp)
        .bind::<Integer, _>(tiredness)
        .execute(conn)
    })
    .await
}

/// Put `item_id` in the Cash Inventory of the account of the character
/// `character_id`, as a gift from `from`. The item's cash serial.
pub async fn gift_cash_item(
    character_id: i32,
    item_id: i32,
    from: &str,
) -> Result<i64, HarnessError> {
    #[derive(QueryableByName)]
    struct Serial {
        #[diesel(sql_type = BigInt)]
        id: i64,
    }
    let from = from.to_string();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "gift a cash item",
        message,
    };
    let serial = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "INSERT INTO cash_items (account_id, item_id, sn, quantity, gift_from) \
             SELECT accountid, $2, 0, 1, $3 FROM characters WHERE id = $1 RETURNING id",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(item_id)
        .bind::<Text, _>(from)
        .get_result::<Serial>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(serial.id)
}

/// Have the Cash Inventory item `serial` run out of time `seconds` from now.
pub async fn expire_cash_item_in(serial: i64, seconds: i32) -> Result<(), HarnessError> {
    let fixture = |message: String| HarnessError::Fixture {
        phase: "expire a cash item",
        message,
    };
    let updated = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "UPDATE cash_items SET expires_at = now() + make_interval(secs => $2) WHERE id = $1",
        )
        .bind::<BigInt, _>(serial)
        .bind::<Integer, _>(seconds)
        .execute(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    if updated == 0 {
        return Err(fixture("no such cash item".to_string()));
    }
    Ok(())
}

/// Whether the Cash Inventory item `serial` is still there.
pub async fn cash_item_exists(serial: i64) -> Result<bool, HarnessError> {
    #[derive(QueryableByName)]
    struct Found {
        #[diesel(sql_type = Bool)]
        found: bool,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "look for a cash item",
        message,
    };
    let found = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query("SELECT EXISTS (SELECT 1 FROM cash_items WHERE id = $1) AS found")
            .bind::<BigInt, _>(serial)
            .get_result::<Found>(&mut conn)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(found.found)
}

/// One row of a hired merchant: `bundles` of `quantity` of `item_id`, at
/// `price` a bundle.
pub struct MerchantRow {
    pub item_id: i32,
    pub bundles: i16,
    pub quantity: i16,
    pub price: i32,
}

/// Give the character `owner_id` a hired merchant on channel `channel_id`
/// (0-based) of `map_id`, in `state` (1 open, 2 closed `days_closed` days
/// ago), holding `meso` and `rows`, in place of any they had.
pub async fn place_merchant(
    owner_id: i32,
    channel_id: i16,
    map_id: i32,
    state: i16,
    days_closed: i32,
    meso: i32,
    rows: Vec<MerchantRow>,
) -> Result<(), HarnessError> {
    let fixture = |message: String| HarnessError::Fixture {
        phase: "place a merchant",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        conn.transaction(|conn| {
            diesel::sql_query("DELETE FROM hired_merchants WHERE owner_id = $1")
                .bind::<Integer, _>(owner_id)
                .execute(conn)?;
            // An hour past the whole days, so they count as whole.
            diesel::sql_query(
                "INSERT INTO hired_merchants \
                 (owner_id, channel_id, map_id, x, y, foothold, item_id, title, state, opened_at, closed_at, meso) \
                 VALUES ($1, $2, $3, 0, 0, 0, 5030000, 'Kept', $4, \
                 now() AT TIME ZONE 'utc' - make_interval(days => CASE WHEN $4 = 2 THEN $5 + 1 ELSE 0 END), \
                 CASE WHEN $4 = 2 THEN now() AT TIME ZONE 'utc' - make_interval(days => $5, hours => 1) END, $6)",
            )
            .bind::<Integer, _>(owner_id)
            .bind::<SmallInt, _>(channel_id)
            .bind::<Integer, _>(map_id)
            .bind::<SmallInt, _>(state)
            .bind::<Integer, _>(days_closed)
            .bind::<Integer, _>(meso)
            .execute(conn)?;
            for row in rows {
                diesel::sql_query(
                    "INSERT INTO hired_merchant_items (owner_id, bundles, price, item_id, quantity) \
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind::<Integer, _>(owner_id)
                .bind::<SmallInt, _>(row.bundles)
                .bind::<Integer, _>(row.price)
                .bind::<Integer, _>(row.item_id)
                .bind::<SmallInt, _>(row.quantity)
                .execute(conn)?;
            }
            QueryResult::Ok(())
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)
}

/// Whether the character `owner_id` has a hired merchant.
pub async fn has_merchant(owner_id: i32) -> Result<bool, HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Bool)]
        found: bool,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "look for a merchant",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT EXISTS (SELECT 1 FROM hired_merchants WHERE owner_id = $1) AS found",
        )
        .bind::<Integer, _>(owner_id)
        .get_result::<Row>(&mut conn)
        .map(|row| row.found)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)
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

/// The owner printed on the logged-out character `character_id`'s
/// `item_id`, and the whole hours left until it expires (`None` for never).
pub async fn item_owner_and_hours(
    character_id: i32,
    item_id: i32,
) -> Result<(String, Option<i64>), HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        owner: String,
        #[diesel(sql_type = Nullable<BigInt>)]
        hours: Option<i64>,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read an item's owner and date",
        message,
    };
    let row = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT owner, floor(extract(epoch FROM expires_at - now()::timestamp) / 3600)::bigint AS hours \
             FROM inventory_items WHERE character_id = $1 AND item_id = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(item_id)
        .get_result::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok((row.owner, row.hours))
}

/// Set the flags of the logged-out character `character_id`'s `item_id`.
pub async fn flag_item(character_id: i32, item_id: i32, flags: i16) -> Result<(), HarnessError> {
    change_one_row("flag item", move |conn| {
        diesel::sql_query(
            "UPDATE inventory_items SET flags = $3 WHERE character_id = $1 AND item_id = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(item_id)
        .bind::<SmallInt, _>(flags)
        .execute(conn)
    })
    .await
}

/// The flags of `character_id`'s `item_id`, and the whole hours it has
/// left if it is dated.
pub async fn item_flags_and_hours(
    character_id: i32,
    item_id: i32,
) -> Result<(i16, Option<i64>), HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = SmallInt)]
        flags: i16,
        #[diesel(sql_type = Nullable<BigInt>)]
        hours: Option<i64>,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read an item's flags and date",
        message,
    };
    let row = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT flags, floor(extract(epoch FROM expires_at - now()::timestamp) / 3600)::bigint AS hours \
             FROM inventory_items WHERE character_id = $1 AND item_id = $2",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(item_id)
        .get_result::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok((row.flags, row.hours))
}

/// Make the family entitlement working for `character_id` end in
/// `seconds`.
pub async fn end_family_buff_in(character_id: i32, seconds: i32) -> Result<(), HarnessError> {
    change_one_row("end family buff", move |conn| {
        diesel::sql_query(
            "UPDATE family_buffs SET until = (now() AT TIME ZONE 'utc') \
             + make_interval(secs => $2) WHERE character_id = $1",
        )
        .bind::<Integer, _>(character_id)
        .bind::<Integer, _>(seconds)
        .execute(conn)
    })
    .await
}

/// Engage the logged-out characters `groom_id` and `bride_id`, as if the
/// groom had proposed with the ring box `ring_box`.
pub async fn engage(groom_id: i32, bride_id: i32, ring_box: i32) -> Result<(), HarnessError> {
    change_one_row("engage", move |conn| {
        diesel::sql_query(
            "INSERT INTO marriages (groom_id, bride_id, status, item_id) VALUES ($1, $2, 1, $3)",
        )
        .bind::<Integer, _>(groom_id)
        .bind::<Integer, _>(bride_id)
        .bind::<Integer, _>(ring_box)
        .execute(conn)
    })
    .await
}

/// Marry the logged-out characters `groom_id` and `bride_id`, as if they
/// had proposed with the ring box `ring_box`.
pub async fn marry(groom_id: i32, bride_id: i32, ring_box: i32) -> Result<(), HarnessError> {
    change_one_row("marry", move |conn| {
        diesel::sql_query(
            "INSERT INTO marriages (groom_id, bride_id, status, item_id) VALUES ($1, $2, 3, $3)",
        )
        .bind::<Integer, _>(groom_id)
        .bind::<Integer, _>(bride_id)
        .bind::<Integer, _>(ring_box)
        .execute(conn)
    })
    .await
}

/// Book the wedding of the couple whose groom is `groom_id` at `venue` (0
/// Chapel, 1 Cathedral), starting `hours` from the last top of the hour;
/// the couple's id. It goes on a channel of the couple's own past the
/// harness's, so it never takes another booking's slot.
pub async fn book_wedding(groom_id: i32, venue: i16, hours: i32) -> Result<i32, HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Integer)]
        marriage_id: i32,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "book a wedding",
        message,
    };
    let row = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "INSERT INTO wedding_reservations (marriage_id, venue, premium, channel, starts_at) \
             SELECT id, $2, false, 100 + id % 100, date_trunc('hour', now()::timestamp) + make_interval(hours => $3) \
             FROM marriages WHERE groom_id = $1 RETURNING marriage_id",
        )
        .bind::<Integer, _>(groom_id)
        .bind::<SmallInt, _>(venue)
        .bind::<Integer, _>(hours)
        .get_result::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(row.marriage_id)
}

/// Put `guest_id` on the guest list of the couple `marriage_id`'s booked
/// wedding.
pub async fn add_wedding_guest(marriage_id: i32, guest_id: i32) -> Result<(), HarnessError> {
    change_one_row("invite a wedding guest", move |conn| {
        diesel::sql_query("INSERT INTO wedding_guests (marriage_id, guest_id) VALUES ($1, $2)")
            .bind::<Integer, _>(marriage_id)
            .bind::<Integer, _>(guest_id)
            .execute(conn)
    })
    .await
}

/// Move the couple `marriage_id`'s booked wedding to the zero-based
/// `channel`, starting a minute ago, so it may begin now.
pub async fn wedding_due(marriage_id: i32, channel: i16) -> Result<(), HarnessError> {
    change_one_row("make a wedding due", move |conn| {
        diesel::sql_query(
            "UPDATE wedding_reservations SET channel = $2, \
             starts_at = now()::timestamp - interval '1 minute' WHERE marriage_id = $1",
        )
        .bind::<Integer, _>(marriage_id)
        .bind::<SmallInt, _>(channel)
        .execute(conn)
    })
    .await
}

/// Make the couple `marriage_id`'s booked wedding a premium one.
pub async fn wedding_premium(marriage_id: i32) -> Result<(), HarnessError> {
    change_one_row("make a wedding premium", move |conn| {
        diesel::sql_query("UPDATE wedding_reservations SET premium = true WHERE marriage_id = $1")
            .bind::<Integer, _>(marriage_id)
            .execute(conn)
    })
    .await
}

/// The couple `marriage_id`'s status, and whether a wedding is booked for
/// them.
pub async fn couple_status(marriage_id: i32) -> Result<(i16, bool), HarnessError> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = SmallInt)]
        status: i16,
        #[diesel(sql_type = Bool)]
        booked: bool,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read a couple's status",
        message,
    };
    let row = tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT status, EXISTS (SELECT 1 FROM wedding_reservations WHERE marriage_id = $1) AS booked \
             FROM marriages WHERE id = $1",
        )
        .bind::<Integer, _>(marriage_id)
        .get_result::<Row>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok((row.status, row.booked))
}

/// What waits for `receiver_id`: the sender and item of each parcel, and
/// the sender of each note, oldest first.
pub async fn parcels_and_notes(
    receiver_id: i32,
) -> Result<(Vec<(String, Option<i32>)>, Vec<String>), HarnessError> {
    #[derive(QueryableByName)]
    struct Parcel {
        #[diesel(sql_type = Text)]
        sender: String,
        #[diesel(sql_type = Nullable<Integer>)]
        item_id: Option<i32>,
    }
    #[derive(QueryableByName)]
    struct Note {
        #[diesel(sql_type = Text)]
        sender: String,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read parcels and notes",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        let parcels = diesel::sql_query(
            "SELECT sender, item_id FROM parcels WHERE receiver_id = $1 ORDER BY id",
        )
        .bind::<Integer, _>(receiver_id)
        .load::<Parcel>(&mut conn)
        .map_err(|e| e.to_string())?;
        let notes =
            diesel::sql_query("SELECT sender FROM notes WHERE receiver_id = $1 ORDER BY id")
                .bind::<Integer, _>(receiver_id)
                .load::<Note>(&mut conn)
                .map_err(|e| e.to_string())?;
        Ok::<_, String>((
            parcels.into_iter().map(|p| (p.sender, p.item_id)).collect(),
            notes.into_iter().map(|n| n.sender).collect(),
        ))
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)
}

/// Fill the Duey inbox of `receiver_id` with `count` parcels of a meso
/// each.
pub async fn fill_parcels(receiver_id: i32, count: i32) -> Result<(), HarnessError> {
    let fixture = |message: String| HarnessError::Fixture {
        phase: "fill a Duey inbox",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "INSERT INTO parcels \
             (receiver_id, sender, mesos, quick, sent_at, announced, quantity, owner, flags) \
             SELECT $1, 'filler', 1, false, now()::timestamp, true, 0, '', 0 \
             FROM generate_series(1, $2)",
        )
        .bind::<Integer, _>(receiver_id)
        .bind::<Integer, _>(count)
        .execute(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(())
}

/// The notes waiting for `receiver_id`, oldest first: each one's id,
/// sender, flag and text.
pub async fn notes_waiting(
    receiver_id: i32,
) -> Result<Vec<(i32, String, i16, String)>, HarnessError> {
    #[derive(QueryableByName)]
    struct Note {
        #[diesel(sql_type = Integer)]
        id: i32,
        #[diesel(sql_type = Text)]
        sender: String,
        #[diesel(sql_type = SmallInt)]
        flag: i16,
        #[diesel(sql_type = Text)]
        message: String,
    }
    let fixture = |message: String| HarnessError::Fixture {
        phase: "read notes",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "SELECT id, sender, flag, message FROM notes WHERE receiver_id = $1 ORDER BY id",
        )
        .bind::<Integer, _>(receiver_id)
        .load::<Note>(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)
    .map(|notes| {
        notes
            .into_iter()
            .map(|n| (n.id, n.sender, n.flag, n.message))
            .collect()
    })
}

/// Leave `receiver_id` `count` plain notes.
pub async fn fill_notes(receiver_id: i32, count: i32) -> Result<(), HarnessError> {
    let fixture = |message: String| HarnessError::Fixture {
        phase: "fill an inbox",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        diesel::sql_query(
            "INSERT INTO notes (receiver_id, sender, message) \
             SELECT $1, 'filler', 'hello' FROM generate_series(1, $2)",
        )
        .bind::<Integer, _>(receiver_id)
        .bind::<Integer, _>(count)
        .execute(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(())
}

/// Make coupon `code` (upper case) giving `rewards`, each `(kind, item_id,
/// quantity)` with kind 0 an item, 1 NX Credit, 2 Maple Points, 3 NX
/// Prepaid and 4 mesos; already expired if `expired`.
pub async fn create_coupon(
    code: &str,
    expired: bool,
    rewards: &[(i16, i32, i32)],
) -> Result<(), HarnessError> {
    let code = code.to_string();
    let rewards = rewards.to_vec();
    let fixture = |message: String| HarnessError::Fixture {
        phase: "create a coupon",
        message,
    };
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url()).map_err(|e| e.to_string())?;
        conn.transaction(|conn| {
            diesel::sql_query(
                "INSERT INTO coupons (code, expires_at) \
                 VALUES ($1, CASE WHEN $2 THEN now()::timestamp - interval '1 day' END)",
            )
            .bind::<Text, _>(&code)
            .bind::<Bool, _>(expired)
            .execute(conn)?;
            for (kind, item_id, quantity) in rewards {
                diesel::sql_query(
                    "INSERT INTO coupon_rewards (coupon_id, kind, item_id, quantity) \
                     SELECT id, $2, $3, $4 FROM coupons WHERE code = $1",
                )
                .bind::<Text, _>(&code)
                .bind::<SmallInt, _>(kind)
                .bind::<Integer, _>(item_id)
                .bind::<Integer, _>(quantity)
                .execute(conn)?;
            }
            diesel::QueryResult::Ok(())
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| fixture(e.to_string()))?
    .map_err(fixture)?;
    Ok(())
}
