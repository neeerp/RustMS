use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::{skill, stat};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `/h`: hide (1) or show (0) the GM.
const HIDE: u8 = 0x10;
/// `/u` (0) lists the map's users; the client also sends 12 on every map
/// change.
const USERS: u8 = 0x11;
/// The artifact ranking, which does nothing.
const ARTIFACT_RANKING: u8 = 0x24;

/// ADMIN_COMMAND 0x80: a GM's slash commands, the mode first. Ignored for
/// anyone who is not a GM, as Cosmic's `AdminCommandHandler` does.
pub struct AdminCommandHandler;

impl WorldHandler for AdminCommandHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if ctx.player.gm_level <= 0 {
            return Ok(HandlerResult::empty());
        }
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let mode = reader.read_byte()?;
        match mode {
            HIDE => set_hidden(ctx.player, reader.read_byte()? == 1),
            USERS | ARTIFACT_RANKING => Ok(HandlerResult::empty()),
            _ => {
                debug!(mode, "Admin command not supported");
                Ok(HandlerResult::empty())
            }
        }
    }
}

/// Hide the GM from those who are not GMs, or show them again; tells the
/// GM either way. Shared by the Hide skill, `/h` and `!hide`.
pub fn set_hidden(player: &mut game::Player, hidden: bool) -> WorldResult {
    let changed = player.hidden != hidden;
    player.hidden = hidden;
    let result = HandlerResult::reply(skill::build_hidden(hidden)?)
        .with_reply(stat::build_stat_update(true, &[])?);
    Ok(if changed {
        result.with(WorldAction::HiddenChanged)
    } else {
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn handle(player: &mut game::Player, bytes: &[u8]) -> HandlerResult<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::AdminCommand as i16).unwrap();
        packet.write_bytes(bytes).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        AdminCommandHandler.handle(&mut packet, &mut ctx).unwrap()
    }

    fn hid(result: &HandlerResult<WorldAction>) -> bool {
        result
            .actions
            .iter()
            .any(|action| matches!(action, WorldAction::HiddenChanged))
    }

    #[test]
    fn slash_h_hides_a_gm_and_shows_them_again() {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = 1;
        let result = handle(&mut player, &[HIDE, 1]);
        assert!(player.hidden && hid(&result));
        // Already hidden: the GM is told, nobody else.
        let result = handle(&mut player, &[HIDE, 1]);
        assert!(player.hidden && !hid(&result));
        assert!(!result.actions.is_empty());
        let result = handle(&mut player, &[HIDE, 0]);
        assert!(!player.hidden && hid(&result));
    }

    #[test]
    fn players_who_are_not_gms_get_nothing() {
        let mut player = crate::codec::stats::sample_player();
        let result = handle(&mut player, &[HIDE, 1]);
        assert!(!player.hidden && result.actions.is_empty());
    }

    #[test]
    fn the_map_change_notice_does_nothing() {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = 1;
        assert!(handle(&mut player, &[USERS, 12]).actions.is_empty());
        assert!(handle(&mut player, &[ARTIFACT_RANKING]).actions.is_empty());
    }
}
