use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::DirectoryMessage};
use db::parcel::{NewParcel, Parcel, Sent};
use net::duey::{result, DueyRequest, DUEY, INBOX_LIMIT, MESSAGE_LIMIT};
use net::packet::build::world::parcel::{
    build_parcel_added, build_parcel_arrived, build_parcel_removed, build_parcel_result,
    build_parcel_window, build_parcels_waiting, build_quick_send_window, DELETED,
};
use net::packet::build::world::stat::build_stat_update;
use std::time::SystemTime;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

/// Which Duey window the player has open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DueyWindow {
    /// The parcels waiting, and a normal send: Duey's own.
    Parcels,
    /// The parcels waiting only, from the notifier of a quick delivery.
    Receive,
    /// A quick send only, from the Quick Delivery Coupon.
    QuickSend,
}

impl ClientActor {
    /// On entering the game: tell the player of parcels that arrived while
    /// they were away. Failing to doesn't keep them out of the game.
    pub(super) async fn announce_parcels(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        match spawn_db(move || db::parcel::announce(cid, SystemTime::now())).await {
            Ok(Some(quick)) => self.send(&mut build_parcels_waiting(quick)?).await,
            Ok(None) => Ok(()),
            Err(error) => {
                warn!(self.client_id, %error, "Couldn't announce parcels");
                Ok(())
            }
        }
    }

    /// Open Duey's window if `npc_id` is Duey.
    pub(super) async fn open_duey(&mut self, npc_id: i32) -> Result<bool, RuntimeError> {
        if npc_id != DUEY {
            return Ok(false);
        }
        self.open_parcels(DueyWindow::Parcels).await?;
        Ok(true)
    }

    /// Open the window with the parcels waiting; those that expired go, and
    /// the window says so. The client can't take a second window, so one
    /// already open stays as it is. It stays open until the player closes
    /// it.
    async fn open_parcels(&mut self, window: DueyWindow) -> Result<(), RuntimeError> {
        if self.duey.is_some() {
            return Ok(());
        }
        let cid = self.client_id;
        let (waiting, expired) = spawn_db(move || db::parcel::open(cid, SystemTime::now())).await?;
        let receive_only = window == DueyWindow::Receive;
        self.send(&mut build_parcel_window(receive_only, &waiting, &expired)?)
            .await?;
        self.duey = Some(window);
        Ok(())
    }

    pub(super) async fn duey_request(&mut self, request: DueyRequest) -> Result<(), RuntimeError> {
        let window = self.duey;
        let receiving = matches!(window, Some(DueyWindow::Parcels | DueyWindow::Receive));
        match request {
            DueyRequest::Open => self.open_parcels(DueyWindow::Receive).await,
            // The client only uses a coupon with no window open, so any we
            // thought open has gone. The coupon waits for the send; the
            // empty update lets the client use items again.
            DueyRequest::OpenQuickSend => {
                self.duey = Some(DueyWindow::QuickSend);
                self.send_all(&mut vec![
                    build_quick_send_window()?,
                    build_stat_update(true, &[])?,
                ])
                .await
            }
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
            DueyRequest::Claim { id } if receiving => self.claim_parcel(id).await,
            DueyRequest::Delete { id } if receiving => {
                let cid = self.client_id;
                let mode = match spawn_db(move || db::parcel::delete(cid, i64::from(id))).await {
                    Ok(true) => None,
                    Ok(false) => Some(result::UNKNOWN),
                    Err(error) => {
                        warn!(self.client_id, id, %error, "Couldn't delete a parcel");
                        Some(result::UNKNOWN)
                    }
                };
                let mut packet = match mode {
                    None => build_parcel_removed(id, DELETED)?,
                    Some(mode) => build_parcel_result(mode)?,
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
                // A window of another kind waits for an answer before it
                // takes requests again.
                match (window, request) {
                    (
                        Some(_),
                        DueyRequest::Send { .. }
                        | DueyRequest::Claim { .. }
                        | DueyRequest::Delete { .. },
                    ) => self.send(&mut build_parcel_result(result::UNKNOWN)?).await,
                    _ => Ok(()),
                }
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
        // The database keeps no NULs in text.
        if quick.as_ref().is_some_and(|message| {
            message.chars().count() > MESSAGE_LIMIT || message.contains('\0')
        }) {
            return self
                .send(&mut build_parcel_result(result::INCORRECT)?)
                .await;
        }
        let receiver = match to.contains('\0') {
            true => Ok(None),
            false => spawn_db(move || db::parcel::recipient(&to)).await,
        };
        let player = &self.in_game()?.player;
        // The sender's own name is on their account too.
        let refusal = match &receiver {
            Err(error) => {
                warn!(self.client_id, %error, "Couldn't look up a parcel's receiver");
                Some(result::UNKNOWN)
            }
            Ok(None) => Some(result::NO_RECEIVER),
            Ok(Some(receiver)) if receiver.account_id == player.account_id => {
                Some(result::SAME_ACCOUNT)
            }
            Ok(Some(_)) => None,
        };
        let (Ok(Some(receiver)), None) = (receiver, refusal) else {
            let mode = refusal.unwrap_or(result::UNKNOWN);
            return self.send(&mut build_parcel_result(mode)?).await;
        };

        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let sending = match net::duey::send(player, kind, slot, quantity, mesos, quick.is_some()) {
            Ok(Ok(sending)) => sending,
            Ok(Err(mode)) => return self.send(&mut build_parcel_result(mode)?).await,
            Err(error) => {
                *player = before;
                return Err(error.into());
            }
        };
        let (snapshot, sender) = (player.clone(), player.stats.name.clone());
        let mut parcel = Parcel {
            id: 0,
            sender,
            mesos,
            quick: quick.is_some(),
            message: quick,
            sent_at: SystemTime::now(),
            item: sending.item.clone(),
        };
        let (receiver_id, only, new) = (receiver.id, sending.only, parcel.clone());
        let sent = spawn_db(move || {
            let parcel = NewParcel {
                receiver_id,
                sender: &new.sender,
                mesos: new.mesos,
                message: new.message.as_deref(),
                quick: new.quick,
                sent_at: new.sent_at,
                item: new.item.as_ref(),
            };
            db::parcel::send(&parcel, only, INBOX_LIMIT as i64, &snapshot)
        })
        .await;
        let client_id = self.client_id;
        let player = &mut self.in_game_mut()?.player;
        let mode = match sent {
            Ok(Sent::Sent(id)) => {
                parcel.id = id;
                result::SENT
            }
            Ok(Sent::InboxFull) => result::INBOX_FULL,
            Ok(Sent::OnlyOneWaiting) => result::ONLY_ONE_WAITING,
            Err(error) => {
                warn!(client_id, %error, "Couldn't send a parcel");
                result::UNKNOWN
            }
        };
        if mode != result::SENT {
            *player = before;
            return self.send(&mut build_parcel_result(mode)?).await;
        }
        player.mark_saved();
        info!(
            self.client_id,
            to = receiver.id,
            mesos,
            item = ?parcel.item.as_ref().map(|item| (item.item_id, item.quantity)),
            "Parcel sent"
        );
        let mut packets = sending.packets;
        packets.push(build_parcel_result(result::SENT)?);
        self.send_all(&mut packets).await?;
        if let Err(error) = self.tell_receiver(receiver.id, &parcel).await {
            warn!(self.client_id, %error, "Couldn't tell a parcel's receiver");
        }
        Ok(())
    }

    /// Tell the receiver of `parcel`, if they're online, that it arrived;
    /// a window they have open lists it.
    async fn tell_receiver(&mut self, receiver: i32, parcel: &Parcel) -> Result<(), RuntimeError> {
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
        for packet in [
            build_parcel_added(parcel)?,
            build_parcel_arrived(&parcel.sender, quick)?,
        ] {
            self.send_to_directory(DirectoryMessage::Deliver {
                to: vec![receiver],
                packet,
            })
            .await?;
        }
        Ok(())
    }

    /// Give the player what parcel `id` holds. It leaves the parcels as the
    /// player is saved, in one transaction.
    async fn claim_parcel(&mut self, id: i32) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let parcel =
            match spawn_db(move || db::parcel::get(cid, i64::from(id), SystemTime::now())).await {
                Ok(parcel) => parcel,
                Err(error) => {
                    warn!(self.client_id, id, %error, "Couldn't load a parcel");
                    None
                }
            };
        let Some(parcel) = parcel else {
            return self.send(&mut build_parcel_result(result::UNKNOWN)?).await;
        };
        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let mut packets = match net::duey::claim(player, &parcel) {
            Ok(Ok(packets)) => packets,
            Ok(Err(mode)) => return self.send(&mut build_parcel_result(mode)?).await,
            Err(error) => {
                *player = before;
                return Err(error.into());
            }
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
                warn!(client_id = cid, id, %error, "Couldn't claim a parcel");
                *player = before;
                return self.send(&mut build_parcel_result(result::UNKNOWN)?).await;
            }
        }
        info!(self.client_id, id, "Parcel claimed");
        self.send_all(&mut packets).await
    }
}
