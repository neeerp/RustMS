use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::load::{self, asset_path};
use crate::packet::build::login;
use crate::packet::build::login::char::NameCheck;
use game_data::forbidden_names::ForbiddenNames;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::sync::OnceLock;

pub struct CheckCharNameHandler;

/// Whether the account may give a new character `name`.
pub(super) fn check_name(name: &str, account_id: i32) -> Result<NameCheck, NetworkError> {
    let gm = db::account::get_account_by_id(account_id)?.gm_level > 0;
    let forbidden = forbidden_names().map_err(NetworkError::PacketHandlerError)?;
    if !name_is_allowed(name, gm, forbidden) {
        return Ok(NameCheck::Refused);
    }
    match db::character::get_character_by_name(name) {
        Ok(_) => Ok(NameCheck::Taken),
        Err(db::Error::NotFound) => Ok(NameCheck::Available),
        Err(e) => Err(e.into()),
    }
}

/// The client's name rules: 4 to 12 ASCII letters or digits, fewer than
/// four `I`s and `l`s (they look alike), and, except for GMs, no forbidden
/// word.
fn name_is_allowed(name: &str, gm: bool, forbidden: &ForbiddenNames) -> bool {
    (4..=12).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric())
        && name.bytes().filter(|&b| b == b'I' || b == b'l').count() < 4
        && (gm || !forbidden.forbids(name))
}

fn forbidden_names() -> Result<&'static ForbiddenNames, &'static str> {
    static NAMES: OnceLock<Result<ForbiddenNames, String>> = OnceLock::new();
    load::once(&NAMES, || {
        ForbiddenNames::load(asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"))
    })
}

impl LoginHandler for CheckCharNameHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        let check = check_name(&name, ctx.session()?.account_id)?;
        Ok(HandlerResult::reply(login::char::build_char_name_response(
            &name, check,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_must_be_short_and_alphanumeric() {
        let none = ForbiddenNames::default();
        assert!(name_is_allowed("Maple123", false, &none));
        assert!(!name_is_allowed("abc", false, &none));
        assert!(!name_is_allowed("thirteenchars", false, &none));
        assert!(!name_is_allowed("bad name", false, &none));
        assert!(!name_is_allowed("ünï", false, &none));
    }

    #[test]
    fn names_may_not_have_four_letters_that_look_alike() {
        let none = ForbiddenNames::default();
        assert!(!name_is_allowed("IllI", true, &none), "GMs too");
        assert!(name_is_allowed("Ilio", false, &none));
        assert!(
            name_is_allowed("iiLLLi", false, &none),
            "only I and l count"
        );
    }

    #[test]
    fn only_gms_may_use_forbidden_words() {
        let Ok(forbidden) = forbidden_names() else {
            return;
        };
        assert!(!name_is_allowed("TheAdmin", false, forbidden));
        assert!(name_is_allowed("TheAdmin", true, forbidden));
    }
}
