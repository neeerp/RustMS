use super::fame::FAME_LIMIT;
use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::DirectoryMessage};
use game::InventoryType;
use net::packet::build::world::inventory::build_inventory_operation;
use net::packet::build::world::note::{
    build_note_arrived, build_note_list, build_note_not_sent, build_note_sent, NotSent, MAX_LISTED,
};
use net::packet::build::world::stat::{build_stat_update, StatUpdate};
use net::NoteRequest;
use tokio::sync::oneshot;

/// The most notes waiting for one character before more are refused
/// ("The receiver's inbox is full."). v83 doesn't say how many; this is a
/// choice.
const INBOX_LIMIT: i64 = 20;

impl ClientActor {
    /// On entering the game: the notes waiting, if any, which the client
    /// announces.
    pub(super) async fn load_notes(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let notes = spawn_db(move || db::note::list(cid, MAX_LISTED as i64)).await?;
        if notes.is_empty() {
            return Ok(());
        }
        self.send(&mut build_note_list(&notes)?).await
    }

    pub(super) async fn note_request(&mut self, request: NoteRequest) -> Result<(), RuntimeError> {
        match request {
            NoteRequest::Send {
                slot,
                item_id,
                to,
                text,
            } => self.send_note(slot, item_id, to, text).await,
            NoteRequest::List => {
                let cid = self.client_id;
                let notes = spawn_db(move || db::note::list(cid, MAX_LISTED as i64)).await?;
                self.send(&mut build_note_list(&notes)?).await
            }
            // The inbox is out of reach in the Cash Shop.
            NoteRequest::Delete { .. } if self.in_cash_shop => Ok(()),
            NoteRequest::Delete { ids } => self.read_notes(ids).await,
            NoteRequest::ThankForGift { to, text, gift } => {
                let player = &self.in_game()?.player;
                let (account, name) = (player.account_id, player.stats.name.clone());
                let thanked =
                    spawn_db(move || db::note::thank(account, gift, &to, &name, &text)).await?;
                let Some(receiver) = thanked else {
                    return Ok(());
                };
                self.send_to_directory(DirectoryMessage::Deliver {
                    to: vec![receiver],
                    packet: build_note_arrived()?,
                })
                .await
            }
        }
    }

    /// Leave the character named `to` a note, using up the note item in
    /// Cash `slot`. v83 sends notes only to characters who are offline;
    /// online ones can be whispered.
    async fn send_note(
        &mut self,
        slot: i16,
        item_id: i32,
        to: String,
        text: String,
    ) -> Result<(), RuntimeError> {
        let receiver = spawn_db(move || db::note::character(&to)).await?;
        let Some((receiver, _)) = receiver.filter(|(id, _)| *id != self.client_id) else {
            return self
                .send(&mut build_note_not_sent(NotSent::NotFound)?)
                .await;
        };
        let (reply, online) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::Online {
            client_id: receiver,
            reply,
        })
        .await?;
        if online.await.unwrap_or(true) {
            return self.send(&mut build_note_not_sent(NotSent::Online)?).await;
        }
        let held = self
            .in_game()?
            .player
            .inventory
            .get(InventoryType::Cash, slot)
            .is_some_and(|item| item.item_id == item_id);
        if !held {
            return self.send(&mut build_stat_update(true, &[])?).await;
        }
        let sender = self.in_game()?.player.stats.name.clone();
        let sent = spawn_db(move || db::note::send(receiver, &sender, &text, INBOX_LIMIT)).await?;
        if !sent {
            return self
                .send(&mut build_note_not_sent(NotSent::InboxFull)?)
                .await;
        }
        let Ok(changes) =
            self.in_game_mut()?
                .player
                .inventory_mut()
                .take(InventoryType::Cash, slot, 1)
        else {
            return self.send(&mut build_stat_update(true, &[])?).await;
        };
        let mut packets = [
            build_inventory_operation(true, &changes)?,
            build_note_sent()?,
        ];
        self.send_all(&mut packets).await
    }

    /// The player read these notes, which are now gone; each thank-you
    /// among them raises their fame by one.
    async fn read_notes(&mut self, ids: Vec<i32>) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let flags = spawn_db(move || db::note::delete(cid, &ids)).await?;
        let thanks = flags.iter().filter(|&&flag| flag == db::note::FAME).count() as i16;
        if thanks == 0 {
            return Ok(());
        }
        let player = &mut self.in_game_mut()?.player;
        let fame = player.stats.fame.saturating_add(thanks).min(FAME_LIMIT);
        if fame == player.stats.fame {
            return Ok(());
        }
        player.stats.fame = fame;
        player.mark_dirty();
        self.send(&mut build_stat_update(false, &[StatUpdate::Fame(fame)])?)
            .await
    }
}
