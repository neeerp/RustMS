extern crate serde;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate serde_derive;

use diesel::pg::PgConnection;
use diesel::prelude::*;

pub mod accounts;
pub mod models;
pub mod schema;
mod settings;

use settings::Settings;

pub fn establish_connection() -> PgConnection {
    let database_url = Settings::new().unwrap().database.url;
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}
