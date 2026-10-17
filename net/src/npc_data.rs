use crate::load::asset_path;
use game_data::NpcDialogueData;
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
