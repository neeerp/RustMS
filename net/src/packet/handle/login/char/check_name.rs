use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use build::login;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct CheckCharNameHandler {}

impl CheckCharNameHandler {
    pub fn new() -> Self {
        CheckCharNameHandler {}
    }

    fn check_name(&self, name: &str) -> Result<bool, NetworkError> {
        match db::character::get_character_by_name(&name) {
            Ok(_) => Ok(false),
            Err(db::Error::NotFound) => Ok(true),
            Err(e) => Err(NetworkError::DbError(e)),
        }
    }
}

impl PacketHandler for CheckCharNameHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let name = reader.read_str_with_length()?;

        client.send(&mut login::char::build_char_name_response(
            &name,
            self.check_name(&name)?,
        )?)
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncCheckCharNameHandler;

impl AsyncCheckCharNameHandler {
    pub fn new() -> Self {
        Self
    }

    fn check_name(name: &str) -> Result<bool, NetworkError> {
        match db::character::get_character_by_name(name) {
            Ok(_) => Ok(false),
            Err(db::Error::NotFound) => Ok(true),
            Err(e) => Err(NetworkError::DbError(e)),
        }
    }
}

impl AsyncPacketHandler for AsyncCheckCharNameHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let name = reader.read_str_with_length()?;
        let available = Self::check_name(&name)?;
        let response_packet = login::char::build_char_name_response(&name, available)?;
        Ok(HandlerResult::reply(response_packet))
    }
}
