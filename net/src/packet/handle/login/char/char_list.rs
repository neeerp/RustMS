use crate::packet::build::login::char;
use crate::{error::NetworkError, io::client::MapleClient, packet::handle::PacketHandler};
use db::character;
use packet::io::read::PktRead;
use packet::Packet;
use std::io::BufReader;

pub struct CharListHandler {}

impl CharListHandler {
    pub fn new() -> Self {
        CharListHandler {}
    }
}

impl PacketHandler for CharListHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let _world = reader.read_byte()?;
        let _channel = reader.read_byte()? + 1;

        let user = client.get_account();
        let id = match user {
            Some(user) => user.id,
            None => return Err(NetworkError::PacketHandlerError("User not logged in.")),
        };

        let chars = character::get_characters_by_accountid(id)?;
        client.send(&mut char::build_char_list(chars)?)
    }
}
