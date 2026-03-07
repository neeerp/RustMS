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
    }
}

diesel::table! {
    use crate::sql_types::*;

    buddies (id) {
        id -> Int4,
        character_id -> Int4,
        buddy_id -> Int4,
        buddy_group -> Varchar,
        pending -> Bool,
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
        action -> Int2,
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

diesel::allow_tables_to_appear_in_same_query!(accounts, buddies, characters, keybindings, sessions,);
