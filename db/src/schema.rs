table! {
    use crate::sql_types::*;

    accounts (id) {
        id -> Int4,
        user_name -> Varchar,
        password -> Varchar,
        pin -> Varchar,
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

table! {
    use crate::sql_types::*;

    characters (id) {
        id -> Int4,
        accountid -> Int4,
        world -> Int2,
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

table! {
    use crate::sql_types::*;

    keybindings (id) {
        id -> Int4,
        character_id -> Int4,
        key -> Int2,
        bind_type -> Keybind_type,
        action -> Int2,
    }
}

table! {
    use crate::sql_types::*;

    sessions (id) {
        id -> Int4,
        account_id -> Int4,
        character_id -> Nullable<Int4>,
        ip -> Inet,
        hwid -> Varchar,
        state -> Session_state,
        updated_at -> Timestamp,
        created_at -> Timestamp,
    }
}

joinable!(sessions -> accounts (account_id));

allow_tables_to_appear_in_same_query!(
    accounts,
    characters,
    keybindings,
    sessions,
);
