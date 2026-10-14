mod buff;
mod combat;
mod dialogue;
mod loot;
mod npc_script;
mod quest;
mod script;

use crate::actor::registry::FieldRegistry;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::io::{PacketReader, PacketWriter};
use crate::message::{
    ClientHandle, DirectoryMessage, FieldHandle, FieldKey, FieldMessage, ServerMessage,
    CLIENT_QUEUE_CAPACITY,
};
use game::Player;
use net::dialogue::{Conversation, NpcInteraction};
use net::login_world::resolve_login_channel;
use net::packet::build;
use net::quests::QuestRequest;
use net::{ClientId, SpawnPosition, WorldAction, WorldContext, WorldRoute};
use packet::Packet;
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep_until, Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// How often a dirty player is written back while online.
const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(60);
/// Fallback stand position when a map has no usable spawn portal.
const FALLBACK_SPAWN: SpawnPosition = SpawnPosition {
    x: 0,
    y: 0,
    stance: 2,
};

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

/// Shared services a world client actor needs.
#[derive(Clone)]
pub struct WorldServices {
    pub directory: mpsc::Sender<DirectoryMessage>,
    pub fields: Arc<FieldRegistry>,
}

/// State that exists only after the client entered the game.
struct InGame {
    player: Player,
    field: FieldHandle,
}

/// Actor for one world-server connection. It is the single owner of its
/// character's state: fields and the directory only hold projections.
pub struct ClientActor {
    connection_id: u64,
    channel_id: u8,
    peer_ip: IpAddr,
    client_id: ClientId,
    reader: PacketReader,
    writer: PacketWriter,
    rx: mpsc::Receiver<ServerMessage>,
    tx: mpsc::Sender<ServerMessage>,
    kill: CancellationToken,
    services: WorldServices,
    session_id: Option<i32>,
    game: Option<InGame>,
    conversation: Option<Conversation>,
    /// A native NPC or quest script conversation.
    script: Option<npc_script::ActiveScript>,
    /// When natural recovery was last accepted.
    last_heal: Option<tokio::time::Instant>,
    /// When the player last sent movement; recovery needs them to rest.
    last_moved: Option<tokio::time::Instant>,
    /// A quest action waiting for its field to confirm the NPC is near.
    pending_quest: Option<(u64, QuestRequest)>,
    /// Portal scripts that already ran and won't run again this session.
    blocked_scripts: HashSet<String>,
    npc_request_id: u64,
    changing_channel: bool,
}

impl ClientActor {
    /// Set up encryption and send the handshake for a connection accepted on
    /// `channel_id`'s listener.
    pub async fn new(
        stream: TcpStream,
        peer_ip: IpAddr,
        channel_id: u8,
        services: WorldServices,
    ) -> Result<Self, RuntimeError> {
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

        let (tx, rx) = mpsc::channel(CLIENT_QUEUE_CAPACITY);
        Ok(Self {
            connection_id: NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed),
            channel_id,
            peer_ip,
            client_id: 0,
            reader,
            writer,
            rx,
            tx,
            kill: CancellationToken::new(),
            services,
            session_id: None,
            game: None,
            conversation: None,
            script: None,
            last_heal: None,
            last_moved: None,
            pending_quest: None,
            blocked_scripts: HashSet::new(),
            npc_request_id: 0,
            changing_channel: false,
        })
    }

    pub async fn run(mut self) {
        let mut autosave = interval(AUTOSAVE_INTERVAL);
        autosave.set_missed_tick_behavior(MissedTickBehavior::Delay);
        autosave.reset();
        let kill = self.kill.clone();

        let outcome = loop {
            let buffs_due = self.buffs_due();
            let step = tokio::select! {
                biased;
                _ = kill.cancelled() => Err(RuntimeError::ClientDisconnected),
                result = self.reader.read_packet() => match result {
                    Ok(packet) => self.handle_packet(packet).await,
                    Err(error) => Err(error),
                },
                Some(message) = self.rx.recv() => self.handle_server_message(message).await,
                _ = autosave.tick() => self.save_if_dirty().await,
                _ = sleep_until(buffs_due.unwrap_or_else(Instant::now)), if buffs_due.is_some() => {
                    self.update_buffs().await
                }
            };
            if let Err(error) = step {
                break error;
            }
        };

        match outcome {
            RuntimeError::ClientDisconnected => info!(self.client_id, "Client disconnected"),
            error => error!(self.client_id, %error, "Closing client connection"),
        }
        self.cleanup().await;
    }

    /// Write one packet to the socket. A kick interrupts a write that is
    /// blocked on a peer that stopped reading, and `PacketWriter` bounds it
    /// with a timeout either way, so the actor always reaches cleanup.
    async fn send(&mut self, packet: &mut Packet) -> Result<(), RuntimeError> {
        let kill = self.kill.clone();
        tokio::select! {
            biased;
            _ = kill.cancelled() => Err(RuntimeError::ClientDisconnected),
            result = self.writer.send_packet(packet) => result,
        }
    }

    fn handle(&self) -> ClientHandle {
        ClientHandle::new(
            self.client_id,
            self.connection_id,
            self.tx.clone(),
            self.kill.clone(),
        )
    }

    async fn handle_packet(&mut self, mut packet: Packet) -> Result<(), RuntimeError> {
        let opcode = packet.opcode();
        let Some(route) = net::world_route(opcode) else {
            debug!(opcode, "Ignoring unhandled world opcode");
            return Ok(());
        };

        let result = match (route, self.game.as_mut()) {
            (WorldRoute::PreGame(handler), None) => handler.handle(&mut packet),
            (WorldRoute::InGame(handler), Some(game)) => {
                let mut ctx = WorldContext {
                    client_id: self.client_id,
                    channel_id: self.channel_id,
                    player: &mut game.player,
                };
                handler.handle(&mut packet, &mut ctx)
            }
            _ => {
                warn!(opcode, "Opcode not valid in the current session stage");
                return Ok(());
            }
        };

        let result = result.map_err(|e| {
            error!(opcode, error = %e, "Handler error");
            RuntimeError::Handler(e.to_string())
        })?;
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }

    async fn apply(&mut self, action: WorldAction) -> Result<(), RuntimeError> {
        match action {
            WorldAction::Reply(mut packet) => self.send(&mut packet).await?,
            WorldAction::EnterGame { character_id } => self.enter_game(character_id).await?,
            WorldAction::ChangeField { map_id, spawn } => self.change_field(map_id, spawn).await?,
            WorldAction::FieldChat(packet) => {
                self.send_to_field(FieldMessage::Chat {
                    from: self.client_id,
                    packet,
                })
                .await?
            }
            WorldAction::FieldMove {
                packet,
                movement_bytes,
            } => {
                self.last_moved = Some(tokio::time::Instant::now());
                self.send_to_field(FieldMessage::Move {
                    from: self.client_id,
                    packet,
                    movement_bytes,
                })
                .await?
            }
            WorldAction::NpcInteraction(NpcInteraction::Start { object_id, .. }) => {
                self.conversation = None;
                self.script = None;
                self.npc_request_id = self.npc_request_id.wrapping_add(1);
                self.send_to_field(FieldMessage::NpcInteraction {
                    from: self.client_id,
                    object_id,
                    request_id: self.npc_request_id,
                })
                .await?
            }
            WorldAction::NpcInteraction(NpcInteraction::Respond {
                message_type,
                response,
                selection,
            }) => {
                if !self
                    .respond_to_script(message_type, response, selection)
                    .await?
                {
                    self.respond_to_npc(message_type, response).await?
                }
            }
            WorldAction::NaturalHeal { hp, mp } => self.natural_heal(hp, mp).await?,
            WorldAction::Field(request) => {
                self.send_to_field(FieldMessage::FromClient {
                    from: self.client_id,
                    request,
                })
                .await?
            }
            WorldAction::Died => self.died().await?,
            WorldAction::Quest(request) => self.request_quest_action(request).await?,
            WorldAction::ScriptPortal { portal } => self.touch_script_portal(&portal).await?,
            WorldAction::LookChanged(announce) => {
                let avatar = self.in_game()?.player.field_avatar(0, 0, 0);
                self.send_to_field(FieldMessage::UpdateAvatar {
                    client_id: self.client_id,
                    avatar,
                    announce: Some(announce),
                })
                .await?
            }
            WorldAction::Whisper(delivery) => {
                self.send_to_directory(DirectoryMessage::Whisper {
                    from: self.client_id,
                    delivery,
                })
                .await?
            }
            WorldAction::ChangeChannel { channel_id } => self.change_channel(channel_id).await?,
            WorldAction::Disconnect => return Err(RuntimeError::ClientDisconnected),
        }
        Ok(())
    }

    /// Claim the login server's single-use ticket for this channel, load the
    /// character, and join the world.
    async fn enter_game(&mut self, character_id: i32) -> Result<(), RuntimeError> {
        if self.game.is_some() || self.session_id.is_some() {
            warn!(character_id, "Ignoring repeated world entry");
            return Ok(());
        }

        let (channel, ip) = (i16::from(self.channel_id), self.peer_ip.into());
        let session =
            match spawn_db(move || db::session::claim_world_transition(character_id, channel, ip))
                .await
            {
                Ok(session) => session,
                Err(RuntimeError::Database(db::Error::NotFound)) => {
                    warn!(
                        character_id,
                        "World entry without a valid transition ticket"
                    );
                    return Err(RuntimeError::NotLoggedIn);
                }
                Err(error) => return Err(error),
            };
        self.session_id = Some(session.id);

        let player = spawn_db(move || db::player::load_player(character_id)).await?;
        if player.account_id != session.account_id {
            return Err(RuntimeError::NotLoggedIn);
        }
        self.client_id = character_id;

        let mut keymap = build::world::keymap::build_keymap(&player.keymap)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        let mut char_info = build::world::char::build_char_info(&player, self.channel_id)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut keymap).await?;
        self.send(&mut char_info).await?;

        self.send_to_directory(DirectoryMessage::Register {
            handle: self.handle(),
            name: player.name().to_string(),
            channel_id: self.channel_id,
        })
        .await?;

        let key = FieldKey::new(self.channel_id, player.stats.map_id);
        let spawn = net::spawn_position(player.stats.map_id, player.stats.spawn_point)
            .unwrap_or(FALLBACK_SPAWN);
        let field = self.services.fields.field(key);
        self.game = Some(InGame { player, field });
        self.join_current_field(spawn).await?;
        info!(
            character_id,
            channel_id = self.channel_id,
            "Entered the game"
        );
        Ok(())
    }

    /// Move field membership after the player changed maps.
    async fn change_field(
        &mut self,
        map_id: i32,
        spawn: Option<SpawnPosition>,
    ) -> Result<(), RuntimeError> {
        self.conversation = None;
        self.script = None;
        self.pending_quest = None;
        self.npc_request_id = self.npc_request_id.wrapping_add(1);

        self.send_to_field(FieldMessage::Leave {
            client_id: self.client_id,
        })
        .await?;
        let key = FieldKey::new(self.channel_id, map_id);
        let field = self.services.fields.field(key);
        let game = self.in_game_mut()?;
        game.field = field;
        let spawn = spawn
            .or_else(|| net::spawn_position(map_id, game.player.stats.spawn_point))
            .unwrap_or(FALLBACK_SPAWN);
        self.join_current_field(spawn).await
    }

    async fn join_current_field(&mut self, spawn: SpawnPosition) -> Result<(), RuntimeError> {
        let handle = self.handle();
        let game = self.in_game()?;
        let avatar = game.player.field_avatar(spawn.x, spawn.y, spawn.stance);
        self.send_to_field(FieldMessage::Join { handle, avatar })
            .await?;
        self.run_map_enter_script().await
    }

    /// Hand the session to another channel: persist, reissue the ticket, and
    /// tell the client where to reconnect.
    async fn change_channel(&mut self, channel_id: u8) -> Result<(), RuntimeError> {
        let world_id = self.in_game()?.player.world_id;
        let channel = resolve_login_channel(world_id, channel_id)
            .map_err(|e| RuntimeError::Handler(format!("Unknown channel: {e}")))?;
        let session_id = self.session_id.ok_or(RuntimeError::NotLoggedIn)?;

        self.cancel_buffs().await?;
        self.flush().await?;
        spawn_db(move || db::session::begin_channel_transition(session_id, i16::from(channel_id)))
            .await?;
        self.changing_channel = true;

        let mut redirect = build::world::channel::build_channel_change(channel.host, channel.port)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut redirect).await?;
        Err(RuntimeError::ClientDisconnected)
    }

    async fn handle_server_message(&mut self, message: ServerMessage) -> Result<(), RuntimeError> {
        match message {
            ServerMessage::NpcConversation {
                npc_id,
                field,
                request_id,
            } => {
                if request_id == self.npc_request_id {
                    self.start_npc_conversation(npc_id, field).await?;
                }
            }
            ServerMessage::QuestNpcChecked { request_id, nearby } => {
                self.quest_npc_checked(request_id, nearby).await?
            }
            ServerMessage::MobKilled {
                mob_id,
                object_id,
                exp,
                x,
                y,
            } => {
                self.mob_killed(mob_id, exp).await?;
                self.count_quest_kill(mob_id).await?;
                self.drop_loot(mob_id, object_id, x, y).await?
            }
            ServerMessage::PickedUp { drop } => self.picked_up(drop).await?,
            ServerMessage::SendPacket(mut packet) => self.send(&mut packet).await?,
        }
        Ok(())
    }

    /// Send to the current field, restarting it once if its actor died.
    async fn send_to_field(&mut self, message: FieldMessage) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        match game.field.tx.send(message).await {
            Ok(()) => Ok(()),
            Err(mpsc::error::SendError(message)) => {
                warn!(self.client_id, field = ?game.field.key, "Field actor gone; rejoining");
                let key = game.field.key;
                let spawn = net::spawn_position(key.map_id, game.player.stats.spawn_point)
                    .unwrap_or(FALLBACK_SPAWN);
                let avatar = game.player.field_avatar(spawn.x, spawn.y, spawn.stance);
                let field = self.services.fields.field(key);
                self.in_game_mut()?.field = field.clone();
                let rejoin = FieldMessage::Join {
                    handle: self.handle(),
                    avatar,
                };
                field
                    .tx
                    .send(rejoin)
                    .await
                    .map_err(|_| RuntimeError::ChannelSend)?;
                if !matches!(
                    message,
                    FieldMessage::Join { .. } | FieldMessage::Leave { .. }
                ) {
                    field
                        .tx
                        .send(message)
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                }
                Ok(())
            }
        }
    }

    async fn send_to_directory(&self, message: DirectoryMessage) -> Result<(), RuntimeError> {
        self.services
            .directory
            .send(message)
            .await
            .map_err(|_| RuntimeError::ChannelSend)
    }

    fn in_game(&self) -> Result<&InGame, RuntimeError> {
        self.game.as_ref().ok_or(RuntimeError::NotLoggedIn)
    }

    fn in_game_mut(&mut self) -> Result<&mut InGame, RuntimeError> {
        self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)
    }

    /// Persist the player if it has unsaved changes. Every mutation marks
    /// the player dirty, so a clean player already matches the database;
    /// skipping it also avoids racing the next channel's load after a
    /// channel change.
    pub(super) async fn flush(&mut self) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if !game.player.is_dirty() {
            return Ok(());
        }
        let snapshot = game.player.clone();
        spawn_db(move || db::player::save_player(&snapshot)).await?;
        game.player.mark_saved();
        Ok(())
    }

    async fn save_if_dirty(&mut self) -> Result<(), RuntimeError> {
        if let Err(error) = self.flush().await {
            // Keep playing; the next autosave or logout retries.
            error!(self.client_id, %error, "Autosave failed");
        }
        Ok(())
    }

    async fn cleanup(&mut self) {
        if self.game.is_some() {
            let _ = self
                .send_to_field(FieldMessage::Leave {
                    client_id: self.client_id,
                })
                .await;
            let _ = self
                .send_to_directory(DirectoryMessage::Unregister {
                    client_id: self.client_id,
                    connection_id: self.connection_id,
                })
                .await;
            if let Err(error) = self.flush().await {
                error!(self.client_id, %error, "Failed to save player on disconnect");
            }
        }

        if let (Some(session_id), false) = (self.session_id, self.changing_channel) {
            if let Err(error) = spawn_db(move || db::session::delete_session(session_id)).await {
                error!(self.client_id, %error, "Failed to close session");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio::time::{timeout, Instant};

    /// A peer that stops reading fills the socket, the actor blocks writing,
    /// its queue fills, and the kick must still end the actor (`run` always
    /// finishes with `cleanup`).
    #[tokio::test]
    async fn kicking_a_client_blocked_on_a_full_socket_ends_the_actor() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, addr) = listener.accept().await.unwrap();

        let (directory, _directory_rx) = mpsc::channel(16);
        let services = WorldServices {
            directory,
            fields: FieldRegistry::new(Duration::from_millis(100)),
        };
        let actor = ClientActor::new(stream, addr.ip(), 0, services)
            .await
            .unwrap();
        let handle = actor.handle();
        let running = tokio::spawn(actor.run());

        // Large packets so the kernel buffers fill quickly; the peer never reads.
        let payload = vec![0u8; 30_000];
        let deadline = Instant::now() + Duration::from_secs(30);
        while handle.send_packet(Packet::new(&payload)) {
            assert!(Instant::now() < deadline, "socket never filled");
            tokio::task::yield_now().await;
        }

        // The full queue kicked the client; the actor must stop promptly even
        // though it is mid-write to a socket nobody is reading.
        timeout(Duration::from_secs(2), running)
            .await
            .expect("actor stayed blocked on the socket write after being kicked")
            .expect("actor task panicked");
        drop(peer);
    }
}
