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

/// Where a character stands after spawning at `spawn_point` on `map_id`,
/// according to the map's portal data.
pub fn spawn_position(map_id: i32, spawn_point: u8) -> Option<crate::SpawnPosition> {
    let field = get().ok()?.field(map_id)?;
    let portal = field
        .portal_by_id(u32::from(spawn_point))
        .or_else(|| field.resolve_spawn_portal(""))?;
    Some(crate::SpawnPosition {
        x: i16::try_from(portal.x).ok()?,
        y: i16::try_from(portal.y).ok()?,
        stance: 2,
    })
}
