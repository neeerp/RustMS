//! NX Credit earned by playing (`net::rates::nx`): kept here as it comes and
//! put on the account at each autosave, on entering the Cash Shop, and on
//! leaving, so a kill costs no write of its own.

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use db::cash_shop::CashType;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use std::time::Duration;
use tracing::warn;

pub(super) const HOUR: Duration = Duration::from_secs(3600);

impl ClientActor {
    /// A kill this player looted.
    pub(super) fn earn_nx_for_kill(&mut self) {
        self.nx_earned = self.nx_earned.saturating_add(net::rates::nx().per_kill);
    }

    /// Another hour logged in.
    pub(super) async fn earn_nx_for_hour(&mut self) -> Result<(), RuntimeError> {
        let per_hour = net::rates::nx().per_hour;
        if per_hour == 0 || self.game.is_none() {
            return Ok(());
        }
        self.nx_earned = self.nx_earned.saturating_add(per_hour);
        // The shop's stage shows no chat.
        if self.in_cash_shop {
            return Ok(());
        }
        let line = format!("You earned {per_hour} NX Credit for an hour of play.");
        self.send(&mut build_broadcast(Broadcast::Pink, &line)?)
            .await
    }

    /// Put what was earned on the account. A failure keeps it for the next
    /// try.
    pub(super) async fn bank_nx(&mut self) {
        let Some(game) = &self.game else {
            return;
        };
        let (account_id, amount) = (game.player.account_id, self.nx_earned);
        if amount == 0 {
            return;
        }
        match spawn_db(move || db::cash_shop::grant(account_id, CashType::Credit, amount)).await {
            Ok(_) => self.nx_earned -= amount,
            Err(error) => warn!(self.client_id, %error, amount, "Failed to bank earned NX"),
        }
    }
}
