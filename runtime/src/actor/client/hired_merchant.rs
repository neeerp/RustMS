use super::ClientActor;
use crate::actor::field_merchants::{self, MerchantEvent, MerchantRequest, Reservation};
use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::message::{FieldKey, FieldMessage};
use game::hired_merchant::{HiredMerchant, MerchantState, Sale, Stand, Stock};
use game::InventoryType;
use net::hired_merchant::leave_reason;
use net::hired_merchant::{
    check_result, close_result, is_permit, is_remote_controller, HiredMerchantRequest,
    MAX_LEVEL_REFUSED, MAX_TITLE,
};
use net::packet::build::world::hired_merchant::{
    build_blacklist, build_check_result, build_close_result, build_merchant_items,
    build_merchant_room, build_meso_taken, build_open_elsewhere, build_organized,
    build_remote_elsewhere, build_remote_here, build_visitor_list, OwnerView,
};
use net::packet::build::world::mini_room::{
    build_enter_error, build_room_leave, buy_result, enter_error,
};
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{inventory, stat};
use net::personal_shop::{self as rules, tax};
use net::trade::{self, TradeFailure, TradeRequest};
use packet::Packet;
use std::time::{Duration, SystemTime};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

/// The player's place in a hired merchant's room (report 165). The
/// database holds the merchant; what the owner puts in or takes out is
/// written with them in one transaction.
pub(super) enum MerchantRoom {
    /// Setting up their merchant, which stands nowhere yet: nothing sells,
    /// so this copy is the merchant.
    Setup {
        merchant: Box<HiredMerchant>,
        blacklist: Vec<String>,
    },
    /// In for maintenance of their open merchant. The field let them in
    /// once nothing was being bought, and sells nothing while they are in,
    /// so this copy is the merchant until they leave.
    Maintenance(Box<HiredMerchant>),
    Visitor {
        owner: i32,
        slot: u8,
    },
}

impl MerchantRoom {
    fn owner(&self) -> i32 {
        match self {
            Self::Setup { merchant, .. } | Self::Maintenance(merchant) => merchant.owner_id,
            Self::Visitor { owner, .. } => *owner,
        }
    }

    /// The player's seat in the room: the owner's is 0.
    fn slot(&self) -> u8 {
        match self {
            Self::Visitor { slot, .. } => *slot,
            _ => 0,
        }
    }
}

impl ClientActor {
    fn in_a_room(&self) -> bool {
        self.trade.is_some() || self.personal_shop.is_some() || self.hired_merchant.is_some()
    }

    pub(super) async fn hired_merchant_request(
        &mut self,
        request: HiredMerchantRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            HiredMerchantRequest::Check => self.check_merchant().await,
            HiredMerchantRequest::Remote { slot } => self.use_remote_controller(slot).await,
            HiredMerchantRequest::RemoteVisit { sn, slot, cash_sn } => {
                self.remote_visit(sn, slot, cash_sn).await
            }
            HiredMerchantRequest::Create {
                title,
                permit_slot,
                permit_id,
            } => self.create_merchant(title, permit_slot, permit_id).await,
            HiredMerchantRequest::PutItem {
                kind,
                slot,
                bundles,
                per_bundle,
                price,
            } => {
                self.put_in_merchant(kind, slot, bundles, per_bundle, price)
                    .await
            }
            HiredMerchantRequest::TakeBack { index } => self.take_from_merchant(index).await,
            HiredMerchantRequest::Buy { index, bundles } => {
                self.buy_from_merchant(index, bundles).await
            }
            HiredMerchantRequest::EndMaintenance => {
                if !matches!(self.hired_merchant, Some(MerchantRoom::Maintenance(_))) {
                    return Ok(());
                }
                self.hired_merchant = None;
                self.to_merchant(MerchantRequest::Reopen).await
            }
            HiredMerchantRequest::Organize => self.organize_merchant().await,
            HiredMerchantRequest::Close => self.close_merchant(true).await,
            HiredMerchantRequest::TakeMeso => self.take_merchant_meso().await,
            HiredMerchantRequest::Visitors => match &self.hired_merchant {
                Some(MerchantRoom::Setup { .. }) => self.send(&mut build_visitor_list(&[])?).await,
                Some(MerchantRoom::Maintenance(_)) => {
                    self.to_merchant(MerchantRequest::Visitors).await
                }
                _ => Ok(()),
            },
            HiredMerchantRequest::Blacklist => match &self.hired_merchant {
                Some(MerchantRoom::Setup { blacklist, .. }) => {
                    let mut packet = build_blacklist(blacklist)?;
                    self.send(&mut packet).await
                }
                Some(MerchantRoom::Maintenance(_)) => {
                    self.to_merchant(MerchantRequest::Blacklist).await
                }
                _ => Ok(()),
            },
            HiredMerchantRequest::BlacklistAdd(name) => match &mut self.hired_merchant {
                Some(MerchantRoom::Setup { blacklist, .. }) => {
                    if !blacklist.iter().any(|b| b.eq_ignore_ascii_case(&name)) {
                        blacklist.push(name);
                    }
                    Ok(())
                }
                Some(MerchantRoom::Maintenance(_)) => {
                    self.to_merchant(MerchantRequest::BlacklistAdd(name)).await
                }
                _ => Ok(()),
            },
            HiredMerchantRequest::BlacklistRemove(name) => match &mut self.hired_merchant {
                Some(MerchantRoom::Setup { blacklist, .. }) => {
                    blacklist.retain(|b| !b.eq_ignore_ascii_case(&name));
                    Ok(())
                }
                Some(MerchantRoom::Maintenance(_)) => {
                    self.to_merchant(MerchantRequest::BlacklistRemove(name))
                        .await
                }
                _ => Ok(()),
            },
        }
    }

    /// The personal shop's requests that a merchant's room shares: the
    /// blacklist the owner's client sends before opening, and opening.
    pub(super) async fn merchant_shop_request(
        &mut self,
        request: net::personal_shop::PersonalShopRequest,
    ) -> Result<(), RuntimeError> {
        use net::personal_shop::PersonalShopRequest;
        match request {
            PersonalShopRequest::Blacklist(names) => {
                if let Some(MerchantRoom::Setup { blacklist, .. }) = &mut self.hired_merchant {
                    *blacklist = names;
                }
                Ok(())
            }
            PersonalShopRequest::Open => {
                if let Err(error) = self.open_merchant().await {
                    // Still in setup: the client leaves the room next,
                    // which tries again.
                    warn!(self.client_id, %error, "Failed to open a merchant");
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Chatting and leaving, shared with the trading room, from a player in
    /// a merchant's room.
    pub(super) async fn merchant_room_request(
        &mut self,
        request: TradeRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            TradeRequest::Chat { text } => match self.hired_merchant {
                Some(MerchantRoom::Setup { .. }) | None => Ok(()),
                Some(_) => self.to_merchant(MerchantRequest::Chat { text }).await,
            },
            // The owner's client closes a merchant with nothing left with
            // this, not 0x29 (report 165 §2).
            TradeRequest::Leave => match &self.hired_merchant {
                Some(MerchantRoom::Maintenance(merchant))
                    if merchant.meso == 0
                        && merchant.stock.iter().all(|s| s.listing.bundles == 0) =>
                {
                    self.close_merchant(false).await
                }
                _ => self.leave_hired_merchant().await,
            },
            _ => Ok(()),
        }
    }

    /// The player used a permit: whether they may hire a merchant here.
    async fn check_merchant(&mut self) -> Result<(), RuntimeError> {
        let mut packet = match self.merchant_stand(None, false).await? {
            Ok(_) => build_check_result(check_result::OK)?,
            Err(refusal) => refusal,
        };
        self.send(&mut packet).await
    }

    /// Where the player's merchant would stand, or the answer that says
    /// why they can't hire one. `permit` is the slot and item the client
    /// names, if it does. `hold`: the field keeps the spot for them.
    async fn merchant_stand(
        &mut self,
        permit: Option<(i16, i32)>,
        hold: bool,
    ) -> Result<Result<Stand, Packet>, RuntimeError> {
        let unavailable = || build_check_result(check_result::UNAVAILABLE);
        if self.in_a_room() || self.merchant_field.is_some() {
            return Ok(Err(unavailable()?));
        }
        let player = &self.in_game()?.player;
        let has_permit = match permit {
            Some((slot, item_id)) => player
                .inventory
                .get(InventoryType::Cash, slot)
                .is_some_and(|item| item.item_id == item_id && is_permit(item_id)),
            None => player
                .inventory
                .items(InventoryType::Cash)
                .any(|(_, item)| is_permit(item.item_id)),
        };
        if !has_permit || player.stats.level <= MAX_LEVEL_REFUSED {
            return Ok(Err(unavailable()?));
        }
        let owner = player.id();
        if let Some(placed) = spawn_db(move || db::merchant::find(owner)).await? {
            // One left in setup by a crash goes, if it is empty.
            let gone = placed.state == MerchantState::Setup
                && spawn_db(move || db::merchant::abandon(owner)).await?;
            if !gone {
                return Ok(Err(match placed.state {
                    MerchantState::Open => build_open_elsewhere(placed.map_id, placed.channel_id)?,
                    MerchantState::Setup | MerchantState::Closed => {
                        build_check_result(check_result::AT_FREDRICK)?
                    }
                }));
            }
        }
        let (reply, stand) = oneshot::channel();
        self.to_merchant(MerchantRequest::Stand { reply, hold })
            .await?;
        match stand.await.ok().flatten() {
            Some(stand) => Ok(Ok(stand)),
            None => Ok(Err(unavailable()?)),
        }
    }

    /// The serial of the Store Remote Controller in cash slot `slot`, and
    /// whether its time is up.
    fn remote_controller(&self, slot: i16) -> Result<Option<(i64, bool)>, RuntimeError> {
        let player = &self.in_game()?.player;
        Ok(player
            .inventory
            .get(InventoryType::Cash, slot)
            .filter(|item| is_remote_controller(item.item_id))
            .map(|item| {
                let expired = item.expiration.is_some_and(|e| e <= SystemTime::now());
                (item.cash_serial.unwrap_or(0), expired)
            }))
    }

    /// The player used a Store Remote Controller: the client asks to come
    /// into their merchant open on this channel, or offers to go to its
    /// channel (report b088 §2). The item stays.
    async fn use_remote_controller(&mut self, slot: i16) -> Result<(), RuntimeError> {
        if self.in_a_room() || self.merchant_field.is_some() {
            return Ok(());
        }
        let Some((cash_sn, expired)) = self.remote_controller(slot)? else {
            return Ok(());
        };
        if expired {
            return self
                .send(&mut build_enter_error(enter_error::EXPIRED)?)
                .await;
        }
        let game = self.in_game()?;
        let (owner, channel) = (game.player.id(), game.field.key.channel_id);
        let placed = spawn_db(move || db::merchant::find(owner)).await?;
        let mut packet = match placed.filter(|p| p.state == MerchantState::Open) {
            Some(p) if p.channel_id == channel => {
                build_remote_here(field_merchants::sn(owner), slot, cash_sn)?
            }
            Some(p) => build_remote_elsewhere(p.map_id, p.channel_id)?,
            None => build_remote_elsewhere(0, check_result::NO_CHANNEL)?,
        };
        self.send(&mut packet).await
    }

    /// The owner, sure with their birthday, comes into their merchant `sn`
    /// from wherever they are on its channel, as for maintenance there.
    async fn remote_visit(&mut self, sn: i32, slot: i16, cash_sn: i64) -> Result<(), RuntimeError> {
        if self.in_a_room() || self.merchant_field.is_some() {
            return Ok(());
        }
        match self.remote_controller(slot)? {
            Some((serial, false)) if serial == cash_sn => {}
            Some((serial, true)) if serial == cash_sn => {
                return self
                    .send(&mut build_enter_error(enter_error::EXPIRED)?)
                    .await;
            }
            _ => {
                debug!(self.client_id, slot, "Remote visit without the controller");
                return Ok(());
            }
        }
        let game = self.in_game()?;
        let owner = game.player.id();
        let channel = game.field.key.channel_id;
        let open_here =
            |p: &db::merchant::Placement| p.state == MerchantState::Open && p.channel_id == channel;
        let placed = spawn_db(move || db::merchant::find(owner)).await?;
        let Some(map_id) = placed
            .filter(|p| open_here(p) && field_merchants::owner_of(sn) == Some(owner))
            .map(|p| p.map_id)
        else {
            return self
                .send(&mut build_enter_error(enter_error::CLOSED)?)
                .await;
        };
        let player = &self.in_game()?.player;
        let (name, look) = (player.stats.name.clone(), player.look());
        self.merchant_visits.push(owner);
        let field = self.services.fields.field(FieldKey::new(channel, map_id));
        self.merchant_field = Some(field);
        let handle = self.handle();
        self.to_merchant(MerchantRequest::RemoteVisit {
            sn,
            character_id: owner,
            name,
            look,
            handle,
        })
        .await
    }

    /// A map change ends a remote visit, before the client leaves the
    /// map, and the client says so (SP 0xd8d, report b088 §5).
    pub(super) async fn stop_remote_merchant(&mut self) -> Result<(), RuntimeError> {
        if self.merchant_field.is_none() {
            return Ok(());
        }
        if let Some(room) = &self.hired_merchant {
            let mut packet = build_room_leave(room.slot(), leave_reason::REMOTE_MOVED)?;
            self.send(&mut packet).await?;
        }
        self.leave_hired_merchant().await
    }

    /// Set the merchant up where the player stands, empty, and seat them in
    /// its room to list what it will sell. Nobody sees it until it opens.
    async fn create_merchant(
        &mut self,
        title: String,
        permit_slot: i16,
        permit_id: i32,
    ) -> Result<(), RuntimeError> {
        let length = title.chars().count();
        if length == 0 || length > MAX_TITLE {
            return self
                .send(&mut build_check_result(check_result::UNAVAILABLE)?)
                .await;
        }
        let stand = match self
            .merchant_stand(Some((permit_slot, permit_id)), true)
            .await?
        {
            Ok(stand) => stand,
            Err(mut refusal) => return self.send(&mut refusal).await,
        };
        let player = &self.in_game()?.player;
        let (owner, owner_name) = (player.id(), player.stats.name.clone());
        let created = {
            let (stand, title) = (stand.clone(), title.clone());
            spawn_db(move || db::merchant::create(owner, &stand, permit_id, &title)).await?
        };
        if !created {
            // Let the spot go.
            self.to_merchant(MerchantRequest::Leave).await?;
            return self
                .send(&mut build_check_result(check_result::UNAVAILABLE)?)
                .await;
        }
        let merchant = HiredMerchant {
            owner_id: owner,
            owner_name,
            stand,
            item_id: permit_id,
            title,
            state: MerchantState::Setup,
            opened_at: None,
            closed_at: None,
            meso: 0,
            stock: Vec::new(),
            sales: Vec::new(),
        };
        info!(self.client_id, owner, permit_id, "Hired merchant set up");
        let view = OwnerView {
            open_for: Duration::ZERO,
            setup: true,
        };
        let mut room = build_merchant_room(&merchant, 0, Some(view), &[], &[])?;
        self.hired_merchant = Some(MerchantRoom::Setup {
            merchant: Box::new(merchant),
            blacklist: Vec::new(),
        });
        self.send(&mut room).await
    }

    /// The owner's merchant, in setup or maintenance.
    fn own_merchant(&mut self) -> Option<&mut HiredMerchant> {
        match &mut self.hired_merchant {
            Some(MerchantRoom::Setup { merchant, .. } | MerchantRoom::Maintenance(merchant)) => {
                Some(merchant)
            }
            _ => None,
        }
    }

    /// Move an item from the inventory into the merchant, in setup or in
    /// maintenance (the client lets one add then too).
    async fn put_in_merchant(
        &mut self,
        kind: i16,
        slot: i16,
        bundles: i16,
        per_bundle: i16,
        price: i32,
    ) -> Result<(), RuntimeError> {
        let Some(merchant) = self.own_merchant() else {
            return self.unlock().await;
        };
        let listings: Vec<_> = merchant.stock.iter().map(|s| s.listing.clone()).collect();
        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let (listing, changes) =
            match rules::list_item(player, &listings, kind, slot, bundles, per_bundle, price) {
                Ok(listed) => listed,
                Err(why) => {
                    debug!(self.client_id, kind, slot, why, "Merchant listing refused");
                    return self.unlock().await;
                }
            };
        let snapshot = player.clone();
        let written = {
            let listing = listing.clone();
            spawn_db(move || db::merchant::put(&snapshot, &listing)).await
        };
        let player = &mut self.in_game_mut()?.player;
        let id = match written {
            Ok(id) => {
                player.mark_saved();
                id
            }
            Err(error) => {
                *player = before;
                warn!(self.client_id, %error, "Failed to list in a merchant");
                return self.unlock().await;
            }
        };
        let merchant = self.own_merchant().expect("checked above");
        merchant.stock.push(Stock { id, listing });
        self.send(&mut inventory::build_inventory_operation(true, &changes)?)
            .await?;
        self.merchant_changed().await
    }

    /// Take a row back into the inventory.
    async fn take_from_merchant(&mut self, index: i16) -> Result<(), RuntimeError> {
        let Some(merchant) = self.own_merchant() else {
            return self.unlock().await;
        };
        let Some(index) = usize::try_from(index)
            .ok()
            .filter(|&i| merchant.stock.get(i).is_some_and(|s| s.listing.bundles > 0))
        else {
            return self.unlock().await;
        };
        let row = merchant.stock[index].clone();
        let items = rules::unsold(std::slice::from_ref(&row.listing));
        let player = &mut self.in_game_mut()?.player;
        if trade::check(player, &items).is_err() {
            self.send(&mut inventory::build_inventory_full()?).await?;
            return self.unlock().await;
        }
        let before = player.clone();
        let given = trade::give(player, items);
        let snapshot = player.clone();
        let bundles = row.listing.bundles;
        let taken = spawn_db(move || db::merchant::take_back(&snapshot, row.id, bundles)).await;
        let player = &mut self.in_game_mut()?.player;
        if !given.overflow.is_empty() || !matches!(taken, Ok(true)) {
            *player = before;
            if let Err(error) = taken {
                warn!(self.client_id, %error, "Failed to take back from a merchant");
            }
            return self.unlock().await;
        }
        player.mark_saved();
        self.own_merchant()
            .expect("checked above")
            .stock
            .remove(index);
        self.send(&mut inventory::build_inventory_operation(
            true,
            &given.changes,
        )?)
        .await?;
        self.merchant_changed().await
    }

    /// Show the owner what their merchant holds now, and tell the field if
    /// it stands there.
    async fn merchant_changed(&mut self) -> Result<(), RuntimeError> {
        let Some(merchant) = self.own_merchant() else {
            return Ok(());
        };
        let listings: Vec<_> = merchant.stock.iter().map(|s| &s.listing).collect();
        let mut items = build_merchant_items(merchant.meso, &listings)?;
        let restock = MerchantRequest::Restock {
            stock: merchant.stock.clone(),
            meso: merchant.meso,
        };
        self.send(&mut items).await?;
        if matches!(self.hired_merchant, Some(MerchantRoom::Maintenance(_))) {
            self.to_merchant(restock).await?;
        }
        Ok(())
    }

    /// Open the merchant set up with something to sell: the owner goes, and
    /// it stands and sells on its own for a day. The permit stays (the
    /// questions log, 2026-10-22).
    async fn open_merchant(&mut self) -> Result<(), RuntimeError> {
        let Some(MerchantRoom::Setup { merchant, .. }) = &self.hired_merchant else {
            return Ok(());
        };
        if merchant.stock.is_empty() {
            return Ok(());
        }
        let Some(MerchantRoom::Setup {
            mut merchant,
            blacklist,
        }) = self.hired_merchant.take()
        else {
            unreachable!("matched above");
        };
        let now = SystemTime::now();
        let snapshot = self.in_game()?.player.clone();
        match spawn_db(move || db::merchant::open(&snapshot, now)).await {
            Ok(true) => {}
            Ok(false) => {
                warn!(self.client_id, "Merchant gone before it opened");
                return self.to_merchant(MerchantRequest::Leave).await;
            }
            Err(error) => {
                // Still in setup; the next try or leaving opens it.
                self.hired_merchant = Some(MerchantRoom::Setup {
                    merchant,
                    blacklist,
                });
                return Err(error);
            }
        }
        self.in_game_mut()?.player.mark_saved();
        merchant.state = MerchantState::Open;
        merchant.opened_at = Some(now);
        info!(
            self.client_id,
            rows = merchant.stock.len(),
            "Hired merchant opened"
        );
        self.to_merchant(MerchantRequest::Place {
            merchant,
            blacklist,
        })
        .await
    }

    /// Strike off the sold-out rows and take the mesos, if they fit.
    async fn organize_merchant(&mut self) -> Result<(), RuntimeError> {
        let Some(merchant) = self.own_merchant() else {
            return Ok(());
        };
        let owner = merchant.owner_id;
        if merchant.meso > 0 {
            self.withdraw_merchant_meso().await?;
        }
        if let Err(error) = spawn_db(move || db::merchant::organize(owner)).await {
            warn!(self.client_id, %error, "Failed to organize a merchant");
            return self.unlock().await;
        }
        let Some(merchant) = self.own_merchant() else {
            return Ok(());
        };
        merchant.stock.retain(|s| s.listing.bundles > 0);
        let mut organized = build_organized(merchant.meso)?;
        let restock = MerchantRequest::Restock {
            stock: merchant.stock.clone(),
            meso: merchant.meso,
        };
        self.send(&mut organized).await?;
        if matches!(self.hired_merchant, Some(MerchantRoom::Maintenance(_))) {
            self.to_merchant(restock).await?;
        }
        Ok(())
    }

    async fn take_merchant_meso(&mut self) -> Result<(), RuntimeError> {
        if !self.own_merchant().is_some_and(|m| m.meso > 0) {
            return self.unlock().await;
        }
        if !self.withdraw_merchant_meso().await? {
            return self.unlock().await;
        }
        self.send(&mut build_meso_taken()?).await?;
        if let Some(MerchantRoom::Maintenance(merchant)) = &self.hired_merchant {
            let restock = MerchantRequest::Restock {
                stock: merchant.stock.clone(),
                meso: 0,
            };
            self.to_merchant(restock).await?;
        }
        Ok(())
    }

    /// Move the merchant's mesos into the owner's purse, if they fit.
    /// Whether they did.
    async fn withdraw_merchant_meso(&mut self) -> Result<bool, RuntimeError> {
        let meso = self.own_merchant().map_or(0, |m| m.meso);
        let player = &mut self.in_game_mut()?.player;
        if meso <= 0 || player.meso.checked_add(meso).is_none() {
            return Ok(false);
        }
        let before = player.clone();
        player.meso += meso;
        player.mark_dirty();
        let snapshot = player.clone();
        let taken = spawn_db(move || db::merchant::take_meso(&snapshot, meso)).await;
        let player = &mut self.in_game_mut()?.player;
        if !matches!(taken, Ok(true)) {
            *player = before;
            if let Err(error) = taken {
                warn!(self.client_id, %error, "Failed to take a merchant's mesos");
            }
            return Ok(false);
        }
        player.mark_saved();
        let purse = player.meso;
        if let Some(merchant) = self.own_merchant() {
            merchant.meso = 0;
        }
        self.send(&mut stat::build_stat_update(
            false,
            &[StatUpdate::Meso(purse)],
        )?)
        .await?;
        Ok(true)
    }

    /// Close the merchant from its room and take back what it holds, as
    /// much as fits; the rest waits with Fredrick. `notify`: the client
    /// waits in the room for the result; it doesn't when the merchant was
    /// empty.
    async fn close_merchant(&mut self, notify: bool) -> Result<(), RuntimeError> {
        let mut blacklist = Vec::new();
        let merchant = match &self.hired_merchant {
            Some(MerchantRoom::Setup { merchant, .. }) => merchant.clone(),
            Some(MerchantRoom::Maintenance(_)) => {
                let (reply, closed) = oneshot::channel();
                self.to_merchant(MerchantRequest::Close { reply }).await?;
                let Ok((_, names)) = closed.await else {
                    // The field won't let it go yet. A client that left
                    // the room isn't waiting: the merchant opens again.
                    if !notify {
                        return self.leave_hired_merchant().await;
                    }
                    return self.unlock().await;
                };
                blacklist = names;
                let owner = self.in_game()?.player.id();
                match spawn_db(move || db::merchant::load(owner)).await {
                    Ok(Some(merchant)) => Box::new(merchant),
                    Ok(None) => {
                        self.hired_merchant = None;
                        self.merchant_field = None;
                        if !notify {
                            return Ok(());
                        }
                        return self
                            .send(&mut build_close_result(close_result::RETRIEVED)?)
                            .await;
                    }
                    Err(error) => {
                        // It still stands open in the database; put it back.
                        warn!(self.client_id, %error, "Failed to read a merchant to close");
                        self.replace_merchant(blacklist).await?;
                        return self.unlock().await;
                    }
                }
            }
            _ => return Ok(()),
        };
        let items = rules::unsold(
            &merchant
                .stock
                .iter()
                .map(|s| s.listing.clone())
                .collect::<Vec<_>>(),
        );
        let player = &mut self.in_game_mut()?.player;
        let meso_fits = player.meso.checked_add(merchant.meso).is_some();
        let (code, retrieved) = if !meso_fits {
            (
                close_result::TOO_MUCH_MESO,
                db::merchant::Retrieved {
                    items: false,
                    meso: false,
                },
            )
        } else {
            let (code, items) = match trade::check(player, &items) {
                Ok(()) => (close_result::RETRIEVED, true),
                Err(TradeFailure::OnlyOne) => (close_result::ONE_OF_A_KIND, false),
                Err(TradeFailure::NoRoom) => (close_result::INVENTORY_FULL, false),
            };
            (code, db::merchant::Retrieved { items, meso: true })
        };
        let before = player.clone();
        let mut given = trade::Given::default();
        if retrieved.items {
            given = trade::give(player, items);
        }
        if !given.overflow.is_empty() {
            // `check` and `give` disagree: keep it all where it is.
            *player = before;
            self.replace_merchant(blacklist).await?;
            return self.unlock().await;
        }
        if retrieved.meso && merchant.meso > 0 {
            player.meso += merchant.meso;
            player.mark_dirty();
        }
        let snapshot = player.clone();
        let now = SystemTime::now();
        let closed =
            spawn_db(move || db::merchant::close(&snapshot, &merchant, retrieved, now)).await;
        let player = &mut self.in_game_mut()?.player;
        if !matches!(closed, Ok(true)) {
            // Nothing changed hands; in setup, the owner stays at it.
            *player = before;
            match closed {
                Err(error) => warn!(self.client_id, %error, "Failed to close a merchant"),
                _ => info!(self.client_id, "Merchant sold something as it closed"),
            }
            self.replace_merchant(blacklist).await?;
            return self.unlock().await;
        }
        player.mark_saved();
        let purse = player.meso;
        self.hired_merchant = None;
        info!(self.client_id, code, "Hired merchant closed");
        // A spot kept for a merchant in setup goes.
        self.to_merchant(MerchantRequest::Leave).await?;
        if notify {
            self.send(&mut build_close_result(code)?).await?;
        }
        if !given.changes.is_empty() {
            self.send(&mut inventory::build_inventory_operation(
                false,
                &given.changes,
            )?)
            .await?;
        }
        self.send(&mut stat::build_stat_update(
            true,
            &[StatUpdate::Meso(purse)],
        )?)
        .await
    }

    /// The close failed after the field let the merchant go: it stands
    /// again, as the owner left it, and they are out of its room.
    /// One in setup stands nowhere, and the owner stays at it.
    async fn replace_merchant(&mut self, blacklist: Vec<String>) -> Result<(), RuntimeError> {
        if !matches!(self.hired_merchant, Some(MerchantRoom::Maintenance(_))) {
            return Ok(());
        }
        let Some(MerchantRoom::Maintenance(merchant)) = self.hired_merchant.take() else {
            return Ok(());
        };
        self.to_merchant(MerchantRequest::Place {
            merchant,
            blacklist,
        })
        .await?;
        self.send(&mut build_room_leave(0, leave_reason::CLOSED)?)
            .await
    }

    /// Buy from the merchant the player visits: the field holds the
    /// bundles, and the purchase is written here, with the buyer.
    async fn buy_from_merchant(&mut self, index: u8, bundles: i16) -> Result<(), RuntimeError> {
        let Some(MerchantRoom::Visitor { owner, .. }) = self.hired_merchant else {
            return self.unlock().await;
        };
        let (reply, reserved) = oneshot::channel();
        self.to_merchant(MerchantRequest::Buy {
            index,
            bundles,
            reply,
        })
        .await?;
        let reservation = match reserved.await {
            Ok(Ok(reservation)) => reservation,
            Ok(Err(result)) => return self.refuse_purchase(result).await,
            Err(_) => return self.refuse_purchase(buy_result::NO_STOCK).await,
        };
        let player = &mut self.in_game_mut()?.player;
        let paid = match rules::check_purchase(player, &reservation.listing, bundles) {
            Ok(paid) => paid,
            Err(result) => {
                self.settle(&reservation, None, false).await?;
                return self.refuse_purchase(result).await;
            }
        };
        let before = player.clone();
        player.meso -= paid;
        player.mark_dirty();
        let mut item = reservation.listing.take(bundles);
        // Buying uses up the item's Karma (FUN_006fd261).
        item.use_karma();
        let sale = Sale {
            item_id: item.item_id,
            quantity: item.quantity,
            mesos: paid,
            buyer: player.stats.name.clone(),
        };
        let given = trade::give(player, rules::goods(vec![item]));
        if !given.overflow.is_empty() {
            // `check_purchase` and `give` disagree: nothing is written.
            *player = before;
            self.settle(&reservation, None, false).await?;
            return self.refuse_purchase(buy_result::NO_STOCK).await;
        }
        let snapshot = player.clone();
        let (stock, written_sale) = (reservation.stock, sale.clone());
        let now = SystemTime::now();
        let sold = spawn_db(move || {
            let purchase = db::merchant::Purchase {
                owner,
                stock,
                bundles,
                net: paid - tax(paid),
                sale: &written_sale,
            };
            db::merchant::sell(&purchase, &snapshot, now)
        })
        .await;
        let player = &mut self.in_game_mut()?.player;
        if !matches!(sold, Ok(true)) {
            *player = before;
            if let Err(error) = &sold {
                warn!(self.client_id, %error, "Failed to write a merchant purchase");
            }
            // Refused, the merchant differs from the field's copy.
            let refused = matches!(sold, Ok(false));
            self.settle(&reservation, None, refused).await?;
            return self.refuse_purchase(buy_result::NO_STOCK).await;
        }
        player.mark_saved();
        let meso = player.meso;
        info!(
            self.client_id,
            owner,
            item_id = sale.item_id,
            bundles,
            paid,
            "Merchant sale"
        );
        self.settle(&reservation, Some(sale), false).await?;
        self.send(&mut inventory::build_inventory_operation(
            false,
            &given.changes,
        )?)
        .await?;
        self.send(&mut stat::build_stat_update(
            true,
            &[StatUpdate::Meso(meso)],
        )?)
        .await
    }

    async fn settle(
        &mut self,
        reservation: &Reservation,
        sale: Option<Sale>,
        refused: bool,
    ) -> Result<(), RuntimeError> {
        self.to_merchant(MerchantRequest::Settle {
            owner: reservation.owner,
            id: reservation.id,
            sale,
            refused,
        })
        .await
    }

    /// News from a merchant's room.
    pub(super) async fn merchant_event(
        &mut self,
        owner: i32,
        event: MerchantEvent,
    ) -> Result<(), RuntimeError> {
        let (room, entered) = match event {
            MerchantEvent::Entered { slot, room } => (MerchantRoom::Visitor { owner, slot }, room),
            MerchantEvent::Maintenance(merchant, room) => {
                (MerchantRoom::Maintenance(merchant), room)
            }
            MerchantEvent::Left(packet) => {
                if self.hired_merchant.as_ref().map(MerchantRoom::owner) == Some(owner) {
                    self.hired_merchant = None;
                }
                if self.hired_merchant.is_none() {
                    self.merchant_field = None;
                }
                if let Some(mut packet) = packet {
                    self.send(&mut packet).await?;
                }
                return Ok(());
            }
            MerchantEvent::Refreshed(fresh) => {
                if let Some(MerchantRoom::Maintenance(merchant)) = &mut self.hired_merchant {
                    if merchant.owner_id == owner {
                        *merchant = fresh;
                    }
                }
                return Ok(());
            }
        };
        // Let in only where the player still asked to come, from this
        // field and in no other room; a seat they no longer want goes.
        let asked = self.merchant_visits.iter().position(|&o| o == owner);
        if let Some(at) = asked {
            self.merchant_visits.swap_remove(at);
        }
        if asked.is_none() || self.in_a_room() {
            if self.hired_merchant.as_ref().map(MerchantRoom::owner) != Some(owner) {
                // An owner's leaving opens the merchant again.
                self.to_merchant(MerchantRequest::Unseat { owner }).await?;
            }
            return Ok(());
        }
        self.hired_merchant = Some(room);
        match entered {
            Some(mut packet) => self.send(&mut packet).await,
            None => Ok(()),
        }
    }

    /// Leave the merchant's room, if the player is in one. A merchant set
    /// up with something to sell opens; an empty one goes. Every way off
    /// the field comes through here first.
    pub(super) async fn leave_hired_merchant(&mut self) -> Result<(), RuntimeError> {
        self.merchant_visits.clear();
        match &self.hired_merchant {
            // Still waiting to come in from elsewhere.
            None if self.merchant_field.is_some() => self.to_merchant(MerchantRequest::Leave).await,
            None => Ok(()),
            Some(MerchantRoom::Setup { merchant, .. }) if !merchant.stock.is_empty() => {
                if let Err(error) = self.open_merchant().await {
                    // It stays in setup in the database, for Fredrick.
                    warn!(self.client_id, %error, "Failed to open a merchant on leaving");
                    self.hired_merchant = None;
                    self.to_merchant(MerchantRequest::Leave).await?;
                }
                Ok(())
            }
            Some(MerchantRoom::Setup { merchant, .. }) => {
                let owner = merchant.owner_id;
                self.hired_merchant = None;
                // Its spot goes first: the database can fail.
                self.to_merchant(MerchantRequest::Leave).await?;
                spawn_db(move || db::merchant::abandon(owner)).await?;
                Ok(())
            }
            Some(MerchantRoom::Maintenance(_) | MerchantRoom::Visitor { .. }) => {
                self.hired_merchant = None;
                self.to_merchant(MerchantRequest::Leave).await
            }
        }
    }

    /// The player's field died with the merchant they were in. One in
    /// setup stands nowhere yet and stays; the new field reads an open one
    /// back.
    pub(super) async fn abandon_hired_merchant(&mut self) -> Result<(), RuntimeError> {
        self.merchant_visits.clear();
        self.merchant_field = None;
        let slot = match &self.hired_merchant {
            None | Some(MerchantRoom::Setup { .. }) => return Ok(()),
            Some(room) => room.slot(),
        };
        self.hired_merchant = None;
        self.send(&mut build_room_leave(slot, leave_reason::CLOSED)?)
            .await
    }

    /// Send to the merchant's field: the player's own, or the one they
    /// came into from elsewhere, which they are done with once out of its
    /// room.
    async fn to_merchant(&mut self, request: MerchantRequest) -> Result<(), RuntimeError> {
        let remote = matches!(request, MerchantRequest::RemoteVisit { .. });
        let message = FieldMessage::Merchant {
            from: self.client_id,
            request,
        };
        let Some(tx) = self.merchant_field.as_ref().map(|f| f.tx.clone()) else {
            return self.send_to_field(message).await;
        };
        if tx.send(message).await.is_err() {
            warn!(
                self.client_id,
                "Merchant's field gone during a remote visit"
            );
            return self.abandon_hired_merchant().await;
        }
        if self.hired_merchant.is_none() && !remote {
            self.merchant_field = None;
        }
        Ok(())
    }
}
