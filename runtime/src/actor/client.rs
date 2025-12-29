use crate::error::RuntimeError;
use crate::handler::{HandlerAction, HandlerResult};
use crate::io::{PacketReader, PacketWriter};
use crate::message::{ClientEvent, ClientId, ServerMessage};
use net::packet::build;
use packet::Packet;
use rand::{thread_rng, Rng};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Actor handling a single world server client connection.
pub struct ClientActor {
    client_id: ClientId,
    reader: PacketReader,
    writer: PacketWriter,
    /// Channel to send events to world server
    world_tx: mpsc::Sender<ClientEvent>,
    /// Channel to receive messages from world server
    server_rx: mpsc::Receiver<ServerMessage>,
    /// Our sender (given to world server on connect)
    server_tx: mpsc::Sender<ServerMessage>,
}

impl ClientActor {
    /// Create a new client actor from an accepted TCP connection.
    pub async fn new(
        stream: TcpStream,
        world_tx: mpsc::Sender<ClientEvent>,
    ) -> Result<Self, RuntimeError> {
        // Generate encryption IVs (scope rng to drop before await)
        let (recv_iv, send_iv) = {
            let mut recv_iv = vec![0u8; 4];
            let mut send_iv = vec![0u8; 4];
            let mut rng = thread_rng();
            rng.fill(&mut recv_iv[..]);
            rng.fill(&mut send_iv[..]);
            (recv_iv, send_iv)
        };

        // Split stream for independent read/write
        let (read_half, write_half) = stream.into_split();

        let reader = PacketReader::new(read_half, &recv_iv);
        let mut writer = PacketWriter::new(write_half, &send_iv);

        // Send handshake
        let handshake = build::build_handshake_packet(&recv_iv, &send_iv)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        writer.send_handshake(&handshake.bytes).await?;

        // Create channel for receiving server messages
        let (server_tx, server_rx) = mpsc::channel(32);

        Ok(Self {
            client_id: 0, // Set after login
            reader,
            writer,
            world_tx,
            server_rx,
            server_tx,
        })
    }

    /// Run the client actor event loop.
    pub async fn run(mut self) {
        info!("ClientActor started");

        loop {
            tokio::select! {
                // Read packets from TCP
                result = self.reader.read_packet() => {
                    match result {
                        Ok(packet) => {
                            if let Err(e) = self.handle_packet(packet).await {
                                match e {
                                    RuntimeError::ClientDisconnected => {
                                        info!(self.client_id, "Client disconnected gracefully");
                                        break;
                                    }
                                    other => {
                                        error!(self.client_id, error = %other, "Error handling packet");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(RuntimeError::ClientDisconnected) => {
                            info!(self.client_id, "Client disconnected");
                            break;
                        }
                        Err(e) => {
                            error!(self.client_id, error = %e, "Error reading packet");
                            break;
                        }
                    }
                }

                // Receive messages from world server
                Some(msg) = self.server_rx.recv() => {
                    if let Err(e) = self.handle_server_message(msg).await {
                        error!(self.client_id, error = %e, "Error handling server message");
                        break;
                    }
                }
            }
        }

        // Cleanup
        self.cleanup().await;
    }

    async fn handle_packet(&mut self, packet: Packet) -> Result<(), RuntimeError> {
        let opcode = packet.opcode();

        // TODO: Phase 5 will implement actual handler migration
        // For now, just log the packet and return empty result
        info!(opcode, "Received packet (handler not yet migrated)");

        // Process handler actions (empty for now)
        let result = HandlerResult::empty();
        self.process_actions(result).await
    }

    async fn process_actions(&mut self, result: HandlerResult) -> Result<(), RuntimeError> {
        for action in result.actions {
            match action {
                HandlerAction::Reply(mut packet) => {
                    self.writer.send_packet(&mut packet).await?;
                }
                HandlerAction::Broadcast { scope, packet } => {
                    let event = ClientEvent::Broadcast {
                        from: self.client_id,
                        scope,
                        packet,
                    };
                    self.world_tx
                        .send(event)
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                }
                HandlerAction::Disconnect => {
                    return Err(RuntimeError::ClientDisconnected);
                }
            }
        }
        Ok(())
    }

    async fn handle_server_message(&mut self, msg: ServerMessage) -> Result<(), RuntimeError> {
        match msg {
            ServerMessage::SendPacket(mut packet) => {
                self.writer.send_packet(&mut packet).await?;
            }
            ServerMessage::Kick(reason) => {
                warn!(self.client_id, reason, "Client kicked");
                return Err(RuntimeError::ClientDisconnected);
            }
            ServerMessage::Shutdown => {
                info!(self.client_id, "Server shutting down");
                return Err(RuntimeError::ClientDisconnected);
            }
        }
        Ok(())
    }

    /// Set the client ID (character ID) after login.
    pub fn set_client_id(&mut self, id: ClientId) {
        self.client_id = id;
    }

    /// Register with world server after character is loaded.
    pub async fn register_with_world(
        &mut self,
        character_id: i32,
        map_id: i32,
    ) -> Result<(), RuntimeError> {
        self.client_id = character_id;

        let event = ClientEvent::Connected {
            client_id: character_id,
            sender: self.server_tx.clone(),
            map_id,
        };

        self.world_tx
            .send(event)
            .await
            .map_err(|_| RuntimeError::ChannelSend)?;

        Ok(())
    }

    async fn cleanup(&mut self) {
        // Notify world server of disconnect
        if self.client_id != 0 {
            let _ = self
                .world_tx
                .send(ClientEvent::Disconnected {
                    client_id: self.client_id,
                })
                .await;
        }

        // Session cleanup will be handled by the handler layer in Phase 5
        info!(self.client_id, "ClientActor cleanup complete");
    }
}
