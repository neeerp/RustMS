use super::op::RecvOpcode;
use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient, listener::ServerType,
};
use packet::Packet;

mod login;
mod world;

pub trait PacketHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        let op = packet.opcode();
        println!("Opcode: {}", op);
        println!("Received packet: {}", to_hex_string(&packet.bytes));
        Err(NetworkError::UnsupportedOpcodeError(op))
    }
}

pub struct DefaultHandler {}

impl PacketHandler for DefaultHandler {}

/// A default handler that echoes the packet in the console and throws an error.
impl DefaultHandler {
    pub fn new() -> Self {
        DefaultHandler {}
    }
}

// TODO: A lot of the login related handlers are very similar and can maybe be
// consolidated, making a check on the opcode to decide how to proceed... These
// handlers include the LoginCredentials handler, AcceptTOS handler,
// SetGenderHandler
//

pub fn get_handler(op: i16, server_type: &ServerType) -> Box<dyn PacketHandler> {
    match server_type {
        ServerType::Login => get_login_handler(op),
        ServerType::World => get_world_handler(op),
    }
}

/// Get the packet handler corresponding to the given opcode.
fn get_login_handler(op: i16) -> Box<dyn PacketHandler> {
    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::LoginCredentials) => Box::new(login::LoginCredentialsHandler::new()),
        Some(RecvOpcode::GuestLogin) => Box::new(login::GuestLoginHandler::new()),

        Some(RecvOpcode::ServerListReRequest) => Box::new(login::WorldListHandler::new()),
        Some(RecvOpcode::CharListRequest) => Box::new(login::CharListHandler::new()),
        Some(RecvOpcode::ServerStatusRequest) => Box::new(login::ServerStatusHandler::new()),

        Some(RecvOpcode::AcceptTOS) => Box::new(login::AcceptTOSHandler::new()),
        Some(RecvOpcode::SetGender) => Box::new(login::SetGenderHandler::new()),

        // TODO: HeavenClient doesn't seem to support PINs...
        Some(RecvOpcode::AfterLogin) => Box::new(DefaultHandler::new()),
        Some(RecvOpcode::RegisterPin) => Box::new(DefaultHandler::new()),

        Some(RecvOpcode::ServerListRequest) => Box::new(login::WorldListHandler::new()),

        Some(RecvOpcode::ViewAllChar) => Box::new(DefaultHandler::new()),
        Some(RecvOpcode::PickAllChar) => Box::new(DefaultHandler::new()),
        Some(RecvOpcode::CharSelect) => Box::new(login::CharacterSelectHandler::new()),

        Some(RecvOpcode::CheckCharName) => Box::new(login::CheckCharNameHandler::new()),
        Some(RecvOpcode::CreateChar) => Box::new(login::CreateCharacterHandler::new()),

        Some(RecvOpcode::DeleteChar) => Box::new(login::DeleteCharHandler::new()),

        Some(RecvOpcode::RegisterPic) => Box::new(DefaultHandler::new()),
        Some(RecvOpcode::CharSelectWithPic) => Box::new(DefaultHandler::new()),
        Some(RecvOpcode::ViewAllPicRegister) => Box::new(DefaultHandler::new()),
        Some(RecvOpcode::ViewAllWithPic) => Box::new(DefaultHandler::new()),

        Some(RecvOpcode::LoginStarted) => Box::new(login::LoginStartHandler::new()),

        None | Some(_) => Box::new(DefaultHandler::new()),
    }
}

fn get_world_handler(op: i16) -> Box<dyn PacketHandler> {
    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::PlayerMove) => Box::new(world::PlayerMoveHandler::new()),
        Some(RecvOpcode::PlayerLoggedIn) => Box::new(world::PlayerLoggedInHandler::new()),
        Some(RecvOpcode::PlayerMapTransfer) => Box::new(world::PlayerMapTransferHandler::new()),
        Some(RecvOpcode::PartySearch) => Box::new(world::PartySearchHandler::new()),
        Some(RecvOpcode::ChangeKeybinds) => Box::new(world::ChangeKeybindsHandler::new()),
        Some(RecvOpcode::AllChat) => Box::new(world::AllChatHandler::new()),

        None | Some(_) => Box::new(DefaultHandler::new()),
    }
}
