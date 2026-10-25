use crate::gm_commands::{tier, ADMIN, GM, HELPER};
use crate::handler::{
    BroadcastScope, Discipline, FieldRequest, HandlerResult, WhisperDelivery, WorldAction,
    WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::messaging::{build_broadcast, build_slide, Broadcast};
use crate::packet::build::world::{skill, stat};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::{debug, info};

/// `/ban NAME`: block the account for good.
const BAN: u8 = 0x03;
/// `/block NAME DAYS REASON`: block it for some days, with the reason and
/// a note from the client's dialog (report b175 s4).
const BLOCK: u8 = 0x04;
/// The longest `/block` the server keeps; the client shows any end three or
/// more years away as for good.
pub const BLOCK_DAYS: u32 = 36_500;
/// `/h`: hide (1) or show (0) the GM.
const HIDE: u8 = 0x10;
/// `/u` (0) lists the map's users; the client also sends 12 on every map
/// change.
const USERS: u8 = 0x11;
/// `/w NAME TEXT`: a warning, shown to the player in a dialog.
const WARN: u8 = 0x1E;
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
            BAN if gated(ctx, ADMIN) => {
                let target = reader.read_str_with_length()?;
                block(ctx, target, 0, None, String::new())
            }
            BLOCK if gated(ctx, ADMIN) => {
                let target = reader.read_str_with_length()?;
                let reason = reader.read_byte()?;
                let days = reader.read_int()?;
                let note = reader.read_str_with_length()?;
                // v83 sends no 0 days but lets a negative number through,
                // and only the reasons Etc/BlockReason.img has.
                match (u32::try_from(days), reason) {
                    (Ok(days @ 1..), 1..=13) => {
                        block(ctx, target, reason, Some(days.min(BLOCK_DAYS)), note)
                    }
                    _ => Ok(HandlerResult::empty()),
                }
            }
            WARN if gated(ctx, HELPER) => {
                let name = reader.read_str_with_length()?;
                let text = reader.read_str_with_length()?;
                warn(ctx, name, &text)
            }
            _ => {
                debug!(mode, "Admin command not supported");
                Ok(HandlerResult::empty())
            }
        }
    }
}

/// Whether the GM may use a command of `needs` tier; the commands that
/// reach others are off with `RUSTMS_GM_COMMANDS=0`, as the chat commands
/// are.
fn gated(ctx: &WorldContext, needs: i16) -> bool {
    crate::policy::gm_commands_enabled() && tier(ctx.player.gm_level) >= needs
}

/// `/ban` and `/block`; the GM hears ADMIN_RESULT 4 (SP 0xa2, blocked) or
/// 6 with 1 (SP 0xa8, no such character).
fn block(
    ctx: &WorldContext,
    target: String,
    reason: u8,
    days: Option<u32>,
    note: String,
) -> WorldResult {
    info!(
        target: "gm_audit",
        character = %ctx.player.stats.name,
        gm_level = ctx.player.gm_level,
        command = if days.is_some() { "/block" } else { "/ban" },
        target,
        reason,
        days,
        note,
    );
    Ok(
        HandlerResult::empty().with(WorldAction::Discipline(Discipline::Block {
            target,
            reason,
            days,
            note,
            slash: true,
        })),
    )
}

/// `/w`: `text` in a dialog for the player named `name`, wherever they
/// are; the GM hears whether it got there (SP 0xbb5 or 0xbb4).
fn warn(ctx: &WorldContext, name: String, text: &str) -> WorldResult {
    info!(
        target: "gm_audit",
        character = %ctx.player.stats.name,
        gm_level = ctx.player.gm_level,
        command = "/w",
        target = name,
        text,
    );
    if name.is_empty() {
        return Ok(HandlerResult::reply(skill::build_admin_result(WARN, 0)?));
    }
    Ok(
        HandlerResult::empty().with(WorldAction::Whisper(WhisperDelivery {
            target_name: name,
            recipient_packet: build_broadcast(Broadcast::Popup, text)?,
            sender_success_packet: Some(skill::build_admin_result(WARN, 1)?),
            sender_failure_packet: Some(skill::build_admin_result(WARN, 0)?),
        })),
    )
}

/// ADMIN_LOG 0x81: the line of every GM slash command, which the client
/// sends after the command's own packet (0x52e28f). Logged for the audit;
/// what the command did is logged where it is done.
pub struct AdminLogHandler;

impl WorldHandler for AdminLogHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if ctx.player.gm_level <= 0 {
            return Ok(HandlerResult::empty());
        }
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let line = reader.read_str_with_length()?;
        info!(
            target: "gm_audit",
            character_id = ctx.player.stats.id,
            character = %ctx.player.stats.name,
            gm_level = ctx.player.gm_level,
            line,
            "Slash command"
        );
        Ok(HandlerResult::empty())
    }
}

/// ADMIN_BROADCAST 0x76: `/notice*`, `/alert*` and `/slide*` (report
/// b175 s1): the scope (0 the world, 1 the channel, 2 the map), the kind (0
/// a notice in chat, 1 a dialog, 4 the scrolling notice) and the text. The
/// `x` slides send no text, which takes the scrolling notice down.
pub struct AdminBroadcastHandler;

impl WorldHandler for AdminBroadcastHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if !gated(ctx, GM) {
            return Ok(HandlerResult::empty());
        }
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let scope = reader.read_byte()?;
        let kind = reader.read_byte()?;
        let text = reader.read_str_with_length()?;
        let packet = match kind {
            0 if !text.is_empty() => build_broadcast(Broadcast::Notice, &text)?,
            1 if !text.is_empty() => build_broadcast(Broadcast::Popup, &text)?,
            4 => build_slide(Some(text.as_str()).filter(|text| !text.is_empty()))?,
            _ => return Ok(HandlerResult::empty()),
        };
        info!(
            target: "gm_audit",
            character = %ctx.player.stats.name,
            gm_level = ctx.player.gm_level,
            scope,
            kind,
            text,
            "Broadcast"
        );
        let action = |scope| WorldAction::Broadcast {
            scope,
            packet: packet.clone(),
        };
        Ok(match scope {
            0 => HandlerResult::empty().with(action(BroadcastScope::World)),
            1 => HandlerResult::empty().with(action(BroadcastScope::Channel(ctx.channel_id))),
            2 => HandlerResult::reply(packet.clone())
                .with(WorldAction::Field(FieldRequest::Broadcast(packet))),
            _ => HandlerResult::empty(),
        })
    }
}

/// GM_SLIDE 0x86 (`/slideax`, report b175 s2): one byte, 0, whose meaning
/// no server of the time gives; taken and dropped.
pub struct GmSlideHandler;

impl WorldHandler for GmSlideHandler {
    fn handle(&self, _: &mut Packet, _: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty())
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
    fn a_warning_goes_to_the_player_named_in_a_dialog() {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = 1;
        let mut bytes = vec![WARN];
        bytes.extend([4, 0]);
        bytes.extend(b"Bob1");
        bytes.extend([5, 0]);
        bytes.extend(b"Stop!");
        let result = handle(&mut player, &bytes);
        let [WorldAction::Whisper(delivery)] = &result.actions[..] else {
            panic!("{:?}", result.actions);
        };
        assert_eq!(delivery.target_name, "Bob1");
        assert_eq!(
            delivery.recipient_packet.bytes,
            build_broadcast(Broadcast::Popup, "Stop!").unwrap().bytes
        );
        let answer = |delivered: &Option<Packet>| delivered.as_ref().unwrap().bytes[2..].to_vec();
        assert_eq!(answer(&delivery.sender_success_packet), [WARN, 1]);
        assert_eq!(answer(&delivery.sender_failure_packet), [WARN, 0]);
        // No name: v83's "Please enter the user's name" (SP 0xbb4).
        let result = handle(&mut player, &[WARN, 0, 0, 2, 0, b'h', b'i']);
        assert_eq!(result.actions.len(), 1);

        player.gm_level = 0;
        assert!(handle(&mut player, &bytes).actions.is_empty());
    }

    fn broadcast(
        player: &mut game::Player,
        scope: u8,
        kind: u8,
        text: &str,
    ) -> HandlerResult<WorldAction> {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::AdminBroadcast as i16)
            .unwrap();
        packet.write_byte(scope).unwrap();
        packet.write_byte(kind).unwrap();
        packet.write_str_with_length(text).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        AdminBroadcastHandler.handle(&mut packet, &mut ctx).unwrap()
    }

    #[test]
    fn broadcasts_reach_their_scope() {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = 3;
        let notice = build_broadcast(Broadcast::Notice, "hi").unwrap();
        let result = broadcast(&mut player, 0, 0, "hi");
        let [WorldAction::Broadcast {
            scope: BroadcastScope::World,
            packet,
        }] = &result.actions[..]
        else {
            panic!("{:?}", result.actions);
        };
        assert_eq!(packet.bytes, notice.bytes);
        let result = broadcast(&mut player, 1, 1, "hi");
        let [WorldAction::Broadcast {
            scope: BroadcastScope::Channel(2),
            packet,
        }] = &result.actions[..]
        else {
            panic!("{:?}", result.actions);
        };
        assert_eq!(
            packet.bytes,
            build_broadcast(Broadcast::Popup, "hi").unwrap().bytes
        );
        // The map: the GM, and the others through the field.
        let result = broadcast(&mut player, 2, 4, "");
        let slide_off = build_slide(None).unwrap();
        let [WorldAction::Reply(own), WorldAction::Field(FieldRequest::Broadcast(theirs))] =
            &result.actions[..]
        else {
            panic!("{:?}", result.actions);
        };
        assert_eq!(
            (&own.bytes, &theirs.bytes),
            (&slide_off.bytes, &slide_off.bytes)
        );
        // An empty notice, and anyone below GM, send nothing.
        assert!(broadcast(&mut player, 0, 0, "").actions.is_empty());
        player.gm_level = 1;
        assert!(broadcast(&mut player, 0, 0, "hi").actions.is_empty());
    }

    #[test]
    fn ban_and_block_need_an_admin() {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = 3;
        let block_for = |reason: u8, days: i32| {
            let mut bytes = vec![BLOCK, 3, 0];
            bytes.extend(b"Bob");
            bytes.push(reason);
            bytes.extend(days.to_le_bytes());
            bytes.extend([2, 0]);
            bytes.extend(b"ad");
            bytes
        };
        let block = |days| block_for(4, days);
        let result = handle(&mut player, &block(7));
        assert!(matches!(
            &result.actions[..],
            [WorldAction::Discipline(Discipline::Block { target, reason: 4, days: Some(7), note, slash: true })]
                if target == "Bob" && note == "ad"
        ));
        assert!(handle(&mut player, &block(-1)).actions.is_empty());
        assert!(handle(&mut player, &block_for(0, 7)).actions.is_empty());
        assert!(handle(&mut player, &block_for(14, 7)).actions.is_empty());
        let result = handle(&mut player, &[BAN, 3, 0, b'B', b'o', b'b']);
        assert!(matches!(
            &result.actions[..],
            [WorldAction::Discipline(Discipline::Block {
                reason: 0,
                days: None,
                ..
            })]
        ));

        player.gm_level = 2;
        assert!(handle(&mut player, &block(7)).actions.is_empty());
        assert!(handle(&mut player, &[BAN, 3, 0, b'B', b'o', b'b'])
            .actions
            .is_empty());
    }

    #[test]
    fn the_map_change_notice_does_nothing() {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = 1;
        assert!(handle(&mut player, &[USERS, 12]).actions.is_empty());
        assert!(handle(&mut player, &[ARTIFACT_RANKING]).actions.is_empty());
    }
}
