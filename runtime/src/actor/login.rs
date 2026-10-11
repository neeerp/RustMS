use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::io::{PacketReader, PacketWriter};
use db::session::{NewSession, Session, SessionState};
use net::packet::build;
use net::{LoginAction, LoginContext};
use packet::Packet;
use rand::{thread_rng, Rng};
use std::net::IpAddr;
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

/// Actor for one login-server connection.
pub struct LoginClientActor {
    reader: PacketReader,
    writer: PacketWriter,
    session: Option<Session>,
    peer_ip: IpAddr,
}

impl LoginClientActor {
    pub async fn new(stream: TcpStream, peer_ip: IpAddr) -> Result<Self, RuntimeError> {
        let (recv_iv, send_iv) = {
            let mut rng = thread_rng();
            (rng.gen::<[u8; 4]>().to_vec(), rng.gen::<[u8; 4]>().to_vec())
        };

        let (read_half, write_half) = stream.into_split();
        let reader = PacketReader::new(read_half, &recv_iv);
        let mut writer = PacketWriter::new(write_half, &send_iv);
        let handshake = build::build_handshake_packet(&recv_iv, &send_iv)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        writer.send_handshake(&handshake.bytes).await?;

        Ok(Self {
            reader,
            writer,
            session: None,
            peer_ip,
        })
    }

    pub async fn run(mut self) {
        loop {
            let step = match self.reader.read_packet().await {
                Ok(packet) => self.handle_packet(packet).await,
                Err(error) => Err(error),
            };
            match step {
                Ok(()) => {}
                Err(RuntimeError::ClientDisconnected) => {
                    info!("Login client disconnected");
                    break;
                }
                Err(error) => {
                    error!(%error, "Closing login connection");
                    break;
                }
            }
        }
        self.cleanup().await;
    }

    async fn handle_packet(&mut self, mut packet: Packet) -> Result<(), RuntimeError> {
        let opcode = packet.opcode();
        let Some(route) = net::login_route(opcode) else {
            debug!(opcode, "Ignoring unhandled login opcode");
            return Ok(());
        };
        if !route.stage.admits(&self.session) {
            warn!(opcode, stage = ?route.stage, "Opcode not valid in the current session stage");
            return Ok(());
        }

        // Login handlers hit the database, so they run on the blocking pool.
        let handler = route.handler;
        let session = self.session.clone();
        let result = tokio::task::spawn_blocking(move || {
            handler.handle(&mut packet, &LoginContext { session: &session })
        })
        .await
        .map_err(|e| RuntimeError::Handler(format!("Task join error: {e}")))?
        .map_err(|e| {
            error!(opcode, error = %e, "Handler error");
            RuntimeError::Handler(e.to_string())
        })?;

        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }

    async fn apply(&mut self, action: LoginAction) -> Result<(), RuntimeError> {
        match action {
            LoginAction::Reply(mut packet) => self.writer.send_packet(&mut packet).await?,
            LoginAction::CreateSession {
                account_id,
                hwid,
                state,
            } => {
                if let Some(previous) = self.session.take() {
                    spawn_db(move || db::session::delete_session(previous.id)).await?;
                }
                let ip = self.peer_ip.into();
                let session = spawn_db(move || {
                    db::session::create_session(NewSession {
                        account_id,
                        ip,
                        hwid: &hwid,
                        state,
                    })
                })
                .await?;
                info!(account_id, "Session created");
                self.session = Some(session);
            }
            LoginAction::SetSessionState(state) => {
                let id = self.session_id()?;
                self.session = Some(spawn_db(move || db::session::set_session_state(id, state)).await?);
            }
            LoginAction::SelectChannel {
                world_id,
                channel_id,
            } => {
                let id = self.session_id()?;
                self.session = Some(
                    spawn_db(move || {
                        db::session::update_session_selection(
                            id,
                            i16::from(world_id),
                            i16::from(channel_id),
                        )
                    })
                    .await?,
                );
            }
            LoginAction::EnterWorld { character_id } => {
                let id = self.session_id()?;
                // Fails unless the character belongs to the session's account,
                // so the redirect queued after this action is never sent.
                self.session = Some(
                    spawn_db(move || db::session::begin_world_transition(id, character_id))
                        .await?,
                );
                info!(character_id, "World transition ticket issued");
            }
            LoginAction::Disconnect => return Err(RuntimeError::ClientDisconnected),
        }
        Ok(())
    }

    fn session_id(&self) -> Result<i32, RuntimeError> {
        self.session
            .as_ref()
            .map(|session| session.id)
            .ok_or(RuntimeError::NotLoggedIn)
    }

    /// A session that is not handing off to the world dies with its
    /// connection.
    async fn cleanup(&mut self) {
        if let Some(session) = self.session.take() {
            if session.state != SessionState::Transition {
                if let Err(error) = spawn_db(move || db::session::delete_session(session.id)).await {
                    error!(%error, "Failed to close login session");
                }
            }
        }
    }
}

/// Accepts login connections and spawns one actor per client.
pub struct LoginServerActor;

impl LoginServerActor {
    pub async fn run(addr: &str) -> Result<(), RuntimeError> {
        // Sessions owned by a previous run of this server cannot be resumed.
        let cleared = spawn_db(db::session::clear_login_sessions).await?;
        info!(cleared, "Cleared stale login sessions");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!(addr, "LoginServerActor listening");

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!(%peer_addr, "Login connection accepted");
                    tokio::spawn(async move {
                        match LoginClientActor::new(stream, peer_addr.ip()).await {
                            Ok(actor) => actor.run().await,
                            Err(e) => error!(error = %e, "Failed to create LoginClientActor"),
                        }
                    });
                }
                Err(e) => error!(error = %e, "Error accepting login connection"),
            }
        }
    }
}
