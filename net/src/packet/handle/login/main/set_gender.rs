use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use account::Account;
use db::account;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct SetGenderHandler {}

impl SetGenderHandler {
    pub fn new() -> Self {
        SetGenderHandler {}
    }

    fn accept_logon(&self, client: &mut MapleClient, acc: Account) -> Result<(), NetworkError> {
        let mut packet = &mut build::login::status::build_successful_login_packet(&acc)?;
        client.user = Some(acc);

        client.send(&mut packet)
    }
}

impl PacketHandler for SetGenderHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let confirmed = reader.read_byte()?;
        let user = client.user.take();

        match (confirmed, user) {
            (0x01, Some(mut user)) => {
                let gender = reader.read_byte()?;
                user.gender = gender as i16;

                let user = account::update_account(&user)?;

                self.accept_logon(client, user)
            }
            _ => Err(NetworkError::PacketHandlerError(
                "Set Gender packet is invalid.",
            )),
        }
    }
}
