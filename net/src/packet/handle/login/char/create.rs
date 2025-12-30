use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use build::login;
use db::character::NewCharacter;
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

        let user = client.get_account();
        let accountid: i32;

        if let Some(acc) = user {
            accountid = acc.id;
        } else {
            return Err(NetworkError::NotLoggedIn);
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

        client.send(&mut login::char::build_char_packet(
            character.create()?.character,
        )?)
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncCreateCharacterHandler;

impl AsyncCreateCharacterHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncCreateCharacterHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        // Get account_id from session
        let accountid = ctx.session.session.as_ref()
            .map(|s| s.account_id)
            .ok_or(NetworkError::NotLoggedIn)?;

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

        let char_packet = login::char::build_char_packet(character.create()?.character)?;
        Ok(HandlerResult::reply(char_packet))
    }
}
