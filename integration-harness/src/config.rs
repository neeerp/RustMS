use crate::error::HarnessError;
use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

const DEFAULT_LOGIN_ADDR: &str = "127.0.0.1:8484";
const DEFAULT_WORLD_ADDR: &str = "127.0.0.1:8485";
const CONFIG_FILE_NAME: &str = "integration-harness.toml";

#[derive(Debug, Deserialize)]
struct HarnessConfigFile {
    username: String,
    password: String,
    character_name: String,
    gender: Option<u8>,
    login_addr: Option<String>,
    world_addr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HarnessConfig {
    pub username: String,
    pub password: String,
    pub character_name: String,
    pub gender: u8,
    pub login_addr: SocketAddr,
    pub world_addr: SocketAddr,
}

impl HarnessConfig {
    pub fn from_file() -> Result<Self, HarnessError> {
        Self::from_path(default_config_path())
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, HarnessError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(HarnessError::MissingConfigFile {
                path: path.display().to_string(),
            });
        }

        let contents = fs::read_to_string(path).map_err(|source| HarnessError::ConfigRead {
            path: path.display().to_string(),
            source,
        })?;

        let raw: HarnessConfigFile =
            toml::from_str(&contents).map_err(|source| HarnessError::ConfigParse {
                path: path.display().to_string(),
                message: source.to_string(),
            })?;

        let login_addr = parse_addr(
            "login_addr",
            raw.login_addr.unwrap_or_else(|| DEFAULT_LOGIN_ADDR.to_string()),
        )?;
        let world_addr = parse_addr(
            "world_addr",
            raw.world_addr.unwrap_or_else(|| DEFAULT_WORLD_ADDR.to_string()),
        )?;

        Ok(Self {
            username: raw.username,
            password: raw.password,
            character_name: raw.character_name,
            gender: raw.gender.unwrap_or(0),
            login_addr,
            world_addr,
        })
    }
}

fn default_config_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("integration-harness crate must live under the workspace root")
        .join(CONFIG_FILE_NAME)
}

fn parse_addr(name: &'static str, value: String) -> Result<SocketAddr, HarnessError> {
    value.parse().map_err(|source| HarnessError::InvalidAddress {
        name,
        value,
        source,
    })
}
