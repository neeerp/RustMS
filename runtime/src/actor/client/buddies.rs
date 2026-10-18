use super::ClientActor;
use crate::{
    db::spawn_db,
    error::RuntimeError,
    message::{DirectoryMessage, ListedBuddy},
};
use db::buddy::{AddError, Buddy, DEFAULT_GROUP};
use net::error::NetworkError;
use net::packet::build::world::buddy::{
    build_buddy_channel, build_buddy_chat, build_buddy_error, build_buddy_request, BuddyError,
};
use net::{BuddyRequest, ClientId};
use packet::Packet;

/// Longest group name the client's list can show.
const GROUP_MAX_LEN: usize = 16;

fn built(packet: Result<Packet, NetworkError>) -> Result<Packet, RuntimeError> {
    packet.map_err(|e| RuntimeError::Handler(e.to_string()))
}

impl ClientActor {
    /// On entering the game: show the list, unanswered requests included,
    /// and tell mutual buddies we're on this channel. The client asks about
    /// each request once the map has loaded. It starts with an empty list,
    /// so nothing is sent for one.
    pub(super) async fn load_buddies(&mut self) -> Result<(), RuntimeError> {
        let rows = self.buddy_rows().await?;
        if rows.is_empty() {
            return Ok(());
        }
        let mutual = rows.iter().filter(|b| b.mutual).map(|b| b.id).collect();
        self.announce_channel(mutual, Some(self.channel_id)).await?;
        self.show_buddies(rows).await
    }

    /// On leaving the game (not a channel change, which announces the new
    /// channel instead): mutual buddies see the player go offline.
    pub(super) async fn announce_offline(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let mutual = spawn_db(move || db::buddy::mutual_ids(cid)).await?;
        self.announce_channel(mutual, None).await
    }

    pub(super) async fn buddy_request(
        &mut self,
        request: BuddyRequest,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        match request {
            BuddyRequest::Add { name, group } => self.add_buddy(name, group).await?,
            BuddyRequest::Accept { id } => {
                if spawn_db(move || db::buddy::accept(cid, id)).await? {
                    // They asked first, so they list us: now both see each other.
                    self.announce_channel(vec![id], Some(self.channel_id))
                        .await?;
                }
                let rows = self.buddy_rows().await?;
                self.show_buddies(rows).await?;
            }
            BuddyRequest::Remove { id } => {
                let removed = spawn_db(move || db::buddy::remove(cid, id)).await?;
                if removed == Some(false) {
                    // Whether or not they still list us, they no longer see us.
                    self.announce_channel(vec![id], None).await?;
                }
                let rows = self.buddy_rows().await?;
                self.show_buddies(rows).await?;
            }
            BuddyRequest::Chat { recipients, text } => {
                let mutual = spawn_db(move || db::buddy::mutual_ids(cid)).await?;
                let to: Vec<ClientId> = recipients
                    .into_iter()
                    .filter(|id| mutual.contains(id))
                    .collect();
                let name = self.in_game()?.player.stats.name.clone();
                let packet = built(build_buddy_chat(&name, &text))?;
                self.send_to_directory(DirectoryMessage::Deliver { to, packet })
                    .await?;
            }
        }
        Ok(())
    }

    async fn add_buddy(&mut self, name: String, group: String) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let (cid, capacity, my_name) = (
            self.client_id,
            player.buddy_capacity,
            player.stats.name.clone(),
        );
        let group = match group.trim() {
            "" => DEFAULT_GROUP.to_string(),
            group => group.chars().take(GROUP_MAX_LEN).collect(),
        };
        let added = spawn_db(move || db::buddy::add(cid, &name, &group, capacity)).await?;
        let added = match added {
            Ok(added) => added,
            Err(error) => {
                let error = match error {
                    AddError::NotFound => BuddyError::NotFound,
                    AddError::AlreadyAdded => BuddyError::AlreadyAdded,
                    AddError::OwnListFull => BuddyError::OwnListFull,
                    AddError::TheirListFull => BuddyError::TheirListFull,
                };
                return self.send(&mut built(build_buddy_error(error))?).await;
            }
        };
        let packet = if added.mutual {
            build_buddy_channel(cid, Some(self.channel_id))
        } else {
            build_buddy_request(cid, &my_name, Some(self.channel_id))
        };
        self.send_to_directory(DirectoryMessage::Deliver {
            to: vec![added.id],
            packet: built(packet)?,
        })
        .await?;
        let rows = self.buddy_rows().await?;
        self.show_buddies(rows).await
    }

    async fn buddy_rows(&self) -> Result<Vec<Buddy>, RuntimeError> {
        let cid = self.client_id;
        spawn_db(move || db::buddy::list(cid)).await
    }

    /// Send the rows through the directory, which knows channels. v83 keeps
    /// the requests in the list too, hidden, so that answering one finds it.
    async fn show_buddies(&mut self, rows: Vec<Buddy>) -> Result<(), RuntimeError> {
        let buddies = rows
            .into_iter()
            .map(|buddy| ListedBuddy {
                id: buddy.id,
                name: buddy.name,
                group: buddy.group,
                mutual: buddy.mutual,
                pending: buddy.pending,
            })
            .collect();
        self.send_to_directory(DirectoryMessage::BuddyList {
            owner: self.client_id,
            buddies,
        })
        .await
    }

    async fn announce_channel(
        &mut self,
        to: Vec<ClientId>,
        channel: Option<u8>,
    ) -> Result<(), RuntimeError> {
        if to.is_empty() {
            return Ok(());
        }
        let packet = built(build_buddy_channel(self.client_id, channel))?;
        self.send_to_directory(DirectoryMessage::Deliver { to, packet })
            .await
    }
}
