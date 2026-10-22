// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "keybind_type"))]
    pub struct KeybindType;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "session_state"))]
    pub struct SessionState;
}

diesel::table! {
    use crate::sql_types::*;

    accounts (id) {
        id -> Int4,
        #[max_length = 13]
        user_name -> Varchar,
        #[max_length = 128]
        password -> Varchar,
        #[max_length = 4]
        pin -> Varchar,
        #[max_length = 26]
        pic -> Varchar,
        logged_in -> Bool,
        last_login_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        character_slots -> Int2,
        gender -> Int2,
        accepted_tos -> Bool,
        banned -> Bool,
        ban_msg -> Nullable<Text>,
        gm_level -> Int2,
        nx_credit -> Int4,
        maple_points -> Int4,
        nx_prepaid -> Int4,
        storage_slots -> Int2,
        storage_meso -> Int4,
    }
}

diesel::table! {
    cash_sales (sn) {
        sn -> Int4,
        sold -> Int8,
    }
}

diesel::table! {
    cash_items (id) {
        id -> Int8,
        account_id -> Int4,
        item_id -> Int4,
        sn -> Int4,
        quantity -> Int2,
        #[max_length = 13]
        gift_from -> Varchar,
        expires_at -> Nullable<Timestamp>,
        #[max_length = 73]
        gift_message -> Nullable<Varchar>,
    }
}

diesel::table! {
    wish_lists (character_id, slot) {
        character_id -> Int4,
        slot -> Int2,
        sn -> Int4,
    }
}

diesel::table! {
    use crate::sql_types::*;

    characters (id) {
        id -> Int4,
        accountid -> Int4,
        world -> Int2,
        #[max_length = 13]
        name -> Varchar,
        level -> Int2,
        exp -> Int4,
        stre -> Int2,
        dex -> Int2,
        luk -> Int2,
        int -> Int2,
        hp -> Int2,
        mp -> Int2,
        maxhp -> Int2,
        maxmp -> Int2,
        ap -> Int2,
        fame -> Int2,
        meso -> Int4,
        job -> Int2,
        face -> Int4,
        hair -> Int4,
        hair_color -> Int4,
        skin -> Int4,
        gender -> Int2,
        created_at -> Timestamp,
        map_id -> Int4,
        sp -> Int2,
        spawn_point -> Int2,
        monster_book_cover -> Int4,
        equip_slots -> Int2,
        use_slots -> Int2,
        setup_slots -> Int2,
        etc_slots -> Int2,
    }
}

diesel::table! {
    inventory_items (id) {
        id -> Int8,
        character_id -> Int4,
        inventory_type -> Int2,
        slot -> Int2,
        item_id -> Int4,
        quantity -> Int2,
        #[max_length = 13]
        owner -> Varchar,
        flags -> Int2,
        expires_at -> Nullable<Timestamp>,
        cash_serial -> Nullable<Int8>,
    }
}

diesel::table! {
    equipment_stats (item_row_id) {
        item_row_id -> Int8,
        upgrade_slots -> Int2,
        level -> Int2,
        stre -> Int2,
        dex -> Int2,
        int -> Int2,
        luk -> Int2,
        hp -> Int2,
        mp -> Int2,
        watk -> Int2,
        matk -> Int2,
        wdef -> Int2,
        mdef -> Int2,
        acc -> Int2,
        avoid -> Int2,
        hands -> Int2,
        speed -> Int2,
        jump -> Int2,
        vicious -> Int4,
        item_level -> Int2,
        item_exp -> Int4,
    }
}

diesel::table! {
    skills (character_id, skill_id) {
        character_id -> Int4,
        skill_id -> Int4,
        level -> Int4,
        master_level -> Int4,
        expires_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use crate::sql_types::*;
    use super::sql_types::KeybindType;

    keybindings (id) {
        id -> Int4,
        character_id -> Int4,
        key -> Int2,
        bind_type -> KeybindType,
        action -> Int4,
    }
}

diesel::table! {
    use crate::sql_types::*;
    use super::sql_types::SessionState;

    sessions (id) {
        id -> Int4,
        account_id -> Int4,
        character_id -> Nullable<Int4>,
        ip -> Inet,
        #[max_length = 12]
        hwid -> Varchar,
        state -> SessionState,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        selected_world_id -> Nullable<Int2>,
        selected_channel_id -> Nullable<Int2>,
    }
}

diesel::joinable!(characters -> accounts (accountid));
diesel::joinable!(keybindings -> characters (character_id));
diesel::joinable!(sessions -> accounts (account_id));
diesel::joinable!(sessions -> characters (character_id));
diesel::joinable!(inventory_items -> characters (character_id));
diesel::joinable!(equipment_stats -> inventory_items (item_row_id));
diesel::joinable!(skills -> characters (character_id));
diesel::joinable!(cash_items -> accounts (account_id));
diesel::joinable!(wish_lists -> characters (character_id));

diesel::table! {
    character_quests (character_id, quest_id) {
        character_id -> Int4,
        quest_id -> Int2,
        status -> Int2,
        completed_at -> Nullable<Timestamp>,
        progress -> Text,
    }
}
diesel::joinable!(character_quests -> characters (character_id));

diesel::table! {
    buddies (character_id, buddy_id) {
        character_id -> Int4,
        buddy_id -> Int4,
        #[max_length = 16]
        group_name -> Varchar,
        pending -> Bool,
    }
}
diesel::table! {
    monster_book_cards (character_id, card_id) {
        character_id -> Int4,
        card_id -> Int4,
        level -> Int2,
    }
}
diesel::joinable!(monster_book_cards -> characters (character_id));
diesel::table! {
    explored_maps (character_id, record_id, map_id) {
        character_id -> Int4,
        record_id -> Int2,
        map_id -> Int4,
    }
}
diesel::joinable!(explored_maps -> characters (character_id));
diesel::table! {
    teleport_rocks (character_id, vip, place) {
        character_id -> Int4,
        vip -> Bool,
        place -> Int2,
        map_id -> Int4,
    }
}
diesel::joinable!(teleport_rocks -> characters (character_id));
diesel::table! {
    skill_macros (character_id, place) {
        character_id -> Int4,
        place -> Int2,
        name -> Text,
        silent -> Bool,
        skill1 -> Int4,
        skill2 -> Int4,
        skill3 -> Int4,
    }
}
diesel::joinable!(skill_macros -> characters (character_id));
diesel::table! {
    saved_locations (character_id, kind) {
        character_id -> Int4,
        kind -> Int2,
        map_id -> Int4,
    }
}
diesel::joinable!(saved_locations -> characters (character_id));
diesel::table! {
    gacha_exp (character_id) {
        character_id -> Int4,
        exp -> Int4,
    }
}
diesel::joinable!(gacha_exp -> characters (character_id));
diesel::table! {
    auto_potions (character_id) {
        character_id -> Int4,
        hp_item -> Int4,
        mp_item -> Int4,
    }
}
diesel::joinable!(auto_potions -> characters (character_id));
diesel::table! {
    quick_slots (character_id) {
        character_id -> Int4,
        key0 -> Int2,
        key1 -> Int2,
        key2 -> Int2,
        key3 -> Int2,
        key4 -> Int2,
        key5 -> Int2,
        key6 -> Int2,
        key7 -> Int2,
    }
}
diesel::joinable!(quick_slots -> characters (character_id));
diesel::table! {
    character_mounts (character_id) {
        character_id -> Int4,
        level -> Int4,
        exp -> Int4,
        tiredness -> Int4,
    }
}
diesel::joinable!(character_mounts -> characters (character_id));
diesel::table! {
    fame_log (id) {
        id -> Int4,
        giver_id -> Int4,
        target_id -> Int4,
        given_at -> Timestamp,
    }
}
diesel::table! {
    storage_items (id) {
        id -> Int8,
        account_id -> Int4,
        position -> Int2,
        item_id -> Int4,
        quantity -> Int2,
        #[max_length = 13]
        owner -> Varchar,
        flags -> Int2,
        expires_at -> Nullable<Timestamp>,
        cash_serial -> Nullable<Int8>,
    }
}
diesel::table! {
    storage_equipment_stats (storage_item_id) {
        storage_item_id -> Int8,
        upgrade_slots -> Int2,
        level -> Int2,
        stre -> Int2,
        dex -> Int2,
        int -> Int2,
        luk -> Int2,
        hp -> Int2,
        mp -> Int2,
        watk -> Int2,
        matk -> Int2,
        wdef -> Int2,
        mdef -> Int2,
        acc -> Int2,
        avoid -> Int2,
        hands -> Int2,
        speed -> Int2,
        jump -> Int2,
        vicious -> Int4,
        item_level -> Int2,
        item_exp -> Int4,
    }
}
diesel::joinable!(storage_items -> accounts (account_id));
diesel::joinable!(storage_equipment_stats -> storage_items (storage_item_id));
diesel::table! {
    guilds (id) {
        id -> Int4,
        world_id -> Int2,
        #[max_length = 12]
        name -> Varchar,
        #[max_length = 12]
        title1 -> Varchar,
        #[max_length = 12]
        title2 -> Varchar,
        #[max_length = 12]
        title3 -> Varchar,
        #[max_length = 12]
        title4 -> Varchar,
        #[max_length = 12]
        title5 -> Varchar,
        capacity -> Int2,
        mark_bg -> Int2,
        mark_bg_color -> Int2,
        mark -> Int2,
        mark_color -> Int2,
        notice -> Text,
        points -> Int4,
        alliance_id -> Int4,
    }
}
diesel::table! {
    guild_members (character_id) {
        character_id -> Int4,
        guild_id -> Int4,
        rank -> Int2,
        joined_at -> Timestamp,
        alliance_rank -> Int2,
    }
}
diesel::table! {
    alliances (id) {
        id -> Int4,
        world_id -> Int2,
        #[max_length = 12]
        name -> Varchar,
        #[max_length = 12]
        title1 -> Varchar,
        #[max_length = 12]
        title2 -> Varchar,
        #[max_length = 12]
        title3 -> Varchar,
        #[max_length = 12]
        title4 -> Varchar,
        #[max_length = 12]
        title5 -> Varchar,
        capacity -> Int2,
        #[max_length = 100]
        notice -> Varchar,
    }
}
diesel::joinable!(guild_members -> characters (character_id));
diesel::joinable!(guild_members -> guilds (guild_id));
diesel::table! {
    pet_ignores (serial, item_id) {
        serial -> Int8,
        item_id -> Int4,
    }
}
diesel::joinable!(pet_ignores -> pets (serial));
diesel::table! {
    pets (serial) {
        serial -> Int8,
        #[max_length = 13]
        name -> Varchar,
        level -> Int2,
        closeness -> Int2,
        fullness -> Int2,
        skill -> Int4,
        attribute -> Int2,
        summoned -> Nullable<Int2>,
    }
}
diesel::table! {
    notes (id) {
        id -> Int4,
        receiver_id -> Int4,
        #[max_length = 13]
        sender -> Varchar,
        message -> Text,
        sent_at -> Timestamp,
        flag -> Int2,
    }
}
diesel::joinable!(notes -> characters (receiver_id));
diesel::table! {
    gift_replies (cash_item_id) {
        cash_item_id -> Int8,
    }
}
diesel::joinable!(gift_replies -> cash_items (cash_item_id));
diesel::table! {
    guild_bbs_threads (id) {
        id -> Int4,
        guild_id -> Int4,
        local_id -> Int4,
        poster_id -> Int4,
        #[max_length = 25]
        title -> Varchar,
        body -> Text,
        icon -> Int2,
        posted_at -> Timestamp,
    }
}
diesel::joinable!(guild_bbs_threads -> guilds (guild_id));
diesel::joinable!(guild_bbs_threads -> characters (poster_id));
diesel::table! {
    guild_bbs_replies (id) {
        id -> Int4,
        thread_id -> Int4,
        poster_id -> Int4,
        #[max_length = 25]
        text -> Varchar,
        posted_at -> Timestamp,
    }
}
diesel::joinable!(guild_bbs_replies -> guild_bbs_threads (thread_id));
diesel::joinable!(guild_bbs_replies -> characters (poster_id));
diesel::table! {
    parcels (id) {
        id -> Int8,
        receiver_id -> Int4,
        #[max_length = 13]
        sender -> Varchar,
        mesos -> Int4,
        message -> Nullable<Text>,
        quick -> Bool,
        sent_at -> Timestamp,
        announced -> Bool,
        item_id -> Nullable<Int4>,
        quantity -> Int2,
        #[max_length = 13]
        owner -> Varchar,
        flags -> Int2,
        expires_at -> Nullable<Timestamp>,
        cash_serial -> Nullable<Int8>,
    }
}
diesel::table! {
    parcel_equipment_stats (parcel_id) {
        parcel_id -> Int8,
        upgrade_slots -> Int2,
        level -> Int2,
        stre -> Int2,
        dex -> Int2,
        int -> Int2,
        luk -> Int2,
        hp -> Int2,
        mp -> Int2,
        watk -> Int2,
        matk -> Int2,
        wdef -> Int2,
        mdef -> Int2,
        acc -> Int2,
        avoid -> Int2,
        hands -> Int2,
        speed -> Int2,
        jump -> Int2,
        vicious -> Int4,
        item_level -> Int2,
        item_exp -> Int4,
    }
}
diesel::joinable!(parcels -> characters (receiver_id));
diesel::joinable!(parcel_equipment_stats -> parcels (parcel_id));
diesel::table! {
    family_members (character_id) {
        character_id -> Int4,
        senior_id -> Nullable<Int4>,
        reputation -> Int4,
        total_reputation -> Int4,
        today_reputation -> Int4,
        senior_reputation -> Int4,
        #[max_length = 200]
        precepts -> Varchar,
        rep_day -> Date,
    }
}
diesel::joinable!(family_members -> characters (character_id));
diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    buddies,
    characters,
    keybindings,
    sessions,
    character_quests,
    inventory_items,
    equipment_stats,
    skills,
    cash_items,
    cash_sales,
    wish_lists,
    monster_book_cards,
    explored_maps,
    teleport_rocks,
    skill_macros,
    saved_locations,
    gacha_exp,
    auto_potions,
    quick_slots,
    character_mounts,
    fame_log,
    storage_items,
    storage_equipment_stats,
    guilds,
    guild_members,
    pet_ignores,
    pets,
    notes,
    gift_replies,
    guild_bbs_threads,
    guild_bbs_replies,
    alliances,
    parcels,
    parcel_equipment_stats,
    family_members,
);
