use crate::io::{accept, client::MapleClient};
use crate::{error::NetworkError, packet::build, packet::handle};

use packet::Packet;

use bufstream::BufStream;
use rand::{thread_rng, Rng};
use std::net::TcpStream;

pub struct ClientConnectionListener {
    pub client: MapleClient,
    server_type: ServerType,
}

impl ClientConnectionListener {
    /// Instantiate a new Login Server Connection Listener
    pub fn login_server(stream: TcpStream) -> Result<Self, NetworkError> {
        Self::new(stream, ServerType::Login)
    }

    /// Instantiate a new World server Connection Listener
    pub fn world_server(stream: TcpStream) -> Result<Self, NetworkError> {
        Self::new(stream, ServerType::World)
    }

    /// Instantiate a new maplestory client listener, generating encryption
    /// IVs in the process.
    fn new(
        stream: TcpStream,
        server_type: ServerType,
    ) -> Result<ClientConnectionListener, NetworkError> {
        let stream = BufStream::new(stream);

        let (recv_iv, send_iv) = ClientConnectionListener::generate_ivs();
        let mut client = MapleClient::new(stream, &recv_iv, &send_iv);

        let handshake_packet = build::build_handshake_packet(&recv_iv, &send_iv)?;

        match client.send_without_encryption(&handshake_packet) {
            Ok(_) => Ok(ClientConnectionListener {
                client,
                server_type,
            }),
            Err(NetworkError::IoError(e)) => Err(NetworkError::CouldNotEstablishConnection(e)),
            Err(e) => Err(e),
        }
    }

    /// Generate a pair of encryption IVs.
    fn generate_ivs() -> (Vec<u8>, Vec<u8>) {
        let mut recv_iv: Vec<u8> = vec![0u8; 4];
        let mut send_iv: Vec<u8> = vec![0u8; 4];

        let mut rng = thread_rng();
        rng.fill(&mut recv_iv[..]);
        rng.fill(&mut send_iv[..]);

        (recv_iv, send_iv)
    }

    /// Listen for packets being sent from the client via the session stream.
    pub fn listen(&mut self) -> Result<(), NetworkError> {
        loop {
            if let Err(e) = self.read_from_stream() {
                return Err(self.close_gracefully(e));
            }
        }
    }

    fn close_gracefully(&mut self, e: NetworkError) -> NetworkError {
        match self.client.logout() {
            Ok(_) => e,
            Err(logout_err) => NetworkError::LogoutError(Box::new(logout_err)), // TODO: Need error type that encapsualtes error
        }
    }

    /// Read packets from the session stream.
    fn read_from_stream(&mut self) -> Result<(), NetworkError> {
        accept::read_packet(&mut self.client).map(|packet| self.handle_packet(packet))?
    }

    /// Deal with the packet data by printing it out.
    fn handle_packet(&mut self, mut packet: Packet) -> Result<(), NetworkError> {
        let handler = handle::get_handler(packet.opcode(), &self.server_type);

        handler.handle(&mut packet, &mut self.client)
    }
}

pub enum ServerType {
    Login,
    World,
}
