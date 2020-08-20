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
        reader.read_short().unwrap();

        let user = client.user.take();
        let accountid: i32;

        match user {
            Some(acc) => {
                accountid = acc.id;
                client.user = Some(acc);
            }
            _ => panic!("No account found!"),
        }

        let name = &reader.read_str_with_length().unwrap();
        let job = reader.read_int().unwrap() as i16;
        let face = reader.read_int().unwrap();
        let hair = reader.read_int().unwrap();
        let hair_color = reader.read_int().unwrap();
        let skin = reader.read_int().unwrap();
        let _top = reader.read_int().unwrap(); // Slot 5
        let _bot = reader.read_int().unwrap(); // Slot 6
        let _shoes = reader.read_int().unwrap(); // Slot 7
        let _weapon = reader.read_int().unwrap(); // Special
        let gender = reader.read_byte().unwrap() as i16;

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

        let character = character::create_character(character).unwrap();

        match client.send(&mut login::char::build_char_packet(character)) {
            Ok(()) => Ok(()),
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }
}
