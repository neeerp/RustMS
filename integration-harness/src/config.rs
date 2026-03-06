use crate::error::HarnessError;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;

const DEFAULT_LOGIN_ADDR: &str = "127.0.0.1:8484";
const DEFAULT_WORLD_ADDR: &str = "127.0.0.1:8485";

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
    pub fn random(login_addr: SocketAddr, world_addr: SocketAddr) -> Self {
        Self {
            username: random_name("iu"),
            password: "test".to_string(),
            character_name: random_name("ic"),
            gender: 0,
            login_addr,
            world_addr,
        }
    }
}

impl MultiHarnessConfig {
    pub fn random_pair(login_addr: SocketAddr, world_addr: SocketAddr) -> Self {
        Self {
            login_addr,
            world_addr,
            players: vec![
                HarnessPlayerConfig {
                    role: "sender".to_string(),
                    username: random_name("is"),
                    password: "test".to_string(),
                    character_name: random_name("cs"),
                    gender: 0,
                },
                HarnessPlayerConfig {
                    role: "recipient".to_string(),
                    username: random_name("ir"),
                    password: "test".to_string(),
                    character_name: random_name("cr"),
                    gender: 0,
                },
            ],
        }
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

pub fn harness_addrs_from_env() -> Result<(SocketAddr, SocketAddr), HarnessError> {
    let login =
        std::env::var("HARNESS_LOGIN_ADDR").unwrap_or_else(|_| DEFAULT_LOGIN_ADDR.to_string());
    let world =
        std::env::var("HARNESS_WORLD_ADDR").unwrap_or_else(|_| DEFAULT_WORLD_ADDR.to_string());
    Ok((
        parse_addr("HARNESS_LOGIN_ADDR", login)?,
        parse_addr("HARNESS_WORLD_ADDR", world)?,
    ))
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

fn random_name(prefix: &str) -> String {
    let mut rng = thread_rng();
    let suffix = (&mut rng)
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .map(|ch| ch.to_ascii_lowercase())
        .take(10)
        .collect::<String>();
    let mut candidate = format!("{prefix}{suffix}");
    candidate.truncate(13);
    candidate
}
