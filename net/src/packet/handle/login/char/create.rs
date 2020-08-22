use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use build::login;
use db::character::{self, NewCharacter};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct CreateCharacterHandler {}

impl CreateCharacterHandler {
    pub fn new() -> Self {
        CreateCharacterHandler {}
    }
}

impl PacketHandler for CreateCharacterHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let user = client.user.take();
        let accountid: i32;

        match user {
            Some(acc) => {
                accountid = acc.id;
                client.user = Some(acc);
            }
            _ => panic!("No account found!"),
        }

        let name = &reader.read_str_with_length()?;
        let job = reader.read_int()? as i16;
        let face = reader.read_int()?;
        let hair = reader.read_int()?;
        let hair_color = reader.read_int()?;
        let skin = reader.read_int()?;
        let _top = reader.read_int()?; // Slot 5
        let _bot = reader.read_int()?; // Slot 6
        let _shoes = reader.read_int()?; // Slot 7
        let _weapon = reader.read_int()?; // Special
        let gender = reader.read_byte()? as i16;

        let world = 0;

        let character = NewCharacter {
            accountid,
            world,
            name,
            job,
            face,
            hair,
            hair_color,
            skin,
            gender,
        };

        // TODO: Need to gracefully return after invalid character!
        let character = character::create_character(character)?;

        client.send(&mut login::char::build_char_packet(character)?)
    }
}
