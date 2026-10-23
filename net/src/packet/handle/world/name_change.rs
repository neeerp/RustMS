//! Name Change (5400000, report c060). In the Cash Shop, the buy button
//! sends op 0x10 with the character id and birthday code; the rules and the
//! name dialog follow only for code 0 of CS 0x149. The dialog's Check sends
//! op 0x15, the login server's name check, and OK sends CASHSHOP_OPERATION
//! 0x2E (in `cash_shop`). Double-clicking the receipt in the bag cancels
//! the request, after two confirmations.

use crate::cash_shop::{self, CashShopRequest, NAME_CHANGE_LEVEL};
use crate::error::NetworkError;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::names;
use crate::packet::build::world::cash_shop::{build_cash_shop_failure, CashShopFailure};
use crate::packet::build::world::name_change::{
    build_name_change_rules, build_new_name_check, NameChangeRules,
};
use crate::packet::build::world::stat::build_stat_update;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// 0x10: may the character ask for a new name?
pub struct NameChangeRulesHandler;

impl WorldHandler for NameChangeRulesHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let character_id = reader.read_int()?;
        let birthday = reader.read_int()?;
        if !cash_shop::birthday_code(birthday) {
            return Ok(HandlerResult::reply(build_cash_shop_failure(
                CashShopFailure::BirthdayCode,
            )?));
        }
        if character_id != ctx.player.stats.id || ctx.player.stats.level < NAME_CHANGE_LEVEL {
            return Ok(HandlerResult::reply(build_name_change_rules(
                NameChangeRules::Refused,
            )?));
        }
        Ok(HandlerResult::empty().with(WorldAction::CashShop(CashShopRequest::NameChangeRules)))
    }
}

/// 0x15 in the world: the Name Change dialog's Check.
pub struct CheckNewNameHandler;

impl WorldHandler for CheckNewNameHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        if !allowed(&name, ctx)? {
            return Ok(HandlerResult::reply(build_new_name_check(&name, false)?));
        }
        Ok(HandlerResult::empty().with(WorldAction::CashShop(CashShopRequest::CheckNewName(name))))
    }
}

/// Whether the character may take `name` by the client's rules; whether
/// someone has it is the runtime's to check.
pub(super) fn allowed(name: &str, ctx: &WorldContext) -> Result<bool, NetworkError> {
    let forbidden = names::forbidden_names().map_err(NetworkError::PacketHandlerError)?;
    Ok(names::allowed(name, ctx.player.gm_level > 0, forbidden)
        && !name.eq_ignore_ascii_case(&ctx.player.stats.name))
}

/// USE_CASH_ITEM for the receipt in Cash `slot`: a byte, 1 for a name
/// change (2 cancels a Character Transfer), then the tick.
pub(super) fn cancel_name_change(reader: &mut Cursor<&Vec<u8>>, slot: i16) -> WorldResult {
    if reader.read_byte()? != 1 {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    }
    Ok(HandlerResult::empty().with(WorldAction::CancelNameChange { slot }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn rules(character_id: i32, birthday: i32, level: u8) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::NameChangeRules as i16)
            .unwrap();
        packet.write_int(character_id).unwrap();
        packet.write_int(birthday).unwrap();
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = level;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        NameChangeRulesHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn the_rules_need_the_birthday_the_character_and_level_ten() {
        let id = crate::codec::stats::sample_player().stats.id;
        assert!(matches!(
            rules(id, 19900101, 10).as_slice(),
            [WorldAction::CashShop(CashShopRequest::NameChangeRules)]
        ));
        let reply = |actions: Vec<WorldAction>| match actions.as_slice() {
            [WorldAction::Reply(packet)] => packet.bytes.clone(),
            other => panic!("expected a reply: {other:?}"),
        };
        assert_eq!(reply(rules(id, 19901301, 10))[2..], [0x5C, 0xC4]);
        assert_eq!(reply(rules(id, 19900101, 9))[..2], [0x49, 0x01]);
        assert_eq!(reply(rules(id + 1, 19900101, 10))[6], 4);
    }

    #[test]
    fn only_a_name_change_cancel_goes_to_the_runtime() {
        let bytes = vec![1, 0, 0, 0, 0];
        let actions = cancel_name_change(&mut Cursor::new(&bytes), 4)
            .unwrap()
            .actions;
        assert!(matches!(
            actions.as_slice(),
            [WorldAction::CancelNameChange { slot: 4 }]
        ));
        let bytes = vec![2, 0, 0, 0, 0];
        let actions = cancel_name_change(&mut Cursor::new(&bytes), 4)
            .unwrap()
            .actions;
        assert!(matches!(actions.as_slice(), [WorldAction::Reply(_)]));
    }
}
