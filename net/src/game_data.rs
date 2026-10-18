use crate::error::NetworkError;
use game_data::GameData;
use std::sync::OnceLock;

static GAME_DATA: OnceLock<Result<GameData, String>> = OnceLock::new();

pub fn get() -> Result<&'static GameData, NetworkError> {
    let state = GAME_DATA.get_or_init(|| {
        let path = crate::load::asset_path("RUSTMS_MAP_NX_PATH", "assets/game-data/Map.nx");

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

/// The id of the spawn point on `map_id` nearest `(x, y)`, to save as the
/// character's spawn point when they leave the game.
pub fn closest_spawn_point(map_id: i32, x: i16, y: i16) -> Option<u8> {
    let field = get().ok()?.field(map_id)?;
    let portal = field.closest_spawn_portal(i32::from(x), i32::from(y))?;
    u8::try_from(portal.id).ok()
}
