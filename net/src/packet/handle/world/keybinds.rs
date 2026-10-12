use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use game::keymap::KEY_COUNT;
use game::KeyBinding;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct ChangeKeybindsHandler;

impl WorldHandler for ChangeKeybindsHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;

        // Mode 0 changes key bindings; other modes (auto HP/MP pots) are not
        // supported yet.
        if reader.read_int()? != 0 {
            return Ok(HandlerResult::empty());
        }

        let count = reader.read_int()?;
        let keymap = ctx.player.keymap_mut();
        for _ in 0..count {
            let key = reader.read_int()?;
            let kind = reader.read_byte()?;
            let action = reader.read_int()?;
            if (0..i32::from(KEY_COUNT)).contains(&key) {
                keymap.bind(key as i16, KeyBinding { kind, action });
            }
        }
        Ok(HandlerResult::empty())
    }
}
