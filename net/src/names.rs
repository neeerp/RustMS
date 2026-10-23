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

/// The client's name rules: 4 to 12 ASCII letters or digits, fewer than
/// four `I`s and `l`s (they look alike), and, except for GMs, no forbidden
/// word.
pub fn allowed(name: &str, gm: bool, forbidden: &ForbiddenNames) -> bool {
    (4..=12).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric())
        && name.bytes().filter(|&b| b == b'I' || b == b'l').count() < 4
        && (gm || !forbidden.forbids(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_must_be_short_and_alphanumeric() {
        let none = ForbiddenNames::default();
        assert!(allowed("Maple123", false, &none));
        assert!(!allowed("abc", false, &none));
        assert!(!allowed("thirteenchars", false, &none));
        assert!(!allowed("bad name", false, &none));
        assert!(!allowed("ünï", false, &none));
    }

    #[test]
    fn names_may_not_have_four_letters_that_look_alike() {
        let none = ForbiddenNames::default();
        assert!(!allowed("IllI", true, &none), "GMs too");
        assert!(allowed("Ilio", false, &none));
        assert!(allowed("iiLLLi", false, &none), "only I and l count");
    }

    #[test]
    fn only_gms_may_use_forbidden_words() {
        let Ok(forbidden) = forbidden_names() else {
            return;
        };
        assert!(!allowed("TheAdmin", false, forbidden));
        assert!(allowed("TheAdmin", true, forbidden));
    }
}
