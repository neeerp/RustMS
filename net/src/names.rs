//! The words names may not contain, which the client checks too.

use crate::load::{self, asset_path};
use game_data::forbidden_names::ForbiddenNames;
use std::sync::OnceLock;

/// The forbidden-word list from `Etc.nx`, loaded on first use.
pub fn forbidden_names() -> Result<&'static ForbiddenNames, &'static str> {
    static NAMES: OnceLock<Result<ForbiddenNames, String>> = OnceLock::new();
    load::once(&NAMES, || {
        ForbiddenNames::load(asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"))
    })
}
