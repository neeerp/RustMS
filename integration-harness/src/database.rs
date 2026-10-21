//! The harness stack's database, for state a test can't reach through the
//! game, such as an account's NX.

use crate::HarnessError;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Nullable, SmallInt, Text};

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
/// `status` (1 started, 2 completed) with kill `progress`.
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
             VALUES ($1, $2, $3, CASE WHEN $3 = 2 THEN now() END, $4)",
        )
        .bind::<Integer, _>(character_id)
        .bind::<SmallInt, _>(quest_id)
        .bind::<SmallInt, _>(status)
        .bind::<Text, _>(progress)
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
