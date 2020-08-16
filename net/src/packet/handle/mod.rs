use super::op::RecvOpcode;
use crate::{error::NetworkError, io::client::MapleClient};
use packet::Packet;

mod default;
mod login;

pub trait PacketHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError>;
}

// TODO: Most of the handlers are hardcoded given that we don't have anything
// backing our data currently... They also have some missing cases.
//
// TODO: A lot of the login related handlers are very similar and can maybe be
// consolidated, making a check on the opcode to decide how to proceed... These
// handlers include the LoginCredentials handler, AcceptTOS handler,
// SetGenderHandler

/// Get the packet handler corresponding to the given opcode.
pub fn get_handler(op: i16) -> Box<dyn PacketHandler> {
    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::LoginCredentials) => Box::new(login::LoginCredentialsHandler::new()),
        Some(RecvOpcode::GuestLogin) => Box::new(login::GuestLoginHandler::new()),
        Some(RecvOpcode::ServerListReRequest) => Box::new(login::WorldListHandler::new()),
        Some(RecvOpcode::CharListRequest) => Box::new(login::CharListHandler::new()),
        Some(RecvOpcode::ServerStatusRequest) => Box::new(login::ServerStatusHandler::new()),

        // TODO
        Some(RecvOpcode::AcceptTOS) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::SetGender) => Box::new(default::DefaultHandler::new()),

        Some(RecvOpcode::AfterLogin) => Box::new(login::AfterLoginHandler::new()),
        Some(RecvOpcode::RegisterPin) => Box::new(login::RegisterPinHandler::new()),
        Some(RecvOpcode::ServerListRequest) => Box::new(login::WorldListHandler::new()),

        // TODO
        Some(RecvOpcode::ViewAllChar) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::PickAllChar) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::CharSelect) => Box::new(default::DefaultHandler::new()),

        Some(RecvOpcode::CheckCharName) => Box::new(login::CheckCharNameHandler::new()),

        // TODO
        Some(RecvOpcode::CreateChar) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::DeleteChar) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::RegisterPic) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::CharSelectWithPic) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::ViewAllPicRegister) => Box::new(default::DefaultHandler::new()),
        Some(RecvOpcode::ViewAllWithPic) => Box::new(default::DefaultHandler::new()),

        Some(RecvOpcode::LoginStarted) => Box::new(login::LoginStartHandler::new()),

        None | Some(_) => Box::new(default::DefaultHandler::new()),
    }
}
