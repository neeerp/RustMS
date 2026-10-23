//! The server content pack (script text, drop tables, shops, Gachapon
//! prizes), loaded once.
//! See `game_data::server_content` and `agent_docs/server-content.md`.

use game_data::server_content::{GachaponTable, RewardTable, ServerText, ShopTable};
use std::path::PathBuf;
use std::sync::OnceLock;

static TEXT: OnceLock<ServerText> = OnceLock::new();
static REWARDS: OnceLock<RewardTable> = OnceLock::new();
static SHOPS: OnceLock<ShopTable> = OnceLock::new();
static GACHAPON: OnceLock<GachaponTable> = OnceLock::new();

/// `RUSTMS_SERVER_CONTENT_PATH`, default `assets/server-content`.
pub fn pack_path() -> PathBuf {
    std::env::var("RUSTMS_SERVER_CONTENT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("assets/server-content"))
}

/// Script text. A broken pack falls back to placeholders rather than
/// stopping the server; `load` reports the problem at startup.
pub fn text() -> &'static ServerText {
    TEXT.get_or_init(|| {
        ServerText::load(Some(&pack_path()))
            .or_else(|_| ServerText::load(None))
            .expect("committed placeholder text must parse")
    })
}

pub fn rewards() -> &'static RewardTable {
    REWARDS.get_or_init(|| {
        RewardTable::load(Some(&pack_path()))
            .or_else(|_| RewardTable::load(None))
            .expect("committed reward overrides must parse")
    })
}

pub fn shops() -> &'static ShopTable {
    SHOPS.get_or_init(|| ShopTable::load(Some(&pack_path())).unwrap_or_default())
}

/// Gachapon prizes; a broken pack falls back to the stand-ins.
pub fn gachapon() -> &'static GachaponTable {
    GACHAPON.get_or_init(|| {
        GachaponTable::load(Some(&pack_path()))
            .or_else(|_| GachaponTable::load(None))
            .expect("committed Gachapon stand-ins must parse")
    })
}

/// Load the pack and describe what was found, for the startup log. A pack that
/// exists but cannot be parsed is an error.
pub fn load() -> Result<String, String> {
    let path = pack_path();
    let text = ServerText::load(Some(&path)).map_err(|e| e.to_string())?;
    let rewards = RewardTable::load(Some(&path)).map_err(|e| e.to_string())?;
    let shops = ShopTable::load(Some(&path)).map_err(|e| e.to_string())?;
    let gachapon = GachaponTable::load(Some(&path)).map_err(|e| e.to_string())?;
    let mut summary = format!(
        "{}: {} text keys from the pack, {} placeholders; drops for {} mobs from the pack; {} shops; Gachapon prizes from {}",
        path.display(),
        text.from_pack(),
        text.placeholders(),
        rewards.from_pack(),
        shops.len(),
        if gachapon.from_pack() { "the pack" } else { "the stand-ins" }
    );
    if !text.unknown_in_pack().is_empty() {
        summary.push_str(&format!("; unused pack keys: {:?}", text.unknown_in_pack()));
    }
    let _ = TEXT.set(text);
    let _ = REWARDS.set(rewards);
    let _ = SHOPS.set(shops);
    let _ = GACHAPON.set(gachapon);
    Ok(summary)
}
