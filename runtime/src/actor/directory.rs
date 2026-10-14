use crate::actor::messenger::{Messengers, Outbox};
use crate::message::{ClientHandle, DirectoryMessage, ServerMessage};
use net::packet::build::world::messaging::{build_find_result, build_whisper_result, FindLocation};
use net::{BroadcastScope, ClientId, MessengerRequest, WhisperDelivery};
use packet::Packet;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// How often seats held through a channel change are checked.
const PURGE_INTERVAL: Duration = Duration::from_secs(10);

struct Entry {
    handle: ClientHandle,
    name: String,
    /// Zero-based channel.
    channel_id: u8,
}

/// World-wide directory of online characters: who is online, under which
/// name, on which channel. It owns no game state; it exists for features
/// that cross fields (whispers, finds, world broadcasts, Maple Messenger, and later
/// buddies/parties).
pub struct WorldDirectory {
    rx: mpsc::Receiver<DirectoryMessage>,
    clients: HashMap<ClientId, Entry>,
    names: HashMap<String, ClientId>,
    messengers: Messengers,
}

impl WorldDirectory {
    pub fn new(rx: mpsc::Receiver<DirectoryMessage>) -> Self {
        Self {
            rx,
            clients: HashMap::new(),
            names: HashMap::new(),
            messengers: Messengers::default(),
        }
    }

    pub async fn run(mut self) {
        let mut purge = tokio::time::interval(PURGE_INTERVAL);
        loop {
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(message) => self.handle(message),
                    None => break,
                },
                _ = purge.tick() => {
                    let out = self.messengers.purge(Instant::now());
                    self.deliver(out);
                }
            }
        }
    }

    fn handle(&mut self, message: DirectoryMessage) {
        match message {
            DirectoryMessage::Register {
                handle,
                name,
                channel_id,
            } => self.register(handle, name, channel_id),
            DirectoryMessage::Unregister {
                client_id,
                connection_id,
                changing_channel,
            } => self.unregister(client_id, connection_id, changing_channel),
            DirectoryMessage::Whisper { from, delivery } => self.whisper(from, delivery),
            DirectoryMessage::Find { from, target_name } => self.find(from, &target_name),
            DirectoryMessage::Broadcast {
                from,
                scope,
                packet,
            } => self.broadcast(from, scope, packet),
            DirectoryMessage::Messenger { from, request } => self.messenger(from, request),
            DirectoryMessage::LookChanged { client_id, look } => {
                let out = self.messengers.look_changed(client_id, look);
                self.deliver(out);
            }
        }
    }

    fn register(&mut self, handle: ClientHandle, name: String, channel_id: u8) {
        let client_id = handle.client_id;
        if let Some(previous) = self.clients.remove(&client_id) {
            // The same character connected again: only one session may live.
            warn!(
                client_id,
                "Duplicate login; disconnecting previous connection"
            );
            previous.handle.kick();
            self.names.remove(&previous.name);
        }
        info!(client_id, name, channel_id, "Character online");
        self.names.insert(name.clone(), client_id);
        self.clients.insert(
            client_id,
            Entry {
                handle,
                name,
                channel_id,
            },
        );
        let out = self.messengers.connected(client_id, channel_id);
        self.deliver(out);
    }

    fn unregister(&mut self, client_id: ClientId, connection_id: u64, changing_channel: bool) {
        // Ignore a stale unregister from a connection that was replaced.
        let is_current = self
            .clients
            .get(&client_id)
            .is_some_and(|entry| entry.handle.connection_id == connection_id);
        if is_current {
            if let Some(entry) = self.clients.remove(&client_id) {
                self.names.remove(&entry.name);
                info!(client_id, "Character offline");
                let out = self
                    .messengers
                    .disconnected(client_id, changing_channel, Instant::now());
                self.deliver(out);
            }
        }
    }

    fn whisper(&self, from: ClientId, delivery: WhisperDelivery) {
        let Some(sender) = self.clients.get(&from) else {
            return;
        };
        let delivered = self
            .names
            .get(&delivery.target_name)
            .and_then(|id| self.clients.get(id))
            .is_some_and(|target| target.handle.send_packet(delivery.recipient_packet));
        sender.handle.send_packet(if delivered {
            delivery.sender_success_packet
        } else {
            delivery.sender_failure_packet
        });
    }

    /// Tell the asker where a character is: their channel if it isn't the
    /// asker's, otherwise their map, which only their own client knows.
    fn find(&self, from: ClientId, target_name: &str) {
        let Some(asker) = self.clients.get(&from) else {
            return;
        };
        let target = self
            .names
            .get(target_name)
            .and_then(|id| self.clients.get(id));
        let packet = match target {
            Some(target) if target.channel_id == asker.channel_id => {
                target.handle.send(ServerMessage::Locate {
                    reply_to: asker.handle.clone(),
                });
                return;
            }
            Some(target) => {
                build_find_result(&target.name, FindLocation::Channel(target.channel_id))
            }
            None => build_whisper_result(target_name, false),
        };
        match packet {
            Ok(packet) => {
                asker.handle.send_packet(packet);
            }
            Err(e) => error!(client_id = from, "find result: {e}"),
        }
    }

    fn messenger(&mut self, from: ClientId, request: MessengerRequest) {
        let Some(sender) = self.clients.get(&from) else {
            return;
        };
        let out = match request {
            MessengerRequest::Open { messenger_id, look } => {
                self.messengers
                    .open(from, &sender.name, sender.channel_id, look, messenger_id)
            }
            MessengerRequest::Leave => self.messengers.leave(from),
            MessengerRequest::Invite { target } => {
                let target_id = self.names.get(&target).copied();
                self.messengers.invite(from, target_id, &target)
            }
            MessengerRequest::Decline { inviter } => match self.names.get(&inviter) {
                Some(&inviter) => self.messengers.decline(from, &sender.name, inviter),
                None => Outbox::new(),
            },
            MessengerRequest::Chat { line } => self.messengers.chat(from, &line),
        };
        self.deliver(out);
    }

    fn deliver(&self, out: Outbox) {
        for (to, packet) in out {
            if let Some(entry) = self.clients.get(&to) {
                entry.handle.send_packet(packet);
            }
        }
    }

    fn broadcast(&self, from: ClientId, scope: BroadcastScope, packet: Packet) {
        for (&client_id, entry) in &self.clients {
            if matches!(scope, BroadcastScope::WorldExcludeSelf) && client_id == from {
                continue;
            }
            entry.handle.send_packet(packet.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ServerMessage;
    use tokio_util::sync::CancellationToken;

    fn handle(
        client_id: ClientId,
        connection_id: u64,
    ) -> (
        ClientHandle,
        mpsc::Receiver<ServerMessage>,
        CancellationToken,
    ) {
        let (tx, rx) = mpsc::channel(4);
        let kill = CancellationToken::new();
        (
            ClientHandle::new(client_id, connection_id, tx, kill.clone()),
            rx,
            kill,
        )
    }

    #[test]
    fn duplicate_login_kicks_the_previous_connection_only() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let (first, _first_rx, first_kill) = handle(1, 10);
        let (second, _second_rx, second_kill) = handle(1, 11);

        directory.register(first, "dup".into(), 0);
        directory.register(second, "dup".into(), 0);
        assert!(first_kill.is_cancelled());
        assert!(!second_kill.is_cancelled());

        // The old connection's late unregister must not remove the new one.
        directory.unregister(1, 10, false);
        assert!(directory.names.contains_key("dup"));
        directory.unregister(1, 11, false);
        assert!(!directory.names.contains_key("dup"));
    }

    #[test]
    fn full_client_queue_kicks_instead_of_blocking() {
        let (client, _rx, kill) = handle(1, 1);
        for _ in 0..4 {
            assert!(client.send_packet(Packet::new_empty()));
        }
        assert!(!client.send_packet(Packet::new_empty()));
        assert!(kill.is_cancelled());
    }
}
