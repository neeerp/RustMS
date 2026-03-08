use config::ConfigError;
use std::convert::TryFrom;
use std::env;
use std::net::Ipv4Addr;

const DEFAULT_WORLD_ID: u8 = 0;
const DEFAULT_WORLD_NAME: &str = "Scania";
const DEFAULT_WORLD_FLAG: u8 = 0;
const DEFAULT_EVENT_MESSAGE: &str = "Test!";
const DEFAULT_CHANNEL_COUNT: u8 = 3;
const DEFAULT_CHANNEL_HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_CHANNEL_PORT: u16 = 8485;
const DEFAULT_CHANNEL_CAPACITY: u16 = 700;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginChannel {
    pub world_id: u8,
    pub channel_id: u8,
    pub name: String,
    pub host: Ipv4Addr,
    pub port: u16,
    pub capacity: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginWorld {
    pub world_id: u8,
    pub name: String,
    pub flag: u8,
    pub event_message: String,
    pub channels: Vec<LoginChannel>,
}

pub fn load_login_worlds() -> Result<Vec<LoginWorld>, ConfigError> {
    match env::var("RUSTMS_LOGIN_CHANNELS") {
        Ok(spec) => parse_env_worlds(spec),
        Err(_) => Ok(vec![default_world()]),
    }
}

pub fn resolve_login_channel(world_id: u8, channel_id: u8) -> Result<LoginChannel, ConfigError> {
    let worlds = load_login_worlds()?;
    let world = worlds
        .into_iter()
        .find(|world| world.world_id == world_id)
        .ok_or_else(|| ConfigError::Message(format!("unknown world id {world_id}")))?;

    world
        .channels
        .into_iter()
        .find(|channel| channel.channel_id == channel_id)
        .ok_or_else(|| {
            ConfigError::Message(format!(
                "unknown channel id {channel_id} for world id {world_id}"
            ))
        })
}

fn parse_env_worlds(spec: String) -> Result<Vec<LoginWorld>, ConfigError> {
    let world_id = env::var("RUSTMS_LOGIN_WORLD_ID")
        .ok()
        .map(|value| parse_u8_env("RUSTMS_LOGIN_WORLD_ID", &value))
        .transpose()?
        .unwrap_or(DEFAULT_WORLD_ID);
    let world_name =
        env::var("RUSTMS_LOGIN_WORLD_NAME").unwrap_or_else(|_| DEFAULT_WORLD_NAME.to_string());
    let flag = env::var("RUSTMS_LOGIN_WORLD_FLAG")
        .ok()
        .map(|value| parse_u8_env("RUSTMS_LOGIN_WORLD_FLAG", &value))
        .transpose()?
        .unwrap_or(DEFAULT_WORLD_FLAG);
    let event_message = env::var("RUSTMS_LOGIN_EVENT_MESSAGE")
        .unwrap_or_else(|_| DEFAULT_EVENT_MESSAGE.to_string());

    let channels = spec
        .split(';')
        .filter(|entry| !entry.trim().is_empty())
        .enumerate()
        .map(|(index, entry)| parse_channel_entry(world_id, index, entry.trim()))
        .collect::<Result<Vec<_>, _>>()?;

    if channels.is_empty() {
        return Err(ConfigError::Message(
            "RUSTMS_LOGIN_CHANNELS must define at least one channel".to_string(),
        ));
    }

    Ok(vec![LoginWorld {
        world_id,
        name: world_name,
        flag,
        event_message,
        channels,
    }])
}

fn parse_channel_entry(
    world_id: u8,
    index: usize,
    entry: &str,
) -> Result<LoginChannel, ConfigError> {
    let mut parts = entry.split('@');
    let name = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_channel_config(entry))?;
    let endpoint = parts.next().ok_or_else(|| invalid_channel_config(entry))?;
    let capacity = parts.next().ok_or_else(|| invalid_channel_config(entry))?;
    if parts.next().is_some() {
        return Err(invalid_channel_config(entry));
    }

    let (host, port) = parse_endpoint(endpoint)?;
    let capacity = capacity
        .parse::<u16>()
        .map_err(|_| invalid_channel_config(entry))?;
    let channel_id = u8::try_from(index).map_err(|_| invalid_channel_config(entry))?;

    Ok(LoginChannel {
        world_id,
        channel_id,
        name: name.to_string(),
        host,
        port,
        capacity,
    })
}

fn parse_endpoint(endpoint: &str) -> Result<(Ipv4Addr, u16), ConfigError> {
    let (host, port) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| invalid_channel_config(endpoint))?;
    let host = host
        .parse::<Ipv4Addr>()
        .map_err(|_| invalid_channel_config(endpoint))?;
    let port = port
        .parse::<u16>()
        .map_err(|_| invalid_channel_config(endpoint))?;
    Ok((host, port))
}

fn parse_u8_env(name: &str, value: &str) -> Result<u8, ConfigError> {
    value
        .parse::<u8>()
        .map_err(|_| ConfigError::Message(format!("{name} must be a valid u8")))
}

fn invalid_channel_config(value: &str) -> ConfigError {
    ConfigError::Message(format!(
        "invalid channel config `{value}`; expected `name@host:port@capacity`"
    ))
}

fn default_world() -> LoginWorld {
    let channels = (0..DEFAULT_CHANNEL_COUNT)
        .map(|channel_id| LoginChannel {
            world_id: DEFAULT_WORLD_ID,
            channel_id,
            name: format!("{DEFAULT_WORLD_NAME}-{}", channel_id + 1),
            host: DEFAULT_CHANNEL_HOST,
            port: DEFAULT_CHANNEL_PORT,
            capacity: DEFAULT_CHANNEL_CAPACITY,
        })
        .collect();

    LoginWorld {
        world_id: DEFAULT_WORLD_ID,
        name: DEFAULT_WORLD_NAME.to_string(),
        flag: DEFAULT_WORLD_FLAG,
        event_message: DEFAULT_EVENT_MESSAGE.to_string(),
        channels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_world_points_all_channels_to_same_runtime() {
        let world = default_world();
        assert_eq!(world.channels.len(), 3);
        assert!(world.channels.iter().all(|channel| channel.port == 8485));
        assert_eq!(world.channels[1].channel_id, 1);
    }

    #[test]
    fn parses_env_channel_spec() {
        let worlds = parse_env_worlds(
            "Scania-1@127.0.0.1:18485@700;Scania-2@127.0.0.1:19485@600".to_string(),
        )
        .expect("parsed channel config");

        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].channels[0].port, 18485);
        assert_eq!(worlds[0].channels[1].channel_id, 1);
        assert_eq!(worlds[0].channels[1].capacity, 600);
    }
}
