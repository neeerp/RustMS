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
}

impl PacketHandler for CheckCharNameHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let name = reader.read_str_with_length()?;
        client.send(&mut login::char::build_char_name_response(&name, true)?)
    }
}
