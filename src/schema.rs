// @generated automatically by Diesel CLI.

diesel::table! {
    personal_access_tokens (id) {
        id -> Int4,
        name -> Varchar,
        token -> Varchar,
        secret -> Varchar,
        user_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        disabled -> Bool,
    }
}

diesel::table! {
    tasks (id) {
        id -> Int4,
        title -> Varchar,
        owner -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        done_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        first_name -> Varchar,
        last_name -> Varchar,
        email -> Varchar,
        password_hash -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(personal_access_tokens -> users (user_id));
diesel::joinable!(tasks -> users (owner));

diesel::allow_tables_to_appear_in_same_query!(personal_access_tokens, tasks, users,);
