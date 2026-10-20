//! Conversion between database rows and the `game` domain model.
//!
//! A logged-in character is loaded once into a `game::Player`, owned by its
//! client actor, and written back with `save_player` (periodically, before
//! risky operations, and on disconnect).

use crate::character::Character;
use crate::keybinding::KeybindType;
use crate::quest::QuestProgress;
use crate::schema::{
    accounts, character_mounts, character_quests, characters, equipment_stats, gacha_exp,
    inventory_items, keybindings, monster_book_cards, saved_locations, skills, teleport_rocks,
};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::upsert::excluded;
use game::inventory::DEFAULT_SLOT_LIMIT;
use game::quest::{decode_kills, encode_kills};
use game::{
    AvatarLook, CharacterStats, EquipStats, Inventory, InventoryType, Item, KeyBinding, Keymap,
    MonsterBook, Mount, Player, QuestLog, QuestRecord, QuestStatus, SkillBook, SkillRecord,
    TeleportRocks,
};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::time::SystemTime;

#[derive(Queryable, Identifiable)]
#[diesel(table_name = inventory_items)]
pub struct InventoryItemRow {
    pub id: i64,
    pub character_id: i32,
    pub inventory_type: i16,
    pub slot: i16,
    pub item_id: i32,
    pub quantity: i16,
    pub owner: String,
    pub flags: i16,
    pub expires_at: Option<SystemTime>,
    pub cash_serial: Option<i64>,
}

#[derive(Insertable)]
#[diesel(table_name = inventory_items)]
struct NewInventoryItem<'a> {
    character_id: i32,
    inventory_type: i16,
    slot: i16,
    item_id: i32,
    quantity: i16,
    owner: &'a str,
    flags: i16,
    expires_at: Option<SystemTime>,
    cash_serial: Option<i64>,
}

/// A table of per-instance equipment stats keyed by an item row: carried
/// items' `equipment_stats` and stored items' `storage_equipment_stats`.
macro_rules! equipment_stats_row {
    ($row:ident, $table:ident, $key:ident) => {
        #[derive(Queryable, Insertable)]
        #[diesel(table_name = $table)]
        pub struct $row {
            pub $key: i64,
            pub upgrade_slots: i16,
            pub level: i16,
            pub stre: i16,
            pub dex: i16,
            pub int: i16,
            pub luk: i16,
            pub hp: i16,
            pub mp: i16,
            pub watk: i16,
            pub matk: i16,
            pub wdef: i16,
            pub mdef: i16,
            pub acc: i16,
            pub avoid: i16,
            pub hands: i16,
            pub speed: i16,
            pub jump: i16,
            pub vicious: i32,
            pub item_level: i16,
            pub item_exp: i32,
        }

        impl $row {
            pub fn new($key: i64, e: &EquipStats) -> Self {
                Self {
                    $key,
                    upgrade_slots: i16::from(e.upgrade_slots),
                    level: i16::from(e.level),
                    stre: e.str,
                    dex: e.dex,
                    int: e.int,
                    luk: e.luk,
                    hp: e.hp,
                    mp: e.mp,
                    watk: e.watk,
                    matk: e.matk,
                    wdef: e.wdef,
                    mdef: e.mdef,
                    acc: e.acc,
                    avoid: e.avoid,
                    hands: e.hands,
                    speed: e.speed,
                    jump: e.jump,
                    vicious: e.vicious,
                    item_level: i16::from(e.item_level),
                    item_exp: e.item_exp,
                }
            }

            pub fn stats(self) -> EquipStats {
                EquipStats {
                    upgrade_slots: u8::try_from(self.upgrade_slots).unwrap_or(0),
                    level: u8::try_from(self.level).unwrap_or(0),
                    str: self.stre,
                    dex: self.dex,
                    int: self.int,
                    luk: self.luk,
                    hp: self.hp,
                    mp: self.mp,
                    watk: self.watk,
                    matk: self.matk,
                    wdef: self.wdef,
                    mdef: self.mdef,
                    acc: self.acc,
                    avoid: self.avoid,
                    hands: self.hands,
                    speed: self.speed,
                    jump: self.jump,
                    vicious: self.vicious,
                    item_level: u8::try_from(self.item_level).unwrap_or(0),
                    item_exp: self.item_exp,
                }
            }
        }
    };
}
pub(crate) use equipment_stats_row;

equipment_stats_row!(EquipmentStatsRow, equipment_stats, item_row_id);

#[derive(Queryable, Insertable)]
#[diesel(table_name = skills)]
pub struct SkillRow {
    pub character_id: i32,
    pub skill_id: i32,
    pub level: i32,
    pub master_level: i32,
    pub expires_at: Option<SystemTime>,
}

/// Load everything needed to play a character.
pub fn load_player(character_id: i32) -> crate::Result<Player> {
    let mut conn = crate::connection()?;
    let character: Character = characters::table.find(character_id).first(&mut conn)?;
    let gm_level: i16 = accounts::table
        .find(character.accountid)
        .select(accounts::gm_level)
        .first(&mut conn)?;

    let keymap = load_keymap(&mut conn, character_id)?;
    let mut inventory = load_inventory(&mut conn, &[character_id])?
        .remove(&character_id)
        .unwrap_or_default();
    for (kind, slots) in [
        (InventoryType::Equip, character.equip_slots),
        (InventoryType::Use, character.use_slots),
        (InventoryType::Setup, character.setup_slots),
        (InventoryType::Etc, character.etc_slots),
    ] {
        inventory.set_slot_limit(kind, u8::try_from(slots).unwrap_or(DEFAULT_SLOT_LIMIT));
    }
    let skill_book = SkillBook::from_records(
        skills::table
            .filter(skills::character_id.eq(character_id))
            .load::<SkillRow>(&mut conn)?
            .into_iter()
            .map(|row| SkillRecord {
                skill_id: row.skill_id,
                level: row.level,
                master_level: row.master_level,
                expiration: row.expires_at,
            }),
    );
    let quests = quest_log(
        character_quests::table
            .filter(character_quests::character_id.eq(character_id))
            .load::<QuestProgress>(&mut conn)?,
    );

    let mut player = Player::new(
        character.accountid,
        u8::try_from(character.world).unwrap_or(0),
        character_stats(&character),
        character.meso,
        inventory,
        skill_book,
        keymap,
        quests,
    );
    player.gm_level = gm_level;
    player.pets = crate::pet::summoned(&mut conn, &player.inventory)?;
    player.guild = crate::guild::tag_of(&mut conn, character_id)?;
    player.monster_book = MonsterBook::new(
        character.monster_book_cover,
        monster_book_cards::table
            .filter(monster_book_cards::character_id.eq(character_id))
            .select((monster_book_cards::card_id, monster_book_cards::level))
            .load::<(i32, i16)>(&mut conn)?
            .into_iter()
            .map(|(card, level)| (card, u8::try_from(level).unwrap_or(0))),
    );
    player.teleport_rocks = load_teleport_rocks(&mut conn, character_id)?;
    player.free_market_return = saved_locations::table
        .find((character_id, FREE_MARKET))
        .select(saved_locations::map_id)
        .first(&mut conn)
        .optional()?;
    player.stats.gacha_exp = gacha_exp::table
        .find(character_id)
        .select(gacha_exp::exp)
        .first(&mut conn)
        .optional()?
        .unwrap_or(0);
    if let Some((level, exp, tiredness)) = character_mounts::table
        .find(character_id)
        .select((
            character_mounts::level,
            character_mounts::exp,
            character_mounts::tiredness,
        ))
        .first(&mut conn)
        .optional()?
    {
        player.mount = Mount {
            level,
            exp,
            tiredness,
        };
    }
    Ok(player)
}

/// Persist a player snapshot. Character columns, keymap, inventory, skills,
/// and Monster Book cards are replaced atomically. Quest progress is written by its own
/// transactions and is not touched here.
pub fn save_player(player: &Player) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| write_player(conn, player))
        .map_err(Into::into)
}

/// Keep a row for a mount only once it differs from a new one.
fn save_mount(conn: &mut PgConnection, id: i32, mount: &Mount) -> QueryResult<()> {
    if *mount == Mount::default() {
        diesel::delete(character_mounts::table.find(id)).execute(conn)?;
        return Ok(());
    }
    let values = (
        character_mounts::level.eq(mount.level),
        character_mounts::exp.eq(mount.exp),
        character_mounts::tiredness.eq(mount.tiredness),
    );
    diesel::insert_into(character_mounts::table)
        .values((character_mounts::character_id.eq(id), values))
        .on_conflict(character_mounts::character_id)
        .do_update()
        .set(values)
        .execute(conn)?;
    Ok(())
}

/// Write the whole player within the caller's transaction.
pub(crate) fn write_player(conn: &mut PgConnection, player: &Player) -> QueryResult<()> {
    {
        let id = player.id();
        let stats = &player.stats;
        let slots = |kind| i16::from(player.inventory.slot_limit(kind));
        diesel::update(characters::table.find(id))
            .set((
                characters::level.eq(i16::from(stats.level)),
                characters::exp.eq(stats.exp),
                characters::job.eq(stats.job),
                characters::stre.eq(stats.str),
                characters::dex.eq(stats.dex),
                characters::int.eq(stats.int),
                characters::luk.eq(stats.luk),
                characters::hp.eq(stats.hp),
                characters::maxhp.eq(stats.max_hp),
                characters::mp.eq(stats.mp),
                characters::maxmp.eq(stats.max_mp),
                characters::ap.eq(stats.ap),
                characters::sp.eq(stats.sp),
                characters::fame.eq(stats.fame),
                characters::meso.eq(player.meso),
                characters::face.eq(stats.face),
                characters::hair.eq(stats.hair),
                characters::skin.eq(i32::from(stats.skin)),
                characters::map_id.eq(stats.map_id),
                characters::spawn_point.eq(i16::from(stats.spawn_point)),
                characters::monster_book_cover.eq(player.monster_book.cover()),
                characters::equip_slots.eq(slots(InventoryType::Equip)),
                characters::use_slots.eq(slots(InventoryType::Use)),
                characters::setup_slots.eq(slots(InventoryType::Setup)),
                characters::etc_slots.eq(slots(InventoryType::Etc)),
            ))
            .execute(conn)?;

        // Cards are only ever added or levelled up.
        let cards: Vec<_> = player
            .monster_book
            .cards()
            .map(|(card, level)| {
                (
                    monster_book_cards::character_id.eq(id),
                    monster_book_cards::card_id.eq(card),
                    monster_book_cards::level.eq(i16::from(level)),
                )
            })
            .collect();
        if !cards.is_empty() {
            diesel::insert_into(monster_book_cards::table)
                .values(&cards)
                .on_conflict((
                    monster_book_cards::character_id,
                    monster_book_cards::card_id,
                ))
                .do_update()
                .set(monster_book_cards::level.eq(excluded(monster_book_cards::level)))
                .execute(conn)?;
        }

        save_keymap(conn, id, &player.keymap)?;
        save_teleport_rocks(conn, id, &player.teleport_rocks)?;
        match player.free_market_return {
            Some(map_id) => {
                diesel::insert_into(saved_locations::table)
                    .values((
                        saved_locations::character_id.eq(id),
                        saved_locations::kind.eq(FREE_MARKET),
                        saved_locations::map_id.eq(map_id),
                    ))
                    .on_conflict((saved_locations::character_id, saved_locations::kind))
                    .do_update()
                    .set(saved_locations::map_id.eq(map_id))
                    .execute(conn)?;
            }
            None => {
                diesel::delete(saved_locations::table.find((id, FREE_MARKET))).execute(conn)?;
            }
        }
        if stats.gacha_exp > 0 {
            diesel::insert_into(gacha_exp::table)
                .values((
                    gacha_exp::character_id.eq(id),
                    gacha_exp::exp.eq(stats.gacha_exp),
                ))
                .on_conflict(gacha_exp::character_id)
                .do_update()
                .set(gacha_exp::exp.eq(stats.gacha_exp))
                .execute(conn)?;
        } else {
            diesel::delete(gacha_exp::table.find(id)).execute(conn)?;
        }
        save_mount(conn, id, &player.mount)?;
        save_inventory(conn, id, &player.inventory)?;
        crate::pet::save(conn, &player.inventory, &player.pets)?;
        // Kill progress of started quests.
        for record in player.quests.started() {
            let kills = player.quests.kills(record.quest_id);
            diesel::update(
                character_quests::table
                    .find((id, record.quest_id))
                    .filter(character_quests::status.eq(1)),
            )
            .set(character_quests::progress.eq(encode_kills(kills)))
            .execute(conn)?;
        }

        diesel::delete(skills::table.filter(skills::character_id.eq(id))).execute(conn)?;
        let skill_rows: Vec<SkillRow> = player
            .skills
            .iter()
            .map(|skill| SkillRow {
                character_id: id,
                skill_id: skill.skill_id,
                level: skill.level,
                master_level: skill.master_level,
                expires_at: skill.expiration,
            })
            .collect();
        diesel::insert_into(skills::table)
            .values(&skill_rows)
            .execute(conn)?;
        Ok(())
    }
}

/// `saved_locations.kind` of the town a character entered the Free Market
/// from.
const FREE_MARKET: i16 = 0;

fn load_teleport_rocks(conn: &mut PgConnection, character_id: i32) -> QueryResult<TeleportRocks> {
    let rows: Vec<(bool, i32)> = teleport_rocks::table
        .filter(teleport_rocks::character_id.eq(character_id))
        .order((teleport_rocks::vip, teleport_rocks::place))
        .select((teleport_rocks::vip, teleport_rocks::map_id))
        .load(conn)?;
    let list = |vip| {
        rows.iter()
            .filter(|&&(of, _)| of == vip)
            .map(|&(_, map)| map)
            .collect()
    };
    Ok(TeleportRocks::new(list(false), list(true)))
}

fn save_teleport_rocks(
    conn: &mut PgConnection,
    character_id: i32,
    rocks: &TeleportRocks,
) -> QueryResult<()> {
    diesel::delete(teleport_rocks::table.filter(teleport_rocks::character_id.eq(character_id)))
        .execute(conn)?;
    let rows: Vec<_> = [false, true]
        .into_iter()
        .flat_map(|vip| {
            rocks
                .maps(vip)
                .iter()
                .enumerate()
                .map(move |(place, &map)| {
                    (
                        teleport_rocks::character_id.eq(character_id),
                        teleport_rocks::vip.eq(vip),
                        teleport_rocks::place.eq(place as i16),
                        teleport_rocks::map_id.eq(map),
                    )
                })
        })
        .collect();
    diesel::insert_into(teleport_rocks::table)
        .values(&rows)
        .execute(conn)?;
    Ok(())
}

/// Stats and looks for every character on an account, for the character list.
pub fn load_character_entries(account_id: i32) -> crate::Result<Vec<(CharacterStats, AvatarLook)>> {
    let mut conn = crate::connection()?;
    let rows: Vec<Character> = characters::table
        .filter(characters::accountid.eq(account_id))
        .order(characters::id)
        .load(&mut conn)?;
    let ids: Vec<i32> = rows.iter().map(|c| c.id).collect();
    let mut inventories = load_inventory(&mut conn, &ids)?;

    Ok(rows
        .iter()
        .map(|character| {
            let stats = character_stats(character);
            let inventory = inventories.remove(&character.id).unwrap_or_default();
            let look = AvatarLook::from_equipped(&stats, inventory.equipped_ids());
            (stats, look)
        })
        .collect())
}

/// Write a freshly created character's starting items.
pub(crate) fn insert_items(
    conn: &mut PgConnection,
    character_id: i32,
    items: impl IntoIterator<Item = (InventoryType, i16, Item)>,
) -> QueryResult<()> {
    for (kind, slot, item) in items {
        insert_item(conn, character_id, kind, slot, &item)?;
    }
    Ok(())
}

pub(crate) fn character_stats(character: &Character) -> CharacterStats {
    CharacterStats {
        id: character.id,
        name: character.name.clone(),
        gender: u8::try_from(character.gender).unwrap_or(0),
        skin: u8::try_from(character.skin).unwrap_or(0),
        face: character.face,
        hair: character.hair,
        level: u8::try_from(character.level).unwrap_or(u8::MAX),
        job: character.job,
        str: character.stre,
        dex: character.dex,
        int: character.int,
        luk: character.luk,
        hp: character.hp,
        max_hp: character.maxhp,
        mp: character.mp,
        max_mp: character.maxmp,
        ap: character.ap,
        sp: character.sp,
        exp: character.exp,
        // Kept in its own table, which load_player reads.
        gacha_exp: 0,
        fame: character.fame,
        map_id: character.map_id,
        spawn_point: u8::try_from(character.spawn_point).unwrap_or(0),
    }
}

pub(crate) fn quest_log(progress: Vec<QuestProgress>) -> QuestLog {
    QuestLog::from_records(progress.iter().map(quest_record))
}

/// The domain form of one quest progress row.
pub fn quest_record(progress: &QuestProgress) -> QuestRecord {
    QuestRecord {
        quest_id: progress.quest_id,
        status: match (progress.status, progress.completed_at) {
            (2, Some(at)) => QuestStatus::Completed { at },
            _ => QuestStatus::Started {
                kills: decode_kills(&progress.progress),
            },
        },
    }
}

fn load_keymap(conn: &mut PgConnection, character_id: i32) -> QueryResult<Keymap> {
    let rows: Vec<(i16, KeybindType, i32)> = keybindings::table
        .filter(keybindings::character_id.eq(character_id))
        .select((
            keybindings::key,
            keybindings::bind_type,
            keybindings::action,
        ))
        .load(conn)?;
    let mut keymap = Keymap::default();
    for (key, kind, action) in rows {
        keymap.bind(
            key,
            KeyBinding {
                kind: kind.into(),
                action,
            },
        );
    }
    Ok(keymap)
}

pub(crate) fn save_keymap(
    conn: &mut PgConnection,
    character_id: i32,
    keymap: &Keymap,
) -> QueryResult<()> {
    diesel::delete(keybindings::table.filter(keybindings::character_id.eq(character_id)))
        .execute(conn)?;
    let rows: Vec<_> = keymap
        .iter()
        .map(|(key, binding)| {
            (
                keybindings::character_id.eq(character_id),
                keybindings::key.eq(key),
                keybindings::bind_type.eq(KeybindType::from(binding.kind)),
                keybindings::action.eq(binding.action),
            )
        })
        .collect();
    diesel::insert_into(keybindings::table)
        .values(&rows)
        .execute(conn)?;
    Ok(())
}

fn load_inventory(
    conn: &mut PgConnection,
    character_ids: &[i32],
) -> QueryResult<HashMap<i32, Inventory>> {
    let rows: Vec<(InventoryItemRow, Option<EquipmentStatsRow>)> = inventory_items::table
        .left_join(equipment_stats::table)
        .filter(inventory_items::character_id.eq_any(character_ids))
        .load(conn)?;
    let pet_serials: Vec<i64> = rows
        .iter()
        .filter(|(row, _)| game::pet::is_pet(row.item_id))
        .filter_map(|(row, _)| row.cash_serial)
        .collect();
    let mut pets = crate::pet::load(conn, &pet_serials)?;

    let mut inventories: HashMap<i32, Inventory> = HashMap::new();
    for (row, equip) in rows {
        let Some(kind) = InventoryType::from_i16(row.inventory_type) else {
            continue;
        };
        let item = Item {
            item_id: row.item_id,
            quantity: row.quantity,
            owner: row.owner,
            flags: row.flags,
            expiration: row.expires_at,
            cash_serial: row.cash_serial,
            equip: equip.map(EquipmentStatsRow::stats),
            pet: row.cash_serial.and_then(|serial| pets.remove(&serial)),
        };
        inventories
            .entry(row.character_id)
            .or_default()
            .insert(kind, row.slot, item);
    }
    Ok(inventories)
}

fn save_inventory(
    conn: &mut PgConnection,
    character_id: i32,
    inventory: &Inventory,
) -> QueryResult<()> {
    // Equipment stats cascade with their item rows.
    diesel::delete(inventory_items::table.filter(inventory_items::character_id.eq(character_id)))
        .execute(conn)?;
    for (kind, slot, item) in inventory.all() {
        insert_item(conn, character_id, kind, slot, item)?;
    }
    Ok(())
}

fn insert_item(
    conn: &mut PgConnection,
    character_id: i32,
    kind: InventoryType,
    slot: i16,
    item: &Item,
) -> QueryResult<()> {
    let row_id: i64 = diesel::insert_into(inventory_items::table)
        .values(&NewInventoryItem {
            character_id,
            inventory_type: kind as i16,
            slot,
            item_id: item.item_id,
            quantity: item.quantity,
            owner: &item.owner,
            flags: item.flags,
            expires_at: item.expiration,
            cash_serial: item.cash_serial,
        })
        .returning(inventory_items::id)
        .get_result(conn)?;

    if let Some(e) = &item.equip {
        diesel::insert_into(equipment_stats::table)
            .values(&EquipmentStatsRow::new(row_id, e))
            .execute(conn)?;
    }
    Ok(())
}
