use crate::error::RuntimeError;
use crate::handler::{HandlerAction, HandlerContext, HandlerResult};
use crate::io::{PacketReader, PacketWriter};
use db::session::{NewSession, SessionWrapper};
use net::listener::ServerType;
use net::packet::build;
use net::get_async_handler;
use packet::Packet;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::{error, info, warn};

/// Actor handling a single login server client connection.
/// Simpler than ClientActor - no inter-client communication needed.
pub struct LoginClientActor {
    reader: PacketReader,
    writer: PacketWriter,
    session: SessionWrapper,
    peer_addr: SocketAddr,
}

impl LoginClientActor {
    /// Create a new login client actor from an accepted TCP connection.
    pub async fn new(stream: TcpStream, peer_addr: SocketAddr) -> Result<Self, RuntimeError> {
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

        Ok(Self {
            reader,
            writer,
            session: SessionWrapper::new_empty(),
            peer_addr,
        })
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

    async fn handle_packet(&mut self, mut packet: Packet) -> Result<(), RuntimeError> {
        let opcode = packet.opcode();

        // Get the async handler for this opcode
        let handler = get_async_handler(opcode, &ServerType::Login);

        // Execute handler in blocking context for DB calls
        // Move session out temporarily to satisfy borrow checker
        let mut session = std::mem::replace(&mut self.session, SessionWrapper::new_empty());

        let (result, returned_session) = tokio::task::spawn_blocking(move || {
            let mut ctx = HandlerContext {
                client_id: 0, // Login server doesn't use client_id
                session: &mut session,
            };
            let result = handler.handle(&mut packet, &mut ctx);
            (result, session)
        })
        .await
        .map_err(|e| RuntimeError::Handler(format!("Task join error: {}", e)))?;

        // Restore session
        self.session = returned_session;

        // Process handler result
        match result {
            Ok(result) => self.process_actions(result).await,
            Err(e) => {
                // Log handler error but don't disconnect for unsupported opcodes
                if matches!(e, net::error::NetworkError::UnsupportedOpcodeError(_)) {
                    info!(opcode, "Unsupported opcode");
                    Ok(())
                } else {
                    error!(opcode, error = %e, "Handler error");
                    Err(RuntimeError::Handler(e.to_string()))
                }
            }
        }
    }

    async fn process_actions(&mut self, result: HandlerResult) -> Result<(), RuntimeError> {
        for action in result.actions {
            match action {
                HandlerAction::Reply(mut packet) => {
                    self.writer.send_packet(&mut packet).await?;
                }
                HandlerAction::Broadcast { .. } => {
                    // Login server doesn't support broadcast
                    warn!("Broadcast action ignored in login server");
                }
                HandlerAction::Disconnect => {
                    return Err(RuntimeError::ClientDisconnected);
                }
                HandlerAction::CreateSession {
                    account_id,
                    hwid,
                    state,
                } => {
                    // Create a new session in the database
                    info!(account_id, "Creating session");
                    let new_session = NewSession {
                        account_id,
                        ip: self.peer_addr.ip().into(),
                        hwid: &hwid,
                        state,
                    };
                    match new_session.create() {
                        Ok(wrapper) => {
                            self.session = wrapper;
                            info!(account_id, "Session created successfully");
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to create session");
                        }
                    }
                }
                HandlerAction::AttachCharacter { character_id } => {
                    // Attach character to session for world server transition
                    info!(character_id, "Attaching character to session");
                    if let Some(ref mut session) = self.session.session {
                        session.character_id = Some(character_id);
                        // Update session in database
                        if let Err(e) = db::session::update_session_character(session.id, character_id) {
                            error!(error = %e, "Failed to update session with character");
                        }
                    }
                }
                HandlerAction::ReattachSession { .. } => {
                    // Login server doesn't reattach sessions
                    warn!("ReattachSession action ignored in login server");
                }
            }
        }
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
                        match LoginClientActor::new(stream, peer_addr).await {
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
