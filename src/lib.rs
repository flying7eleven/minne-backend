#[macro_use]
extern crate diesel;

pub mod fairings;
pub mod models;
pub mod routes {
    pub mod health;
    pub mod version;
}
pub mod schema;
