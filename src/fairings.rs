use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::PgConnection;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};

/// Wrapper around a connection pool for the database access
pub struct MinneDatabaseConnection(Pool<ConnectionManager<PgConnection>>);

/// The actual connection to the database obtained from the pool
impl MinneDatabaseConnection {
    #[inline(always)]
    pub fn get(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, r2d2::Error> {
        self.0.get()
    }
}

/// Implementation of the From trait to get a single database connection from a connection pool
impl From<Pool<ConnectionManager<PgConnection>>> for MinneDatabaseConnection {
    #[inline(always)]
    fn from(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        MinneDatabaseConnection(pool)
    }
}

#[derive(Clone)]
pub struct BackendConfiguration {
    /// The pre-shared-key which is used to sign and validate the generated token.
    pub token_signature_psk: String,
    /// The access token-lifetime in seconds.
    pub access_token_lifetime_in_seconds: usize,
    /// The refresh token-lifetime in seconds.
    pub refresh_token_lifetime_in_seconds: usize,
    /// Whether or not the user registration is enabled.
    pub user_registration_enabled: bool,
}

/// The fairing which can be used for setting a cache-control
/// header which instructs the calling party to not cache the
/// API result.
pub struct NoCacheFairing;

#[rocket::async_trait]
impl Fairing for NoCacheFairing {
    /// Get some generic information about this fairing.
    fn info(&self) -> Info {
        Info {
            name: "Ensure the client is instructed to not cache the result.",
            kind: Kind::Response,
        }
    }

    /// Ensure that each response has the corresponding header set.
    async fn on_response<'r>(&self, _: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_raw_header("Cache-Control", "no-cache");
    }
}
