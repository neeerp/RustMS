extern crate serde;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use settings::Settings;

pub mod schema;
mod settings;

pub mod account;
pub mod character;

pub use diesel::result::Error;

pub fn establish_connection() -> PgConnection {
    // TODO: This needs proper error handling...
    let database_url = Settings::new().unwrap().database.url;
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}
