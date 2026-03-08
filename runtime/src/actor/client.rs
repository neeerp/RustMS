use crate::error::RuntimeError;
use crate::handler::{ClientId, HandlerAction, HandlerContext, HandlerResult};
use crate::io::{PacketReader, PacketWriter};
use crate::message::{ClientEvent, FieldCharacter, RuntimeLocation, ServerMessage};
use db::session::{SessionState, SessionWrapper};
use net::get_handler;
use net::listener::ServerType;
use net::login_world::resolve_login_channel;
use net::packet::build;
use packet::Packet;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Actor handling a single world server client connection.
pub struct ClientActor {
    client_id: ClientId,
    reader: PacketReader,
    writer: PacketWriter,
    session: SessionWrapper,
    /// Channel to send events to world server
    world_tx: mpsc::Sender<ClientEvent>,
    /// Channel to receive messages from world server
    server_rx: mpsc::Receiver<ServerMessage>,
    /// Our sender (given to world server on connect)
    server_tx: mpsc::Sender<ServerMessage>,
    peer_addr: SocketAddr,
}

impl ClientActor {
    /// Create a new client actor from an accepted TCP connection.
    pub async fn new(
        stream: TcpStream,
        world_tx: mpsc::Sender<ClientEvent>,
        peer_addr: SocketAddr,
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
            session: SessionWrapper::new_empty(),
            world_tx,
            server_rx,
            server_tx,
            peer_addr,
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

    async fn handle_packet(&mut self, mut packet: Packet) -> Result<(), RuntimeError> {
        let opcode = packet.opcode();

        // Get the handler for this opcode
        let handler = get_handler(opcode, &ServerType::World);

        // Execute handler in blocking context for DB calls
        // Move session out temporarily to satisfy borrow checker
        let mut session = std::mem::replace(&mut self.session, SessionWrapper::new_empty());
        let client_id = self.client_id;

        let (result, returned_session) = tokio::task::spawn_blocking(move || {
            let mut ctx = HandlerContext {
                client_id,
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
                HandlerAction::Whisper {
                    target_name,
                    recipient_packet,
                    sender_success_packet,
                    sender_failure_packet,
                } => {
                    let event = ClientEvent::Whisper {
                        from: self.client_id,
                        target_name,
                        recipient_packet,
                        sender_success_packet,
                        sender_failure_packet,
                    };
                    self.world_tx
                        .send(event)
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                }
                HandlerAction::FieldChat { packet } => {
                    let event = ClientEvent::FieldChat {
                        from: self.client_id,
                        packet,
                    };
                    self.world_tx
                        .send(event)
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                }
                HandlerAction::FieldMove {
                    packet,
                    movement_bytes,
                } => {
                    let event = ClientEvent::FieldMove {
                        from: self.client_id,
                        packet,
                        movement_bytes,
                    };
                    self.world_tx
                        .send(event)
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                }
                HandlerAction::MapChanged {
                    old_map_id,
                    new_map_id,
                    spawn_portal_id,
                    spawn_x,
                    spawn_y,
                    spawn_stance,
                } => {
                    let channel_id = self
                        .session
                        .session
                        .as_ref()
                        .and_then(|session| session.selected_channel_id)
                        .unwrap_or(0) as u8;
                    let event = ClientEvent::LocationChanged {
                        client_id: self.client_id,
                        old: RuntimeLocation {
                            channel_id,
                            map_id: old_map_id,
                            instance_id: 0,
                        },
                        new: RuntimeLocation {
                            channel_id,
                            map_id: new_map_id,
                            instance_id: 0,
                        },
                        spawn_portal_id,
                        spawn_x,
                        spawn_y,
                        spawn_stance,
                    };
                    self.world_tx
                        .send(event)
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                }
                HandlerAction::Disconnect => {
                    return Err(RuntimeError::ClientDisconnected);
                }
                HandlerAction::CreateSession { .. } => {
                    // World server doesn't create sessions - login server does
                    warn!("CreateSession action ignored in world server");
                }
                HandlerAction::AttachCharacter { character_id } => {
                    // Load character into session
                    info!(character_id, "Attaching character to session");
                    // The character loading is done via session wrapper
                    if let Some(ref mut session) = self.session.session {
                        session.character_id = Some(character_id);
                    }
                    // Force reload character
                    let _ = self.session.get_character();
                }
                HandlerAction::UpdateSessionSelection { .. } => {
                    warn!("UpdateSessionSelection action ignored in world server");
                }
                HandlerAction::ReattachSession {
                    character_id,
                    channel_id,
                } => {
                    // Reattach session from login server
                    info!(character_id, "Reattaching session for character");
                    self.client_id = character_id;

                    // Load session from database by character_id
                    // Build packets synchronously, then release all locks before await
                    let reattach_result: Option<(FieldCharacter, RuntimeLocation, Packet, Packet)> = (|| {
                        let mut session = db::session::get_transition_session_by_character_channel_ip(
                            character_id,
                            i16::from(channel_id),
                            self.peer_addr.ip().into(),
                        )
                        .ok()?;
                        session.state = SessionState::InGame;
                        let session = db::session::update_session(&session).ok()?;
                        let wrapper = SessionWrapper::from(session).ok()?;
                        self.session = wrapper;
                        let chr_ref = self.session.get_character().ok()?;
                        let mut chr = chr_ref.lock().ok()?;
                        let character = FieldCharacter {
                            id: chr.character.id,
                            name: chr.character.name.clone(),
                            level: chr.character.level,
                            job: chr.character.job,
                            face: chr.character.face,
                            hair: chr.character.hair,
                            skin: chr.character.skin,
                            gender: chr.character.gender,
                            channel_id,
                            map_id: chr.character.map_id,
                            x: 240,
                            y: 190,
                            stance: 2,
                        };
                        let location = RuntimeLocation {
                            channel_id,
                            map_id: chr.character.map_id,
                            instance_id: 0,
                        };

                        // Build the character data packets
                        let keymap_packet =
                            build::world::keymap::build_keymap(&mut chr.key_binds).ok()?;
                        let char_info_packet =
                            build::world::char::build_char_info(&chr.character, channel_id).ok()?;

                        Some((character, location, keymap_packet, char_info_packet))
                    })(
                    );

                    if let Some((character, location, mut keymap_packet, mut char_info_packet)) =
                        reattach_result
                    {
                        // Send character data packets to client
                        self.writer.send_packet(&mut keymap_packet).await?;
                        self.writer.send_packet(&mut char_info_packet).await?;

                        // Register with world server
                        let event = ClientEvent::Connected {
                            client_id: character_id,
                            sender: self.server_tx.clone(),
                            character,
                            location,
                        };
                        self.world_tx
                            .send(event)
                            .await
                            .map_err(|_| RuntimeError::ChannelSend)?;
                    } else {
                        error!(character_id, "Failed to reattach session");
                    }
                }
                HandlerAction::ChangeChannel {
                    world_id,
                    channel_id,
                } => {
                    let Some(ref mut session) = self.session.session else {
                        return Err(RuntimeError::Handler(
                            "ChangeChannel requested without active session".to_string(),
                        ));
                    };
                    let channel = resolve_login_channel(world_id, channel_id).map_err(|e| {
                        RuntimeError::Handler(format!(
                            "Failed to resolve channel migration target: {e}"
                        ))
                    })?;

                    session.selected_world_id = Some(i16::from(world_id));
                    session.selected_channel_id = Some(i16::from(channel_id));
                    session.state = SessionState::Transition;
                    db::session::update_session(session)
                        .map_err(|e| RuntimeError::Handler(e.to_string()))?;

                    let mut redirect_packet =
                        build::world::channel::build_channel_change(channel.host, channel.port)
                    .map_err(|e| RuntimeError::Handler(e.to_string()))?;
                    self.writer.send_packet(&mut redirect_packet).await?;

                    self.world_tx
                        .send(ClientEvent::Disconnected {
                            client_id: self.client_id,
                        })
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                    self.client_id = 0;
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
        channel_id: u8,
        map_id: i32,
        character_name: String,
    ) -> Result<(), RuntimeError> {
        self.client_id = character_id;

        let event = ClientEvent::Connected {
            client_id: character_id,
            sender: self.server_tx.clone(),
            character: FieldCharacter {
                id: character_id,
                name: character_name,
                level: 1,
                job: 0,
                face: 20000,
                hair: 30000,
                skin: 0,
                gender: 0,
                channel_id,
                map_id,
                x: 240,
                y: 190,
                stance: 2,
            },
            location: RuntimeLocation {
                channel_id,
                map_id,
                instance_id: 0,
            },
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

        info!(self.client_id, "ClientActor cleanup complete");
    }
}
