//! A GM's `!mute` (row 205 S6): the account's chat block. The client learns
//! of it at its next login (`build_successful_login_packet`) and refuses to
//! chat itself; till then the world drops what the player says
//! (`net::chat_block::is_chat`) and answers as the client would.

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use net::chat_block::BLOCKED;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::stat::build_stat_update;
use std::time::SystemTime;

impl ClientActor {
    /// Whether the account may not chat now.
    pub(super) fn chat_blocked(&self) -> bool {
        self.chat_blocked_until
            .is_some_and(|until| until > SystemTime::now())
    }

    /// Drop a line the player tried to say. An item's use leaves the client
    /// waiting, so it is let act again.
    pub(super) async fn refuse_chat(&mut self) -> Result<(), RuntimeError> {
        self.send_all(&mut [
            build_broadcast(Broadcast::Pink, BLOCKED)?,
            build_stat_update(true, &[])?,
        ])
        .await
    }

    /// A GM blocked the account from chatting or lifted it: read it again.
    pub(super) async fn chat_block_changed(&mut self) -> Result<(), RuntimeError> {
        let Some(account_id) = self.game.as_ref().map(|game| game.player.account_id) else {
            return Ok(());
        };
        let account = spawn_db(move || db::account::get_account_by_id(account_id)).await?;
        let now = SystemTime::now();
        let until = account.chat_blocked_at(now).map(|(_, until)| until);
        self.chat_blocked_until = until;
        if self.in_cash_shop {
            return Ok(());
        }
        let line = match until {
            Some(until) => {
                let minutes = until
                    .duration_since(now)
                    .unwrap_or_default()
                    .as_secs()
                    .div_ceil(60);
                format!("A GM blocked you from chatting for {minutes} minutes.")
            }
            // A client that logged in blocked keeps refusing until it logs
            // in again.
            None => "A GM let you chat again. If you still can't, log in again.".to_string(),
        };
        self.send(&mut build_broadcast(Broadcast::Pink, &line)?)
            .await
    }
}
