extern crate serde;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel_derive_enum;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use settings::Settings;
use std::sync::OnceLock;
use std::time::Duration;

pub mod schema;
mod settings;
mod sql_types;

pub mod account;
pub mod character;
pub mod keybinding;
pub mod quest;
pub mod session;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;
pub type Result<T> = std::result::Result<T, Error>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
/// Arbitrary key for the Postgres advisory lock that serializes migrations
/// when several server processes start against the same database.
const MIGRATION_LOCK_KEY: i64 = 0x5275_7374_4d53;
const DEFAULT_POOL_SIZE: u32 = 16;

/// Errors surfaced by the persistence layer. Diesel's error type stays an
/// implementation detail except through the `Query` variant.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("record not found")]
    NotFound,
    #[error("database query failed: {0}")]
    Query(diesel::result::Error),
    #[error("database unavailable: {0}")]
    Unavailable(String),
    #[error("database configuration error: {0}")]
    Config(String),
    #[error("database migration failed: {0}")]
    Migration(String),
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Error::NotFound,
            other => Error::Query(other),
        }
    }
}

impl From<diesel::r2d2::PoolError> for Error {
    fn from(error: diesel::r2d2::PoolError) -> Self {
        Error::Unavailable(error.to_string())
    }
}

static POOL: OnceLock<DbPool> = OnceLock::new();

/// The process-wide connection pool, created on first use from the
/// configured database URL.
pub fn pool() -> Result<&'static DbPool> {
    if let Some(pool) = POOL.get() {
        return Ok(pool);
    }

    let settings = Settings::new().map_err(|e| Error::Config(e.to_string()))?;
    let size = std::env::var("RUSTMS_DB_POOL_SIZE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_POOL_SIZE);
    let pool = Pool::builder()
        .max_size(size)
        .connection_timeout(Duration::from_secs(10))
        .build(ConnectionManager::<PgConnection>::new(settings.database.url))?;

    // Another thread may have raced us; either pool is equivalent.
    Ok(POOL.get_or_init(|| pool))
}

/// Check out a pooled connection.
pub fn connection() -> Result<DbConnection> {
    Ok(pool()?.get()?)
}

/// Apply any pending embedded migrations. Safe to call from several
/// processes at once: an advisory lock serializes them.
pub fn run_migrations() -> Result<usize> {
    let mut conn = connection()?;
    diesel::sql_query(format!("SELECT pg_advisory_lock({MIGRATION_LOCK_KEY})"))
        .execute(&mut conn)?;
    let applied = conn
        .run_pending_migrations(MIGRATIONS)
        .map(|versions| versions.len())
        .map_err(|e| Error::Migration(e.to_string()));
    diesel::sql_query(format!("SELECT pg_advisory_unlock({MIGRATION_LOCK_KEY})"))
        .execute(&mut conn)?;
    applied
}
