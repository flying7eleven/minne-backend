use chrono::{DateTime, Utc};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

/// The representation of an authenticated user. As soon as this is included in the parameters
/// of a route, the call can be just made with an valid token in the header.
pub struct AuthenticatedUser {
    /// The internally used ID for the current user.
    pub id: i32,
    /// The Personal Access Token which was used or an empty string if the user used a access token.
    pub used_pat: String,
}

#[derive(Queryable, Clone)]
pub struct PersonalAccessToken {
    pub id: i32,
    pub name: String,
    pub token: String,
    pub secret: String,
    pub user_id: i32,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum AuthorizationError {
    /// Could not find any authentication header in the request.
    MissingAuthorizationHeader,
    /// It seems that the authentication header is not well-formed (e.g. Bearer is missing)
    MalformedAuthorizationHeader,
    /// It seems that the supplied token is not valid (e.g. signature validation failed)
    InvalidToken,
}

impl<'r> AuthenticatedUser {
    async fn pat_flow(
        authorization_information: Vec<&str>,
        request: &'r Request<'_>,
    ) -> Outcome<AuthenticatedUser, AuthorizationError> {
        use crate::fairings::MinneDatabaseConnection;
        use crate::schema::personal_access_tokens::{disabled, secret, table, token};
        use diesel::ExpressionMethods;
        use diesel::{QueryDsl, RunQueryDsl};
        use log::{debug, trace};
        use rocket::http::Status;

        // ensure that we know which flow we are using
        debug!("Using the Personal Access Token authentication flow");

        // get a database connection from the connection pool to fetch more token information
        let db_connection_pool = request
            .rocket()
            .state::<MinneDatabaseConnection>()
            .expect("Could not get a database connection from the pool");

        // get the personal access token entry from the database based on the supplied token
        let token_and_secret = authorization_information[1]
            .split(':')
            .collect::<Vec<&str>>();
        let pat = table
            .filter(token.eq(token_and_secret[0]))
            .filter(secret.eq(token_and_secret[1]))
            .filter(disabled.eq(false))
            .first::<PersonalAccessToken>(&mut db_connection_pool.get().unwrap());

        // if no pat could be found return an error
        if pat.is_err() {
            debug!("There was no PAT token found in the database which matches the supplied token and is not disabled");
            trace!(
                "The token was {} and its secret was {}",
                token_and_secret[0],
                token_and_secret[1]
            );
            return Outcome::Failure((Status::Forbidden, AuthorizationError::InvalidToken));
        }

        // otherwise it seems that the user is authenticated and we can return the corresponding data structure
        let unwrapped_pat = pat.unwrap();
        Outcome::Success(AuthenticatedUser {
            id: unwrapped_pat.user_id,
            used_pat: unwrapped_pat.token,
        })
    }

    async fn bearer_flow(
        authorization_information: Vec<&str>,
        request: &'r Request<'_>,
    ) -> Outcome<AuthenticatedUser, AuthorizationError> {
        use crate::fairings::{BackendConfiguration, MinneDatabaseConnection};
        use crate::routes::auth::Claims;
        use crate::schema::users::{dsl::users, email, id};
        use diesel::ExpressionMethods;
        use diesel::{QueryDsl, RunQueryDsl};
        use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
        use log::{debug, error};
        use rocket::http::Status;

        // ensure that we know which flow we are using
        debug!("Using the Bearer authentication flow");

        // specify the parameter for the validation of the token
        let mut validation_parameter = Validation::new(Algorithm::HS512);
        validation_parameter.leeway = 5; // allow a time difference of max. 5 seconds
        validation_parameter.validate_exp = true;
        validation_parameter.validate_nbf = true;

        // get the current backend configuration for the token signature psk
        let backend_config = request.rocket().state::<BackendConfiguration>().map_or(
            BackendConfiguration {
                token_signature_psk: "".to_string(),
                access_token_lifetime_in_seconds: 0,
                refresh_token_lifetime_in_seconds: 0,
                user_registration_enabled: false,
            },
            |config| config.clone(),
        );

        // get the 'validation' key for the token
        let decoding_key = DecodingKey::from_secret(backend_config.token_signature_psk.as_ref());

        // verify the validity of the token supplied in the header
        let decoded_token = match decode::<Claims>(
            authorization_information[1],
            &decoding_key,
            &validation_parameter,
        ) {
            Ok(token) => token,
            Err(error) => {
                error!(
                    "The supplied token seems to be invalid. The error was: {}",
                    error
                );
                return Outcome::Failure((Status::Forbidden, AuthorizationError::InvalidToken));
            }
        };

        // get a database connection from the connection pool to fetch more user information
        let db_connection_pool = request
            .rocket()
            .state::<MinneDatabaseConnection>()
            .expect("Could not get a database connection from the pool");

        // get the user id using diesel based on the supplied JWT tokens subject
        let user_id = users
            .select(id)
            .filter(email.eq(decoded_token.claims.sub.clone()))
            .first::<i32>(&mut db_connection_pool.get().unwrap())
            .unwrap();

        // if we reach this step, the validation was successful, and we can allow the user to
        // call the route
        return Outcome::Success(AuthenticatedUser {
            id: user_id,
            used_pat: "".to_string(),
        });
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AuthorizationError;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<AuthenticatedUser, AuthorizationError> {
        use log::error;
        use rocket::http::Status;

        // try to get the authentication header, if there is non, return an error
        match request.headers().get_one("Authorization") {
            Some(maybe_authorization) => {
                // split the token type from the actual token... there have to be two parts
                let authorization_information =
                    maybe_authorization.split(" ").collect::<Vec<&str>>();
                if authorization_information.len() != 2 {
                    error!("It seems that the authorization header is malformed. There were 2 parts expected but we got {}", authorization_information.len());
                    return Outcome::Failure((
                        Status::Forbidden,
                        AuthorizationError::MalformedAuthorizationHeader,
                    ));
                }

                // we support bearer and pat authentication flows
                return match authorization_information[0].to_lowercase().as_ref() {
                    "bearer" => Self::bearer_flow(authorization_information, request).await,
                    "pat" => Self::pat_flow(authorization_information, request).await,
                    _ => {
                        error!("It seems that the authorization header is malformed. We expected as token type 'bearer' or 'pat' but got '{}'", authorization_information[0].to_lowercase());
                        Outcome::Failure((
                            Status::Forbidden,
                            AuthorizationError::MalformedAuthorizationHeader,
                        ))
                    }
                };
            }
            _ => {
                error!("No authorization header could be found for an authenticated route!");
                Outcome::Failure((
                    Status::Forbidden,
                    AuthorizationError::MissingAuthorizationHeader,
                ))
            }
        }
    }
}
