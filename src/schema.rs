// @generated automatically by Diesel CLI.

diesel::table! {
    personal_access_tokens (id) {
        id -> Int4,
        name -> Varchar,
        token -> Varchar,
        secret -> Varchar,
        user_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        disabled -> Bool,
    }
}

diesel::table! {
    tasks (id) {
        id -> Int4,
        title -> Varchar,
        owner -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        done_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        first_name -> Varchar,
        last_name -> Varchar,
        email -> Varchar,
        password_hash -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(personal_access_tokens -> users (user_id));
diesel::joinable!(tasks -> users (owner));

diesel::allow_tables_to_appear_in_same_query!(personal_access_tokens, tasks, users,);
