table! {
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
        banned -> Bool,
        ban_msg -> Nullable<Text>,
    }
}
