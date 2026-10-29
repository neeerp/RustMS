//! Cash Inventory items whose time runs out while the player is in the Cash
//! Shop: they go, and the client names each in a notice.

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use db::cash_shop::CashItem;
use net::packet::build::world::cash_shop::build_cash_item_expired;
use std::time::SystemTime;
use tokio::time::Instant;

impl ClientActor {
    /// The client now lists `items` in the Cash Inventory.
    pub(super) fn listed_in_locker(&mut self, items: &[CashItem]) {
        for item in items {
            if let Some(at) = item.expires_at {
                self.locker_expiry.insert(item.id, at);
            }
        }
    }

    /// When the next listed Cash Inventory item expires.
    pub(super) fn locker_due(&self) -> Option<Instant> {
        let next = *self.locker_expiry.values().min()?;
        let wait = next.duration_since(SystemTime::now()).unwrap_or_default();
        Some(Instant::now() + wait)
    }

    /// Take the listed items whose time ran out, telling the client.
    pub(super) async fn expire_locker(&mut self) -> Result<(), RuntimeError> {
        let now = SystemTime::now();
        let due: Vec<i64> = self
            .locker_expiry
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(id, _)| *id)
            .collect();
        self.locker_expiry.retain(|_, at| *at > now);
        let account_id = self.in_game()?.player.account_id;
        let gone = spawn_db(move || db::cash_shop::expire(account_id, &due, now)).await?;
        let mut packets = gone
            .into_iter()
            .map(build_cash_item_expired)
            .collect::<Result<Vec<_>, _>>()?;
        self.send_all(&mut packets).await
    }
}
