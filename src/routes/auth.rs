use crate::fairings::{BackendConfiguration, MinneDatabaseConnection};
use crate::guards::AuthenticatedUser;
use crate::schema::personal_access_tokens;
use chrono::NaiveDateTime;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, post};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct NewPersonalAccessTokenData {
    /// The name of the new personal access token.
    pub name: String,
}

#[derive(Deserialize)]
pub struct Credentials {
    /// The email address of the user used as the username.
    pub email: String,
    /// The password of the user.
    pub password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    /// The access token to use for API requests.
    access_token: String,
}

#[derive(Serialize)]
pub struct PersonalAccessTokenResponse {
    pub token: String,
    pub secret: String,
}

#[derive(Queryable)]
pub struct PersonalAccessToken {
    pub id: i32,
    pub name: String,
    pub token: String,
    pub secret: String,
    pub user_id: i32,
    pub disabled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = personal_access_tokens)]
pub struct NewPersonalAccessToken {
    pub name: String,
    pub user_id: i32,
    pub token: String,
    pub secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    exp: usize,
    iat: usize,
    nbf: usize,
    pub sub: String,
}

fn get_token_for_user(
    subject: &String,
    signature_psk: &String,
    access_token_lifetime: usize,
) -> Option<String> {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use log::error;
    use std::time::{SystemTime, UNIX_EPOCH};

    // get the issuing time for the token
    let token_issued_at = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as usize,
        Err(error) => {
            error!(
                "Could not get the issuing time for the token. The error was: {}",
                error
            );
            return None;
        }
    };

    // calculate the time when the token expires
    let token_expires_at = token_issued_at + 1 + access_token_lifetime;

    // define the content of the actual token
    let token_claims = Claims {
        exp: token_expires_at,
        iat: token_issued_at,
        nbf: token_issued_at + 1,
        sub: subject.clone(),
    };

    // get the signing key for the token
    let encoding_key = EncodingKey::from_secret(signature_psk.as_ref());

    // generate a new JWT for the supplied header and token claims. if we were successful, return
    // the token
    let header = Header::new(Algorithm::HS512);
    if let Ok(token) = encode(&header, &token_claims, &encoding_key) {
        return Some(token);
    }

    // if we fail, return None
    None
}

#[delete("/auth/pat")]
pub async fn disable_pat(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
) -> Status {
    use crate::schema::personal_access_tokens::{disabled, table, token, updated_at};
    use diesel::ExpressionMethods;
    use diesel::RunQueryDsl;
    use log::error;

    // if no personal access token was used, exit early
    if authenticated_user.used_pat.is_empty() {
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

    // set the personal access token to disabled based on the use personal access token for authentication
    diesel::update(table)
        .filter(token.eq(authenticated_user.used_pat))
        .set((disabled.eq(true), updated_at.eq(diesel::dsl::now)))
        .execute(db_connection)
        .unwrap();

    // we assume that we've succeeded and can return with an appropriate status code
    Status::NoContent
}

pub async fn create_new_pat(
    db_connection_pool: &State<MinneDatabaseConnection>,
    authenticated_user: AuthenticatedUser,
    new_pata_data: Json<NewPersonalAccessTokenData>,
) -> Result<Json<PersonalAccessTokenResponse>, Status> {
    use diesel::RunQueryDsl;
    use log::error;
    use uuid::Uuid;

    // get a connection to the database for dealing with the request
    let db_connection = &mut match db_connection_pool.get() {
        Ok(connection) => connection,
        Err(error) => {
            error!(
                "Could not get a connection from the database connection pool. The error was: {}",
                error
            );
            return Err(Status::InternalServerError);
        }
    };

    // create a new personal access token for the user
    let new_pat = NewPersonalAccessToken {
        name: new_pata_data.name.clone(),
        user_id: authenticated_user.id,
        token: Uuid::new_v4().to_string(),
        secret: Uuid::new_v4().to_string(),
    };

    // try to insert the new personal access token into the database
    let entries_added = diesel::insert_into(personal_access_tokens::table)
        .values(&new_pat)
        .execute(db_connection)
        .unwrap();

    // if we did not add exactly one entry, return an error
    if entries_added != 1 {
        return Err(Status::InternalServerError);
    }

    // return the token as well as the corresponding secret to the calling party
    Ok(Json(PersonalAccessTokenResponse {
        token: new_pat.token,
        secret: new_pat.secret,
    }))
}

#[post("/auth/login", data = "<credentials>")]
pub async fn get_authentication_token(
    db_connection_pool: &State<MinneDatabaseConnection>,
    config: &State<BackendConfiguration>,
    credentials: Json<Credentials>,
) -> Result<Json<TokenResponse>, Status> {
    use crate::routes::user::User;
    use crate::schema::users::dsl::{email, users};
    use bcrypt::verify;
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
    use log::error;

    // get a connection to the database for dealing with the request
    let db_connection = &mut match db_connection_pool.get() {
        Ok(connection) => connection,
        Err(error) => {
            error!(
                "Could not get a connection from the database connection pool. The error was: {}",
                error
            );
            return Err(Status::InternalServerError);
        }
    };

    // try to get the user record for the supplied username
    let supplied_username = credentials.email.clone();
    let maybe_user_result = db_connection
        .build_transaction()
        .read_only()
        .run::<_, diesel::result::Error, _>(move |connection| {
            if let Ok(found_users) = users
                .filter(email.eq(supplied_username))
                .load::<User>(connection)
            {
                // if we did not get exactly one user, return an 'error'
                if found_users.len() != 1 {
                    return Err(diesel::result::Error::NotFound);
                }

                // return the found user
                return Ok(found_users[0].clone());
            }

            //
            return Err(diesel::result::Error::NotFound); // TODO: not the real error
        });

    // try to get the actual user object or delay a bit and then return with the corresponding error
    let user = match maybe_user_result {
        Ok(user) => user,
        Err(_) => {
            // ensure that we know what happened
            error!("Could not get the user record for '{}'", credentials.email);

            // just slow down the process to prevent easy checking if a user name exists or not
            let _ = verify(
                "some_password",
                "$2y$12$7xMzqvnHyizkumZYpIRXheGMAqDKVo8HKtpmQSn51JUfY0N2VN4ua",
            );

            // finally we can tell teh user that he/she is not authorized
            return Err(Status::Unauthorized);
        }
    };

    // check if the supplied password matches the one we stored in the database using the same bcrypt
    // parameters
    match verify(&credentials.password, user.password_hash.as_str()) {
        Ok(is_password_correct) => {
            if !is_password_correct {
                return Err(Status::Unauthorized);
            }
        }
        Err(error) => {
            error!("Could not verify the supplied password with the one stored in the database. The error was: {}", error);
            return Err(Status::InternalServerError);
        }
    }

    // if we get here, the we ensured that the user is known and that the supplied password
    // was valid, we can generate a new access token and return it to the calling party
    if let Some(token) = get_token_for_user(
        &credentials.email,
        &config.token_signature_psk,
        config.access_token_lifetime_in_seconds,
    ) {
        return Ok(Json(TokenResponse {
            access_token: token,
        }));
    }

    // it seems that we failed to generate a valid token, this should never happen, something
    // seems to be REALLY wrong
    Err(Status::InternalServerError)
}
