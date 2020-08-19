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
        reader.read_short().unwrap();

        let _world = reader.read_byte().unwrap();
        let _channel = reader.read_byte().unwrap() + 1;

        let user = client.user.take();
        let id = match user {
            Some(user) => {
                let id = user.id;
                client.user = Some(user);
                id
            }
            None => return Err(NetworkError::PacketHandlerError("User not logged in.")),
        };

        match character::get_characters_by_accountid(id) {
            Some(chars) => match client.send(&mut char::build_char_list(chars)) {
                Ok(()) => Ok(()),
                Err(e) => Err(NetworkError::CouldNotSend(e)),
            },
            None => {
                return Err(NetworkError::PacketHandlerError(
                    "Could not query for characters",
                ))
            }
        }
    }
}
