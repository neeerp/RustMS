use crate::load::asset_path;
use game_data::{MapNpcTemplate, NpcDialogueData};
use std::sync::OnceLock;

static NPC_DIALOGUES: OnceLock<Result<NpcDialogueData, String>> = OnceLock::new();

pub fn get() -> Result<&'static NpcDialogueData, &'static str> {
    let state = NPC_DIALOGUES.get_or_init(|| {
        let npc_path = asset_path("RUSTMS_NPC_NX_PATH", "assets/game-data/Npc.nx");
        let string_path = asset_path("RUSTMS_STRING_NX_PATH", "assets/game-data/String.nx");
        NpcDialogueData::load_from_nx(&npc_path, &string_path).map_err(|error| {
            format!(
                "failed to load NPC speech from {} and {}: {error}",
                npc_path.display(),
                string_path.display()
            )
        })
    });
    state.as_ref().map_err(|message| message.as_str())
}

/// NPCs the server stands in maps whose data leaves them out. v83's Map.nx
/// places the Rock, Paper, Scissors Admin nowhere (report b038); here it
/// stands on Henesys's lower street, by Spinel.
const EXTRA_NPCS: &[(i32, MapNpcTemplate)] = &[(
    100_000_000,
    MapNpcTemplate {
        npc_id: crate::rps::RPS_ADMIN,
        x: 2190,
        y: 324,
        foothold: 124,
        flip: false,
        rx0: 2140,
        rx1: 2240,
    },
)];

/// Rooney (`go_xmas`), who takes players to Happyville from 23 towns.
pub const ROONEY: i32 = 1_022_101;

/// Whether Happyville is open, so that Rooney stands in the towns: unless
/// `RUSTMS_HAPPYVILLE=0`. Rupi, its way out, stays either way, since
/// Happyville returns players to itself.
pub fn happyville_open() -> bool {
    static OPEN: OnceLock<bool> = OnceLock::new();
    *OPEN.get_or_init(|| std::env::var("RUSTMS_HAPPYVILLE").map_or(true, |value| value != "0"))
}

/// Whether `npc_id`, placed on a map by its data, stands there.
pub fn npc_stands(npc_id: i32) -> bool {
    npc_id != ROONEY || happyville_open()
}

/// The NPCs the server adds to `map_id`, after those of its data.
pub fn extra_npcs(map_id: i32) -> impl Iterator<Item = &'static MapNpcTemplate> {
    EXTRA_NPCS
        .iter()
        .filter(move |(map, _)| *map == map_id)
        .map(|(_, npc)| npc)
}
