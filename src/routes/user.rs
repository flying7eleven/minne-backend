use crate::fairings::MinneDatabaseConnection;
use crate::schema::users;
use chrono::NaiveDateTime;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{post, State};
use serde::Deserialize;

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Deserialize)]
pub struct NewUserCallData {
    /// The first name of the user.
    pub first_name: String,
    /// The last name of the user.
    pub last_name: String,
    /// The email address of the user used as the username.
    pub email: String,
    /// The password of the user.
    pub password: String,
    /// The password of the user repeated to ensure that the user entered the password correctly.
    pub password_repeat: String,
}

#[post("/user/create", data = "<new_user>")]
pub async fn create_new_user(
    db_connection_pool: &State<MinneDatabaseConnection>,
    new_user: Json<NewUserCallData>,
) -> Status {
    use diesel::ExpressionMethods;
    use diesel::QueryDsl;
    use diesel::RunQueryDsl;
    use log::error;

    // ensure that the password and the repeated password are the same
    if new_user.password != new_user.password_repeat {
        return Status::BadRequest;
    }

    // check that all fields in the passed data are set and not empty
    if new_user.first_name.is_empty()
        || new_user.last_name.is_empty()
        || new_user.email.is_empty()
        || new_user.password.is_empty()
    {
        return Status::BadRequest;
    }

    // get a connection to the database for dealing with the request
    let db_connection = &mut match db_connection_pool.get() {
        Ok(connection) => connection,
        Err(error) => {
            error!(
                "Could not get a connection from the database connection pool. The error was: {}",
                error
            );
            return Status::InternalServerError;
        }
    };

    // check the database if a user with the given email address already exists and return if so
    let user_already_exists = diesel::select(diesel::dsl::exists(
        users::table.filter(users::email.eq(new_user.email.clone())),
    ))
    .get_result::<bool>(db_connection)
    .unwrap();
    if user_already_exists {
        return Status::BadRequest;
    }

    // create a bcrypt hash of the password with a cost of 12
    let password_hash = bcrypt::hash(new_user.password.clone(), 12).unwrap();

    // prepare the DTO for creating the new user
    let new_user = NewUser {
        first_name: new_user.first_name.clone(),
        last_name: new_user.last_name.clone(),
        email: new_user.email.clone(),
        password_hash,
    };

    // add the DTO to the database
    let entries_added = diesel::insert_into(users::table)
        .values(&new_user)
        .execute(db_connection)
        .unwrap();

    // check if the user was added to the database
    if entries_added == 0 {
        return Status::InternalServerError;
    }
    Status::NoContent
}
