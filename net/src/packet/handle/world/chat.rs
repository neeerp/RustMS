use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct AllChatHandler;

impl AllChatHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for AllChatHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        let _op = reader.read_short()?;

        let msg = reader.read_str_with_length()?;
        println!("Somebody said: {}", msg);

        let _hidden = reader.read_byte()?;

        // TODO: Broadcast to map players
        Ok(HandlerResult::empty())
    }
}
