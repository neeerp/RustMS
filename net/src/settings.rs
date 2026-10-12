use config::{Config, ConfigError, File};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct Login {
    pub pin_required: bool,
    pub gender_required: bool,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub login: Login,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name("config/login_server_config"))?;

        s.try_into()
    }
}

/// The login server settings, read from `config/login_server_config` once.
pub fn login_settings() -> Result<&'static Settings, ConfigError> {
    static SETTINGS: OnceLock<Result<Settings, String>> = OnceLock::new();
    SETTINGS
        .get_or_init(|| Settings::new().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|message| ConfigError::Message(message.clone()))
}
