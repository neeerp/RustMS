use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::rps::RpsRequest;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `RPS_ACTION` (report b038): the Rock, Paper, Scissors window's buttons by
/// mode, a hand (mode 1) with its byte. The runtime answers.
pub struct RpsActionHandler;

impl WorldHandler for RpsActionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0 => RpsRequest::Start,
            1 => RpsRequest::Choose(reader.read_byte()?),
            2 => RpsRequest::TimeUp,
            3 => RpsRequest::Continue,
            4 => RpsRequest::Exit,
            5 => RpsRequest::Retry,
            other => {
                debug!(mode = other, "Ignoring RPS request");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Rps(request)))
    }
}
