use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use packet::Packet;

pub struct SetGenderHandler {}

impl SetGenderHandler {
    pub fn new() -> Self {
        SetGenderHandler {}
    }

    // fn accept_logon(&self, client: &mut MapleClient) -> Result<(), NetworkError> {
    //     println!("Logging in!");

    //     let mut return_packet = build::login::status::build_successful_login_packet();
    //     match client.send(&mut return_packet) {
    //         Ok(_) => {
    //             println!("Logon success packet sent.");
    //             Ok(())
    //         }
    //         Err(e) => Err(NetworkError::CouldNotSend(e)),
    //     }
    // }
}

impl PacketHandler for SetGenderHandler {
    fn handle(&self, _packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        // // TODO: Can implement this when we've got a DB...
        // println!("Ignoring Set Gender Packet and continuing logon.");
        // self.accept_logon(client)
        Ok(())
    }
}
