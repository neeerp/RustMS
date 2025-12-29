use crate::error::RuntimeError;
use crate::io::{PacketReader, PacketWriter};
use net::packet::build;
use packet::Packet;
use rand::{thread_rng, Rng};
use tokio::net::TcpStream;
use tracing::{error, info};

/// Actor handling a single login server client connection.
/// Simpler than ClientActor - no inter-client communication needed.
pub struct LoginClientActor {
    reader: PacketReader,
    writer: PacketWriter,
}

impl LoginClientActor {
    /// Create a new login client actor from an accepted TCP connection.
    pub async fn new(stream: TcpStream) -> Result<Self, RuntimeError> {
        // Generate encryption IVs (scope rng to drop before await)
        let (recv_iv, send_iv) = {
            let mut recv_iv = vec![0u8; 4];
            let mut send_iv = vec![0u8; 4];
            let mut rng = thread_rng();
            rng.fill(&mut recv_iv[..]);
            rng.fill(&mut send_iv[..]);
            (recv_iv, send_iv)
        };

        // Split stream
        let (read_half, write_half) = stream.into_split();

        let reader = PacketReader::new(read_half, &recv_iv);
        let mut writer = PacketWriter::new(write_half, &send_iv);

        // Send handshake
        let handshake = build::build_handshake_packet(&recv_iv, &send_iv)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        writer.send_handshake(&handshake.bytes).await?;

        Ok(Self { reader, writer })
    }

    /// Run the login client actor event loop.
    pub async fn run(mut self) {
        info!("LoginClientActor started");

        loop {
            match self.reader.read_packet().await {
                Ok(packet) => {
                    if let Err(e) = self.handle_packet(packet).await {
                        match e {
                            RuntimeError::ClientDisconnected => {
                                info!("Login client disconnected gracefully");
                                break;
                            }
                            other => {
                                error!(error = %other, "Error handling login packet");
                                break;
                            }
                        }
                    }
                }
                Err(RuntimeError::ClientDisconnected) => {
                    info!("Login client disconnected");
                    break;
                }
                Err(e) => {
                    error!(error = %e, "Error reading login packet");
                    break;
                }
            }
        }

        info!("LoginClientActor finished");
    }

    async fn handle_packet(&mut self, packet: Packet) -> Result<(), RuntimeError> {
        let opcode = packet.opcode();

        // TODO: Phase 5 will implement proper handler migration
        // For now, just log the packet
        info!(opcode, "Received login packet (handler not yet migrated)");

        Ok(())
    }

    /// Send a packet to the client.
    #[allow(dead_code)]
    pub async fn send(&mut self, packet: &mut Packet) -> Result<(), RuntimeError> {
        self.writer.send_packet(packet).await
    }
}

/// Simple login server that just accepts connections.
/// No world server needed since login clients don't communicate with each other.
pub struct LoginServerActor;

impl LoginServerActor {
    pub async fn run(addr: &str) -> Result<(), RuntimeError> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!(addr, "LoginServerActor listening");

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!(%peer_addr, "Login connection accepted");

                    tokio::spawn(async move {
                        match LoginClientActor::new(stream).await {
                            Ok(actor) => actor.run().await,
                            Err(e) => error!(error = %e, "Failed to create LoginClientActor"),
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Error accepting login connection");
                }
            }
        }
    }
}
