#[macro_use]
extern crate diesel;

pub mod fairings;
pub mod routes {
    pub mod auth;
    pub mod health;
    pub mod user;
    pub mod version;
}
pub mod guards;
pub mod schema;
