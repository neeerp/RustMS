use crate::error::NetworkError;
use game_data::GameData;
use std::path::PathBuf;
use std::sync::OnceLock;

static GAME_DATA: OnceLock<Result<GameData, String>> = OnceLock::new();

pub fn get() -> Result<&'static GameData, NetworkError> {
    let state = GAME_DATA.get_or_init(|| {
        let path = std::env::var("RUSTMS_MAP_NX_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("assets/game-data/Map.nx"));

        GameData::load_from_nx_map(&path).map_err(|error| {
            format!(
                "failed to load game data from '{}': {}",
                path.display(),
                error
            )
        })
    });

    match state {
        Ok(data) => Ok(data),
        Err(message) => {
            eprintln!("{message}");
            Err(NetworkError::PacketHandlerError("Failed to load game data"))
        }
    }
}
