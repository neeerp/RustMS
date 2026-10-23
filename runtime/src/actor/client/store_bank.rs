use super::ClientActor;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use game::hired_merchant::{HiredMerchant, MerchantState};
use game::Storage;
use net::hired_merchant::MAX_ITEMS;
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::store_bank::{
    build_store_bank, build_store_bank_elsewhere, build_store_bank_fee, build_store_bank_message,
    build_store_bank_unavailable,
};
use net::packet::build::world::{inventory, stat};
use net::personal_shop as rules;
use net::store_bank::{days_kept, message, storing_fee, StoreBankRequest, FREDRICK};
use net::trade::{self, TradeFailure};
use std::time::SystemTime;
use tracing::{info, warn};

/// Fredrick's Store Bank, while the player has its window open.
#[derive(Default)]
pub(super) struct StoreBankWindow {
    /// What Get All quoted: the merchant as it was read, the days and the
    /// fee.
    quote: Option<(Box<HiredMerchant>, i32, i32)>,
}

/// What a closed merchant (or one left in setup) keeps for its owner: its
/// unsold items and its mesos.
fn kept(merchant: &HiredMerchant) -> Option<trade::Offer> {
    if merchant.state == MerchantState::Open {
        return None;
    }
    let listings: Vec<_> = merchant.stock.iter().map(|s| s.listing.clone()).collect();
    let items = rules::unsold(&listings);
    (merchant.meso > 0 || !items.items.is_empty()).then_some(items)
}

impl ClientActor {
    /// Open the Store Bank if `npc_id` is Fredrick: the window with what
    /// the player's merchant kept, or Fredrick saying where it is open or
    /// that he keeps nothing.
    pub(super) async fn open_store_bank(&mut self, npc_id: i32) -> Result<bool, RuntimeError> {
        if npc_id != FREDRICK {
            return Ok(false);
        }
        self.store_bank = None;
        if self.hired_merchant.is_some() {
            // Their merchant is in setup with them: it isn't kept yet.
            self.send(&mut build_store_bank_unavailable()?).await?;
            return Ok(true);
        }
        let owner = self.in_game()?.player.id();
        let merchant = spawn_db(move || db::merchant::load(owner)).await?;
        let mut packet = match merchant {
            Some(m) if m.state == MerchantState::Open => {
                build_store_bank_elsewhere(npc_id, Some((m.stand.map_id, m.stand.channel_id)))?
            }
            Some(m) => match kept(&m) {
                Some(items) => {
                    let items = items.items.into_iter().map(|(_, item)| item).collect();
                    let storage = Storage::new(MAX_ITEMS, m.meso, items);
                    self.store_bank = Some(StoreBankWindow::default());
                    build_store_bank(npc_id, &storage)?
                }
                None => build_store_bank_elsewhere(npc_id, None)?,
            },
            None => build_store_bank_elsewhere(npc_id, None)?,
        };
        self.send(&mut packet).await?;
        Ok(true)
    }

    pub(super) async fn store_bank_request(
        &mut self,
        request: StoreBankRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            StoreBankRequest::Close => {
                self.store_bank = None;
                Ok(())
            }
            StoreBankRequest::GetAll => self.quote_store_bank().await,
            StoreBankRequest::Confirm => self.retrieve_from_store_bank().await,
        }
    }

    /// Get All: read what the merchant keeps again and quote the fee.
    async fn quote_store_bank(&mut self) -> Result<(), RuntimeError> {
        let Some(window) = self.store_bank.as_mut() else {
            return Ok(());
        };
        window.quote = None;
        let owner = self.in_game()?.player.id();
        let merchant = match spawn_db(move || db::merchant::load(owner)).await {
            Ok(Some(merchant)) => merchant,
            Ok(None) => return self.store_bank_failed().await,
            Err(error) => {
                warn!(self.client_id, %error, "Failed to read the Store Bank");
                return self.store_bank_failed().await;
            }
        };
        if kept(&merchant).is_none() {
            return self.store_bank_failed().await;
        }
        let worth = i64::from(merchant.meso)
            + merchant
                .stock
                .iter()
                .map(|s| i64::from(s.listing.price) * i64::from(s.listing.bundles))
                .sum::<i64>();
        let days = days_kept(merchant.closed_at, SystemTime::now());
        let fee = i32::try_from(storing_fee(worth, days)).unwrap_or(i32::MAX);
        if let Some(window) = self.store_bank.as_mut() {
            window.quote = Some((Box::new(merchant), days, fee));
        }
        self.send(&mut build_store_bank_fee(days, fee)?).await
    }

    /// Yes to the fee: everything comes to the player at once, less the
    /// fee, or nothing does.
    async fn retrieve_from_store_bank(&mut self) -> Result<(), RuntimeError> {
        let Some((merchant, days, fee)) = self.store_bank.as_mut().and_then(|w| w.quote.take())
        else {
            return Ok(());
        };
        // A day went by since the quote: the fee went up.
        let stale = days_kept(merchant.closed_at, SystemTime::now()) != days;
        let Some(items) = kept(&merchant).filter(|_| !stale) else {
            return self.store_bank_failed().await;
        };
        let player = &mut self.in_game_mut()?.player;
        let purse = i64::from(player.meso) + i64::from(merchant.meso) - i64::from(fee);
        let refused = if purse < 0 {
            Some(message::NO_FEE)
        } else if purse > i64::from(i32::MAX) {
            Some(message::TOO_MUCH_MESO)
        } else {
            match trade::check(player, &items) {
                Ok(()) => None,
                Err(TradeFailure::OnlyOne) => Some(message::ONE_OF_A_KIND),
                Err(TradeFailure::NoRoom) => Some(message::INVENTORY_FULL),
            }
        };
        if let Some(code) = refused {
            return self.send(&mut build_store_bank_message(code)?).await;
        }
        let before = player.clone();
        let given = trade::give(player, items);
        if !given.overflow.is_empty() {
            // `check` and `give` disagree: keep it all where it is.
            *player = before;
            return self.store_bank_failed().await;
        }
        player.meso = purse as i32;
        player.mark_dirty();
        let snapshot = player.clone();
        let retrieved = spawn_db(move || db::merchant::retrieve(&snapshot, &merchant)).await;
        let player = &mut self.in_game_mut()?.player;
        if !matches!(retrieved, Ok(true)) {
            *player = before;
            match retrieved {
                Err(error) => warn!(self.client_id, %error, "Failed to retrieve from Fredrick"),
                _ => info!(self.client_id, "The Store Bank changed before retrieving"),
            }
            return self.store_bank_failed().await;
        }
        player.mark_saved();
        let meso = player.meso;
        info!(
            self.client_id,
            fee, "Retrieved a merchant's items from Fredrick"
        );
        self.send(&mut build_store_bank_message(message::RETRIEVED)?)
            .await?;
        if !given.changes.is_empty() {
            self.send(&mut inventory::build_inventory_operation(
                false,
                &given.changes,
            )?)
            .await?;
        }
        self.send(&mut stat::build_stat_update(
            true,
            &[StatUpdate::Meso(meso)],
        )?)
        .await
    }

    /// The request comes to nothing: "This is currently unavailable".
    async fn store_bank_failed(&mut self) -> Result<(), RuntimeError> {
        self.send(&mut build_store_bank_unavailable()?).await
    }
}
