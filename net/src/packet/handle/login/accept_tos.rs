use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use packet::Packet;

pub struct AcceptTOSHandler {}

impl AcceptTOSHandler {
    pub fn new() -> Self {
        AcceptTOSHandler {}
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

impl PacketHandler for AcceptTOSHandler {
    fn handle(&self, _packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        // println!("TOS Accepted, Accepting Login.");
        // self.accept_logon(client)
        Ok(())
    }
}
