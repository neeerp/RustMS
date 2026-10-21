use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::DirectoryMessage};
use db::parcel::{NewParcel, Sent};
use net::duey::{result, DueyRequest, DUEY, INBOX_LIMIT, MESSAGE_LIMIT};
use net::packet::build::world::parcel::{
    build_parcel_arrived, build_parcel_removed, build_parcel_result, build_parcel_window,
    build_parcels_waiting, build_quick_send_window, DELETED,
};
use net::packet::build::world::stat::build_stat_update;
use std::time::SystemTime;
use tokio::sync::oneshot;
use tracing::{debug, info};

/// Which Duey window the player has open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DueyWindow {
    /// The parcels waiting, and a normal send.
    Parcels,
    /// A quick send only, from the Quick Delivery Coupon.
    QuickSend,
}

impl ClientActor {
    /// On entering the game: tell the player of parcels that arrived while
    /// they were away.
    pub(super) async fn announce_parcels(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let quick = spawn_db(move || db::parcel::announce(cid, SystemTime::now())).await?;
        match quick {
            Some(quick) => self.send(&mut build_parcels_waiting(quick)?).await,
            None => Ok(()),
        }
    }

    /// Open Duey's window if `npc_id` is Duey.
    pub(super) async fn open_duey(&mut self, npc_id: i32) -> Result<bool, RuntimeError> {
        if npc_id != DUEY {
            return Ok(false);
        }
        self.open_parcels().await?;
        Ok(true)
    }

    /// Open the window with the parcels waiting; those that expired go, and
    /// the window says so. The client can't take a second window, so one
    /// already open stays as it is. It stays open until the player closes
    /// it.
    async fn open_parcels(&mut self) -> Result<(), RuntimeError> {
        if self.duey.is_some() {
            return Ok(());
        }
        let cid = self.client_id;
        let (waiting, expired) = spawn_db(move || db::parcel::open(cid, SystemTime::now())).await?;
        self.send(&mut build_parcel_window(false, &waiting, &expired)?)
            .await?;
        self.duey = Some(DueyWindow::Parcels);
        Ok(())
    }

    pub(super) async fn duey_request(&mut self, request: DueyRequest) -> Result<(), RuntimeError> {
        let window = self.duey;
        match request {
            DueyRequest::Open => self.open_parcels().await,
            DueyRequest::OpenQuickSend if window.is_none() => {
                self.duey = Some(DueyWindow::QuickSend);
                self.send(&mut build_quick_send_window()?).await
            }
            // The client doesn't ask with a window open, but would wait.
            DueyRequest::OpenQuickSend => self.send(&mut build_stat_update(true, &[])?).await,
            DueyRequest::Close => {
                self.duey = None;
                Ok(())
            }
            DueyRequest::Send {
                kind,
                slot,
                quantity,
                mesos,
                to,
                quick,
            } if window
                == Some(match quick {
                    Some(_) => DueyWindow::QuickSend,
                    None => DueyWindow::Parcels,
                }) =>
            {
                self.send_parcel(kind, slot, quantity, mesos, to, quick)
                    .await
            }
            DueyRequest::Claim { id } if window == Some(DueyWindow::Parcels) => {
                self.claim_parcel(id).await
            }
            DueyRequest::Delete { id } if window == Some(DueyWindow::Parcels) => {
                let cid = self.client_id;
                let deleted = spawn_db(move || db::parcel::delete(cid, i64::from(id))).await?;
                let mut packet = match deleted {
                    true => build_parcel_removed(id, DELETED)?,
                    false => build_parcel_result(result::UNKNOWN)?,
                };
                self.send(&mut packet).await
            }
            request => {
                debug!(
                    self.client_id,
                    ?request,
                    ?window,
                    "Duey request out of place"
                );
                Ok(())
            }
        }
    }

    /// Send a parcel to the character named `to`. What it costs leaves the
    /// player as the parcel is saved, in one transaction; if saving fails
    /// the player is as they were.
    async fn send_parcel(
        &mut self,
        kind: u8,
        slot: i16,
        quantity: i16,
        mesos: i32,
        to: String,
        quick: Option<String>,
    ) -> Result<(), RuntimeError> {
        if quick
            .as_ref()
            .is_some_and(|message| message.chars().count() > MESSAGE_LIMIT)
        {
            return self
                .send(&mut build_parcel_result(result::INCORRECT)?)
                .await;
        }
        let receiver = spawn_db(move || db::parcel::recipient(&to)).await?;
        let player = &self.in_game()?.player;
        let refusal = match &receiver {
            None => Some(result::NO_RECEIVER),
            Some(receiver) if receiver.id == self.client_id => Some(result::NO_RECEIVER),
            Some(receiver) if receiver.account_id == player.account_id => {
                Some(result::SAME_ACCOUNT)
            }
            Some(_) => None,
        };
        let (Some(receiver), None) = (receiver, refusal) else {
            let mode = refusal.unwrap_or(result::NO_RECEIVER);
            return self.send(&mut build_parcel_result(mode)?).await;
        };

        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let sending = match net::duey::send(player, kind, slot, quantity, mesos, quick.is_some())? {
            Ok(sending) => sending,
            Err(mode) => return self.send(&mut build_parcel_result(mode)?).await,
        };
        let (snapshot, sender) = (player.clone(), player.stats.name.clone());
        let item = sending.item.clone();
        let (receiver_id, only) = (receiver.id, sending.only);
        let sent = spawn_db(move || {
            let parcel = NewParcel {
                receiver_id,
                sender: &sender,
                mesos,
                message: quick.as_deref(),
                quick: quick.is_some(),
                sent_at: SystemTime::now(),
                item: item.as_ref(),
            };
            db::parcel::send(&parcel, only, INBOX_LIMIT as i64, &snapshot)
        })
        .await;
        let player = &mut self.in_game_mut()?.player;
        let mode = match sent {
            Ok(Sent::Sent) => result::SENT,
            Ok(Sent::InboxFull) => result::INBOX_FULL,
            Ok(Sent::OnlyOneWaiting) => result::ONLY_ONE_WAITING,
            Err(error) => {
                *player = before;
                return Err(error);
            }
        };
        if mode != result::SENT {
            *player = before;
            return self.send(&mut build_parcel_result(mode)?).await;
        }
        player.mark_saved();
        let sender = player.stats.name.clone();
        info!(
            self.client_id,
            to = receiver.id,
            mesos,
            item = ?sending.item.as_ref().map(|item| (item.item_id, item.quantity)),
            "Parcel sent"
        );
        let mut packets = sending.packets;
        packets.push(build_parcel_result(result::SENT)?);
        self.send_all(&mut packets).await?;
        self.tell_receiver(receiver.id, &sender).await
    }

    /// Tell the receiver of a parcel, if they're online, that it arrived.
    async fn tell_receiver(&mut self, receiver: i32, sender: &str) -> Result<(), RuntimeError> {
        let (reply, online) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::Online {
            client_id: receiver,
            reply,
        })
        .await?;
        if !online.await.unwrap_or(false) {
            return Ok(());
        }
        let quick = spawn_db(move || db::parcel::announce(receiver, SystemTime::now())).await?;
        let Some(quick) = quick else {
            return Ok(());
        };
        self.send_to_directory(DirectoryMessage::Deliver {
            to: vec![receiver],
            packet: build_parcel_arrived(sender, quick)?,
        })
        .await
    }

    /// Give the player what parcel `id` holds. It leaves the parcels as the
    /// player is saved, in one transaction.
    async fn claim_parcel(&mut self, id: i32) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let parcel =
            spawn_db(move || db::parcel::get(cid, i64::from(id), SystemTime::now())).await?;
        let Some(parcel) = parcel else {
            return self.send(&mut build_parcel_result(result::UNKNOWN)?).await;
        };
        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let mut packets = match net::duey::claim(player, &parcel)? {
            Ok(packets) => packets,
            Err(mode) => return self.send(&mut build_parcel_result(mode)?).await,
        };
        let snapshot = player.clone();
        let claimed = spawn_db(move || db::parcel::claim(cid, parcel.id, &snapshot)).await;
        let player = &mut self.in_game_mut()?.player;
        match claimed {
            Ok(true) => player.mark_saved(),
            Ok(false) => {
                *player = before;
                return self.send(&mut build_parcel_result(result::UNKNOWN)?).await;
            }
            Err(error) => {
                *player = before;
                return Err(error);
            }
        }
        info!(self.client_id, id, "Parcel claimed");
        self.send_all(&mut packets).await
    }
}
