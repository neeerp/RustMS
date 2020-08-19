use config::{Config, ConfigError, File};

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
