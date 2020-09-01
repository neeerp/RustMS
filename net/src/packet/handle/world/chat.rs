use crate::{error::NetworkError, io::client::MapleClient, packet::handle::PacketHandler};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct AllChatHandler {}

impl AllChatHandler {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Serious implementation later...
impl PacketHandler for AllChatHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        let _op = reader.read_short()?;

        // TODO: We have yet to actually save the character with the session!
        let msg = reader.read_str_with_length()?;
        println!("Somebody said: {}", msg);

        let _hidden = reader.read_byte()?;

        // TODO: One of the first "Server actions" we can try to implement is
        // TODO: perhaps sending chat messages to every player on a given map?

        Ok(())
    }
}
