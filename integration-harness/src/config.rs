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
    username: Option<String>,
    password: Option<String>,
    character_name: Option<String>,
    gender: Option<u8>,
    login_addr: Option<String>,
    world_addr: Option<String>,
    players: Option<Vec<HarnessPlayerConfigFile>>,
}

#[derive(Debug, Deserialize)]
struct HarnessPlayerConfigFile {
    role: String,
    username: String,
    password: String,
    character_name: String,
    gender: Option<u8>,
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

#[derive(Debug, Clone)]
pub struct HarnessPlayerConfig {
    pub role: String,
    pub username: String,
    pub password: String,
    pub character_name: String,
    pub gender: u8,
}

#[derive(Debug, Clone)]
pub struct MultiHarnessConfig {
    pub login_addr: SocketAddr,
    pub world_addr: SocketAddr,
    pub players: Vec<HarnessPlayerConfig>,
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

        let raw = read_raw_config(path)?;

        let login_addr = parse_addr(
            "login_addr",
            raw.login_addr
                .unwrap_or_else(|| DEFAULT_LOGIN_ADDR.to_string()),
        )?;
        let world_addr = parse_addr(
            "world_addr",
            raw.world_addr
                .unwrap_or_else(|| DEFAULT_WORLD_ADDR.to_string()),
        )?;

        Ok(Self {
            username: require_field("username", raw.username)?,
            password: require_field("password", raw.password)?,
            character_name: require_field("character_name", raw.character_name)?,
            gender: raw.gender.unwrap_or(0),
            login_addr,
            world_addr,
        })
    }
}

impl MultiHarnessConfig {
    pub fn from_file() -> Result<Self, HarnessError> {
        Self::from_path(default_config_path())
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, HarnessError> {
        let path = path.as_ref();
        let raw = read_raw_config(path)?;
        let login_addr = parse_addr(
            "login_addr",
            raw.login_addr
                .unwrap_or_else(|| DEFAULT_LOGIN_ADDR.to_string()),
        )?;
        let world_addr = parse_addr(
            "world_addr",
            raw.world_addr
                .unwrap_or_else(|| DEFAULT_WORLD_ADDR.to_string()),
        )?;

        let players = raw.players.ok_or_else(|| {
            HarnessError::fixture(
                "integration config",
                "expected at least two [[players]] entries in integration-harness.toml",
            )
        })?;
        if players.len() < 2 {
            return Err(HarnessError::fixture(
                "integration config",
                "expected at least two [[players]] entries in integration-harness.toml",
            ));
        }

        let players = players
            .into_iter()
            .map(|player| HarnessPlayerConfig {
                role: player.role,
                username: player.username,
                password: player.password,
                character_name: player.character_name,
                gender: player.gender.unwrap_or(0),
            })
            .collect::<Vec<_>>();

        let sender_count = players
            .iter()
            .filter(|player| player.role == "sender")
            .count();
        let recipient_count = players
            .iter()
            .filter(|player| player.role == "recipient")
            .count();
        if sender_count != 1 || recipient_count != 1 {
            return Err(HarnessError::fixture(
                "integration config",
                format!(
                    "expected exactly one sender and one recipient player, got sender={} recipient={}",
                    sender_count, recipient_count
                ),
            ));
        }

        Ok(Self {
            login_addr,
            world_addr,
            players,
        })
    }

    pub fn player(&self, role: &str) -> Result<HarnessConfig, HarnessError> {
        let player = self
            .players
            .iter()
            .find(|player| player.role == role)
            .ok_or_else(|| {
                HarnessError::fixture(
                    "integration config",
                    format!("missing player with role `{role}`"),
                )
            })?;

        Ok(HarnessConfig {
            username: player.username.clone(),
            password: player.password.clone(),
            character_name: player.character_name.clone(),
            gender: player.gender,
            login_addr: self.login_addr,
            world_addr: self.world_addr,
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

fn read_raw_config(path: &Path) -> Result<HarnessConfigFile, HarnessError> {
    if !path.exists() {
        return Err(HarnessError::MissingConfigFile {
            path: path.display().to_string(),
        });
    }

    let contents = fs::read_to_string(path).map_err(|source| HarnessError::ConfigRead {
        path: path.display().to_string(),
        source,
    })?;

    toml::from_str(&contents).map_err(|source| HarnessError::ConfigParse {
        path: path.display().to_string(),
        message: source.to_string(),
    })
}

fn require_field(name: &'static str, value: Option<String>) -> Result<String, HarnessError> {
    value.ok_or_else(|| {
        HarnessError::fixture(
            "integration config",
            format!("missing required top-level field `{name}` in integration-harness.toml"),
        )
    })
}

fn parse_addr(name: &'static str, value: String) -> Result<SocketAddr, HarnessError> {
    value
        .parse()
        .map_err(|source| HarnessError::InvalidAddress {
            name,
            value,
            source,
        })
}
