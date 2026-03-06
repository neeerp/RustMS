use config::{Config, ConfigError, File};
use std::env;

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: Database,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        if let Ok(url) = env::var("RUSTMS_DATABASE_URL") {
            return Ok(Self {
                database: Database { url },
            });
        }

        let mut s = Config::new();

        s.merge(File::with_name("config/db_config"))?;

        s.try_into()
    }
}
