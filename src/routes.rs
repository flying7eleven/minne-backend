use crate::fairings::MinneDatabaseConnection;
use rocket::get;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;

#[derive(Serialize)]
pub struct VersionInformation {
    /// The version of the backend which is currently running.
    pub backend_version: &'static str,
    /// The architecture the backend was build for.
    pub backend_arch: &'static str,
    /// The version of the rustc compiler used to compile the backend.
    pub rustc_version: &'static str,
    /// The date on which the backend was build.
    pub build_date: &'static str,
    /// The time on which the backend was build.
    pub build_time: &'static str,
}

#[get("/version")]
pub async fn get_backend_version() -> Json<VersionInformation> {
    Json(VersionInformation {
        backend_version: env!("VERGEN_GIT_SEMVER"),
        backend_arch: env!("VERGEN_CARGO_TARGET_TRIPLE"),
        rustc_version: env!("VERGEN_RUSTC_SEMVER"),
        build_date: env!("VERGEN_BUILD_DATE"),
        build_time: env!("VERGEN_BUILD_TIME"),
    })
}

#[derive(Serialize)]
pub struct HealthCheck {
    /// A flag which indicates if the database is healthy or not.
    pub database_healthy: bool,
    /// A flag which indicates if the backend itself is healthy or not.
    pub backend_healthy: bool,
}

#[get("/health")]
pub async fn check_backend_health(
    db_connection_pool: &State<MinneDatabaseConnection>,
) -> Result<Json<HealthCheck>, Status> {
    use crate::schema::users::dsl::users;
    use diesel::dsl::count_star;
    use diesel::{QueryDsl, RunQueryDsl};
    use log::{debug, error};

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

    // check if the connection to the database is working or not
    let database_is_healthy = db_connection
        .build_transaction()
        .read_only()
        .run::<_, diesel::result::Error, _>(|connection| {
            if let Err(error) = users.select(count_star()).first::<i64>(connection) {
                error!("The health check of the database connection failed with the following error: {}", error);
                return Err(error);
            }
            debug!("Last health check was successful");
            return Ok(());
        });

    // if the database is healthy, we can return the status immediately
    if database_is_healthy.is_ok() {
        return Ok(Json(HealthCheck {
            database_healthy: true,
            backend_healthy: true,
        }));
    }

    // if seems that the health check failed, indicate that by returning a 500
    Err(Status::InternalServerError)
}
