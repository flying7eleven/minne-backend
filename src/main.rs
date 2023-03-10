use diesel::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use log::LevelFilter;
use rocket::figment::map;
use rocket::get;
use rocket_dyn_templates::Template;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[inline(always)]
fn unset_environment_variable(name: &str) {
    std::env::remove_var(name)
}

pub fn run_migrations(connection: &mut PgConnection) {
    use diesel_migrations::MigrationHarness;
    use log::{error, info};
    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(ran_migrations) => {
            if !ran_migrations.is_empty() {
                info!(
                    "Successfully ran {} database migrations",
                    ran_migrations.len()
                );
            } else {
                info!("No migrations had to be run since the database is up to date");
            }
        }
        Err(error) => {
            error!(
                "Failed to run the database migrations. The error was: {}",
                error
            )
        }
    }
}

fn setup_logging(logging_level: LevelFilter) {
    use chrono::Utc;

    // create an instance for the Dispatcher to create a new logging configuration
    let mut base_config = fern::Dispatch::new();

    // set the corresponding logging level
    base_config = base_config.level(logging_level);

    // define how a logging line should look like and attach the streams to which the output will be
    // written to
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Utc::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(std::io::stderr());

    // now chain everything together and get ready for actually logging stuff
    base_config
        .chain(file_config)
        .level_for("reqwest", LevelFilter::Off)
        .level_for("want", LevelFilter::Off)
        .level_for("mio", LevelFilter::Off)
        .level_for("rocket_dyn_templates", LevelFilter::Error)
        .level_for("handlebars::render", LevelFilter::Error)
        .level_for("rocket", LevelFilter::Error)
        .level_for("_", LevelFilter::Error)
        .apply()
        .unwrap();
}

#[get("/")]
fn show_login_page_to_user() -> Template {
    use uuid::Uuid;
    let context = map! {
        "title" => "Minne iOS App Login".to_owned(),
        "login_process_id" => Uuid::new_v4().to_string(),
    };
    Template::render("login_from_app", &context)
}

#[rocket::main]
async fn main() {
    use log::{debug, error, info};
    use minne_backend::fairings::{BackendConfiguration, MinneDatabaseConnection, NoCacheFairing};
    use minne_backend::routes::{
        auth::authenticate_app_with_pat, auth::disable_pat, auth::get_authentication_token,
        health::check_backend_health, task::add_new_task, task::delete_task, task::edit_task,
        task::get_all_task_ids_from_user, task::get_task, user::create_new_user,
        version::get_backend_version,
    };
    use rocket::config::{Shutdown, Sig};
    use rocket::figment::{
        util::map,
        value::{Map, Value},
    };
    use rocket::fs::FileServer;
    use rocket::http::Method;
    use rocket::routes;
    use rocket::Config as RocketConfig;
    use rocket_cors::{AllowedHeaders, AllowedOrigins};
    use std::env;

    // select the logging level from a set environment variable
    let logging_level = match env::var("MINNE_LOGGING_LEVEL") {
        Ok(value) => match value.to_lowercase().as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        },
        Err(_) => LevelFilter::Info,
    };

    // setup the logging of the application based on the environment variable
    setup_logging(logging_level);

    // just inform the user that we are starting up
    info!(
        "Starting Minne backend ({}, build with rustc {})...",
        env!("VERGEN_GIT_SEMVER"),
        env!("VERGEN_RUSTC_SEMVER")
    );

    // get the configuration for the database server and terminate if something is missing
    let database_connection_url =
        env::var("MINNE_DB_CONNECTION").unwrap_or_else(|_| "".to_string());
    if database_connection_url.is_empty() {
        error!("Could not get the configuration for the database server. Ensure MINNE_DB_CONNECTION is set properly");
        return;
    }

    // get the psk for the token signature
    let token_signature_psk =
        env::var("MINNE_TOKEN_SIGNATURE_PSK").unwrap_or_else(|_| "".to_string());
    if token_signature_psk.is_empty() {
        error!("Could not get the token signature PSK. Ensure MINNE_TOKEN_SIGNATURE_PSK is set properly");
        return;
    }

    // get the access token life time in seconds
    let access_token_lifetime_in_seconds = env::var("MINNE_ACCESS_TOKEN_LIFETIME_IN_SECONDS")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<usize>()
        .unwrap_or(300);

    // get the refresh token life time in seconds
    let refresh_token_lifetime_in_seconds = env::var("MINNE_REFRESH_TOKEN_LIFETIME_IN_SECONDS")
        .unwrap_or_else(|_| "3600".to_string())
        .parse::<usize>()
        .unwrap_or(3600);

    // get the refresh token life time in seconds
    let user_registration_enabled = env::var("MINNE_ENABLE_USER_REGISTRATION")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // create a struct which holds the whole configuration
    let backend_config = BackendConfiguration {
        token_signature_psk: token_signature_psk.to_string(),
        access_token_lifetime_in_seconds,
        refresh_token_lifetime_in_seconds,
        user_registration_enabled,
    };

    // just wait for 10 seconds until we continue. This is just an ugly fix that we have to wait until the database server
    // has spun up
    #[cfg(not(debug_assertions))]
    {
        info!("Waiting for 10 seconds to ensure that the database had enough time to spin up...");
        std::thread::sleep(std::time::Duration::from_secs(10));
    }

    // create a db connection pool manager and the corresponding pool
    let db_connection_pool_manager =
        diesel::r2d2::ConnectionManager::new(database_connection_url.clone());
    let db_connection_pool = r2d2::Pool::builder()
        .max_size(15)
        .build(db_connection_pool_manager)
        .unwrap();
    debug!("Successfully connected to the database server");

    // ensure the database is setup correctly
    run_migrations(&mut db_connection_pool.get().unwrap_or_else(|e| {
        error!(
            "Could not get a database connection from the connection pool. The error was: {}",
            e
        );
        std::process::exit(-1);
    }));
    info!("Database preparations finished");

    // configure the database pool based on the supplied connection URL
    let minne_database_config: Map<_, Value> = map! {
        "url" => database_connection_url.into(),
        "pool_size" => 25.into()
    };

    // rocket configuration figment
    #[allow(unused_mut)]
    let mut rocket_configuration_figment = RocketConfig::figment()
        .merge(("databases", map!["saker" => minne_database_config]))
        .merge(("port", 5842))
        .merge(("address", std::net::Ipv4Addr::new(0, 0, 0, 0)))
        .merge((
            "shutdown",
            Shutdown {
                ctrlc: true,
                signals: {
                    let mut set = std::collections::HashSet::new();
                    set.insert(Sig::Term);
                    set
                },
                grace: 2,
                mercy: 3,
                force: true,
                __non_exhaustive: (),
            },
        ));

    // if we are in debug mode, we are using the local template directory; otherwise use the installation directory
    #[cfg(not(debug_assertions))]
    {
        rocket_configuration_figment = rocket_configuration_figment
            .merge(("template_dir", "/usr/local/share/minne-backend/templates"));
    }

    // if we are in debug mode, we are using the local static file directory; otherwise use the installation directory
    let static_file_directory = if cfg!(debug_assertions) {
        "static"
    } else {
        "/usr/local/share/minne-backend/static"
    };

    // prepare the fairing for the CORS headers
    let allowed_origins = AllowedOrigins::All;
    let cors_header = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Patch,
            Method::Delete,
        ]
        .into_iter()
        .map(From::from)
        .collect(),
        allowed_headers: AllowedHeaders::All,
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .unwrap();

    // create a fairing which is setting a cache control header to not cache
    // the API responses
    let no_cache_header = NoCacheFairing {};

    // after everything is set up, we should unset ann environment variables to prevent leaking
    // sensitive information
    unset_environment_variable("MINNE_LOGGING_LEVEL");
    unset_environment_variable("MINNE_DB_CONNECTION");
    unset_environment_variable("MINNE_TOKEN_SIGNATURE_PSK");

    // mount all supported routes and launch the rocket :)
    info!("Server started and the routes are ready to process queries");
    let _ = rocket::custom(rocket_configuration_figment)
        .attach(cors_header)
        .attach(no_cache_header)
        .attach(Template::fairing())
        .manage(backend_config)
        .manage(MinneDatabaseConnection::from(db_connection_pool))
        .mount(
            "/v1",
            routes![
                check_backend_health,
                get_backend_version,
                create_new_user,
                get_authentication_token,
                add_new_task,
                delete_task,
                get_all_task_ids_from_user,
                disable_pat,
                get_task,
                edit_task,
                authenticate_app_with_pat,
            ],
        )
        .mount("/", routes![show_login_page_to_user,])
        .mount("/static", FileServer::from(static_file_directory))
        .launch()
        .await;
}
