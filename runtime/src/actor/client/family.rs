//! Families: the Family window, the Family Tree, taking juniors (report
//! b039), and the entitlements bought with Rep (report b050).

use super::ClientActor;
use crate::message::{DirectoryMessage, FamilyTrip, SummonFailure};
use crate::{db::spawn_db, error::RuntimeError};
use db::family::{Separation, Spending};
use game::family::{
    Benefit, Entitlement, FamilyBuff, Refusal, BONDING_JUNIORS, ENTITLEMENTS, SUMMON, TELEPORT,
};
use game_data::server_content::keys;
use net::packet::build::world::family::{
    build_family_info, build_family_refusal, build_join_accepted, build_join_result,
    build_privilege_list, build_rep_gain, build_separated, build_separation_fee,
    build_set_privilege,
};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::stat::{self, StatUpdate};
use net::packet::handle::world::{may_summon_to, may_travel, travel_to, Whereabouts};
use net::{ClientId, FamilyRequest};
use std::time::SystemTime;
use tokio::time::Instant;
use tracing::warn;

impl ClientActor {
    /// On entering the game, and when the Family window opens: its numbers
    /// and the entitlements. The client asks for nothing else until then.
    /// Relatives are read again too: a change made while the player was
    /// entering the game reached no one.
    pub(super) async fn load_family(&mut self) -> Result<(), RuntimeError> {
        self.family_changed().await?;
        self.send(&mut build_privilege_list()?).await
    }

    async fn send_family_info(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let info = spawn_db(move || db::family::info(cid)).await?;
        self.send(&mut build_family_info(&info)?).await
    }

    pub(super) async fn family_request(
        &mut self,
        request: FamilyRequest,
    ) -> Result<(), RuntimeError> {
        match request {
            FamilyRequest::Open => self.load_family().await,
            FamilyRequest::Chart { name } => {
                let viewer = self.client_id;
                let chart = spawn_db(move || {
                    db::family::character(&name)?
                        .map(|viewed| db::family::chart(viewed, viewer))
                        .transpose()
                })
                .await?;
                let Some(chart) = chart else {
                    return Ok(());
                };
                self.send_to_directory(DirectoryMessage::FamilyChart {
                    owner: self.client_id,
                    chart,
                })
                .await
            }
            FamilyRequest::Add { name } => self.ask_junior(name).await,
            FamilyRequest::Answer { inviter, accept } => {
                self.send_to_directory(DirectoryMessage::FamilyAnswer {
                    from: self.client_id,
                    inviter,
                    accept,
                })
                .await
            }
            FamilyRequest::Precepts { precepts } => {
                let cid = self.client_id;
                let ids = spawn_db(move || db::family::set_precepts(cid, &precepts)).await?;
                match ids {
                    Some(ids) => {
                        self.send_to_directory(DirectoryMessage::FamilyChanged { ids })
                            .await
                    }
                    None => Ok(()),
                }
            }
            FamilyRequest::Separate { junior } => self.ask_separate(junior).await,
            FamilyRequest::Use { index, target } => self.use_entitlement(index, target).await,
            FamilyRequest::SummonAnswer { summoner, accept } => {
                self.send_to_directory(DirectoryMessage::FamilySummonAnswer {
                    from: self.client_id,
                    summoner,
                    accept,
                })
                .await
            }
        }
    }

    /// Spend entitlement `index`. The client lets the player choose one
    /// only with the Rep and uses left, but checks nothing else.
    async fn use_entitlement(
        &mut self,
        index: usize,
        target: Option<String>,
    ) -> Result<(), RuntimeError> {
        let Some(entitlement) = ENTITLEMENTS.get(index) else {
            return Ok(());
        };
        match entitlement.benefit {
            Benefit::Teleport | Benefit::Summon => {}
            Benefit::Rate { party: false, .. } => return self.buy_buff(index).await,
            Benefit::Rate { party: true, .. } => {
                if !self.spend_entitlement(index).await? {
                    return Ok(());
                }
                let buff = FamilyBuff::new(index, SystemTime::now());
                self.family_buff(buff).await?;
                let from = self.client_id;
                self.send_to_directory(DirectoryMessage::FamilyPartyBuff { from, buff })
                    .await?;
                return self.tell_spent(index).await;
            }
            Benefit::Bonding { .. } => {
                // Paid for once the juniors are counted.
                let cid = self.client_id;
                let juniors = spawn_db(move || db::family::tree_juniors(cid)).await?;
                return self
                    .send_to_directory(DirectoryMessage::FamilyBondingCount { from: cid, juniors })
                    .await;
            }
        }
        let Some(name) = target else {
            return Ok(());
        };
        let cid = self.client_id;
        let member = spawn_db(move || {
            let member = db::family::character(&name)?.filter(|&id| id != cid);
            let Some(id) = member else {
                return Ok(Err(Refusal::NotFound));
            };
            if !db::family::same_family(cid, id)? {
                return Ok(Err(Refusal::NotSameFamily));
            }
            Ok(Ok((id, name)))
        })
        .await?;
        let (target, name) = match member {
            Ok(member) => member,
            Err(refusal) => return self.send(&mut build_family_refusal(refusal)?).await,
        };
        if entitlement.benefit == Benefit::Teleport {
            // Paid for once they are found.
            return self
                .send_to_directory(DirectoryMessage::FamilyLocate { from: cid, target })
                .await;
        }
        let player = &self.in_game()?.player;
        if !may_summon_to(player) {
            return self
                .send(&mut build_family_refusal(Refusal::CannotSummon)?)
                .await;
        }
        let map_id = player.stats.map_id;
        if !self.spend_entitlement(SUMMON).await? {
            return Ok(());
        }
        // The summons is sent before the player hears they paid, so that it
        // can be refunded whatever happens next.
        let map_name = net::items::map_name(map_id).unwrap_or_default();
        self.send_to_directory(DirectoryMessage::FamilySummon {
            from: cid,
            target,
            name,
            map_id,
            map_name,
        })
        .await?;
        self.tell_spent(SUMMON).await
    }

    /// Pay for the rate entitlement `index`, for the player alone.
    async fn buy_buff(&mut self, index: usize) -> Result<(), RuntimeError> {
        if !self.spend_entitlement(index).await? {
            return Ok(());
        }
        self.family_buff(FamilyBuff::new(index, SystemTime::now()))
            .await?;
        self.tell_spent(index).await
    }

    /// `online` of the juniors bonding needs are: buy it, or say why not.
    pub(super) async fn family_bonding(&mut self, online: usize) -> Result<(), RuntimeError> {
        if online >= BONDING_JUNIORS {
            let index = ENTITLEMENTS
                .iter()
                .position(|entitlement| matches!(entitlement.benefit, Benefit::Bonding { .. }))
                .unwrap_or_default();
            return self.buy_buff(index).await;
        }
        // The client has the line (0x1200) but never shows it.
        let text = net::content::text().get(keys::FAMILY_BONDING_SHORT, &[]);
        self.send(&mut build_broadcast(Broadcast::Pink, &text)?)
            .await
    }

    /// `buff` now works for the player, in place of any other, and its icon
    /// shows.
    pub(super) async fn family_buff(&mut self, buff: FamilyBuff) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        if let Err(error) = spawn_db(move || db::family::set_buff(&[cid], buff)).await {
            warn!(cid, %error, "Failed to save a family entitlement");
        }
        // A party member may be on their way in.
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        game.player.family_buff = Some(buff);
        self.show_family_buff().await
    }

    /// The icon of the family entitlement working for the player, if any.
    pub(super) async fn show_family_buff(&mut self) -> Result<(), RuntimeError> {
        let buff = self.in_game()?.player.family_buff;
        self.send(&mut build_set_privilege(buff.as_ref(), SystemTime::now())?)
            .await
    }

    /// When the family entitlement working for the player ends.
    pub(super) fn family_buff_due(&self) -> Option<Instant> {
        let until = self.game.as_ref()?.player.family_buff?.until;
        let wait = until.duration_since(SystemTime::now()).unwrap_or_default();
        Some(Instant::now() + wait)
    }

    /// The family entitlement ran out: its rates stop and its icon goes.
    pub(super) async fn end_family_buff(&mut self) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let Some(buff) = player
            .family_buff
            .filter(|buff| buff.until <= SystemTime::now())
        else {
            return Ok(());
        };
        player.family_buff = None;
        let (cid, until) = (self.client_id, buff.until);
        if let Err(error) = spawn_db(move || db::family::end_buff(cid, until)).await {
            warn!(cid, %error, "Failed to end a family entitlement");
        }
        self.show_family_buff().await
    }

    /// Pay for entitlement `index`. False if the player can't: short of Rep
    /// or out of uses, which the client's button already showed.
    async fn spend_entitlement(&self, index: usize) -> Result<bool, RuntimeError> {
        let cid = self.client_id;
        let Entitlement { cost, limit, .. } = ENTITLEMENTS[index];
        let slot = index as i16;
        let spending = spawn_db(move || db::family::spend(cid, slot, cost, limit)).await?;
        Ok(spending == Spending::Spent)
    }

    /// "You have used %d Rep Points.", and the window's numbers.
    async fn tell_spent(&mut self, index: usize) -> Result<(), RuntimeError> {
        self.send(&mut build_rep_gain(-ENTITLEMENTS[index].cost, "")?)
            .await?;
        self.send_family_info().await
    }

    /// Where the family member the player is going to stands, if found.
    pub(super) async fn family_located(
        &mut self,
        trip: FamilyTrip,
        found: Option<Whereabouts>,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let found = found.filter(|found| may_travel(player, found.map_id));
        match (trip, found) {
            (FamilyTrip::Teleport, None) => {
                self.send(&mut build_family_refusal(Refusal::CannotSummon)?)
                    .await
            }
            (FamilyTrip::Teleport, Some(found)) => {
                if self.spend_entitlement(TELEPORT).await? {
                    self.travel(found).await?;
                    self.tell_spent(TELEPORT).await?;
                }
                Ok(())
            }
            (FamilyTrip::Summoned { summoner }, None) => {
                self.send_to_directory(DirectoryMessage::FamilySummonFailed {
                    from: self.client_id,
                    summoner,
                })
                .await
            }
            (FamilyTrip::Summoned { .. }, Some(found)) => self.travel(found).await,
        }
    }

    async fn travel(&mut self, found: Whereabouts) -> Result<(), RuntimeError> {
        let channel_id = self.channel_id;
        let player = &mut self.in_game_mut()?.player;
        for action in travel_to(player, found, channel_id)? {
            self.apply(action).await?;
        }
        Ok(())
    }

    /// The summons the player paid for to bring `name` didn't happen.
    pub(super) async fn summon_refund(
        &mut self,
        name: String,
        why: SummonFailure,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let cost = ENTITLEMENTS[SUMMON].cost;
        let slot = SUMMON as i16;
        if let Err(error) = spawn_db(move || db::family::refund(cid, slot, cost)).await {
            warn!(cid, %error, "Failed to refund a summons");
        }
        let mut packet = match why {
            SummonFailure::NotFound => build_family_refusal(Refusal::NotFound)?,
            SummonFailure::Busy => build_family_refusal(Refusal::SummonPending)?,
            SummonFailure::Location => build_family_refusal(Refusal::CannotSummon)?,
            SummonFailure::Declined => {
                let text =
                    net::content::text().get(keys::FAMILY_SUMMON_DECLINED, &[("name", &name)]);
                build_broadcast(Broadcast::Pink, &text)?
            }
        };
        self.send(&mut packet).await?;
        self.send_family_info().await
    }

    /// Part from `junior`, or from the senior. The directory has the levels
    /// of those online, which the client worked the fee out from.
    async fn ask_separate(&mut self, junior: Option<ClientId>) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let family = &self.in_game()?.player.family;
        let (senior, junior) = match (junior, family.senior) {
            (Some(junior), _) if family.juniors.contains(&junior) => (cid, junior),
            (None, Some(senior)) => (senior, cid),
            _ => return Ok(()),
        };
        self.send_to_directory(DirectoryMessage::FamilySeparate {
            from: cid,
            senior,
            junior,
        })
        .await
    }

    /// Part `junior` from `senior` for the player, who pays. Only they hear
    /// how it went; the whole family as it was gets INFO again.
    pub(super) async fn separate(
        &mut self,
        senior: ClientId,
        junior: ClientId,
        levels: (Option<u8>, Option<u8>),
    ) -> Result<(), RuntimeError> {
        let meso = self.in_game()?.player.meso;
        let separation = spawn_db(move || db::family::separate(senior, junior, levels, meso)).await;
        let (fee, ids) = match separation {
            // The client's tree is out of date, or the rows were busy: show
            // the family as it is.
            Ok(Separation::NotRelated) => return self.family_changed().await,
            Err(error) => {
                warn!(senior, junior, %error, "Failed to separate");
                return self.family_changed().await;
            }
            Ok(Separation::Short { fee }) => {
                let from_junior = senior == self.client_id;
                return self
                    .send(&mut build_separation_fee(from_junior, fee)?)
                    .await;
            }
            Ok(Separation::Parted { fee, ids }) => (fee, ids),
        };
        if fee > 0 {
            let player = &mut self.in_game_mut()?.player;
            player.meso -= fee;
            player.mark_dirty();
            let meso = player.meso;
            self.flush().await?;
            self.send(&mut stat::build_stat_update(
                false,
                &[StatUpdate::Meso(meso)],
            )?)
            .await?;
        }
        self.send(&mut build_separated()?).await?;
        self.send_to_directory(DirectoryMessage::FamilyChanged { ids })
            .await
    }

    /// The player earned their seniors `gain` Rep, by a kill or a level.
    pub(super) async fn family_earn(&mut self, gain: i32) -> Result<(), RuntimeError> {
        if self.in_game()?.player.family.senior.is_none() {
            return Ok(());
        }
        self.send_to_directory(DirectoryMessage::FamilyEarn {
            from: self.client_id,
            gain,
        })
        .await
    }

    /// Credit the seniors with `gain`. Each online hears it and gets INFO.
    pub(super) async fn earn_rep(
        &mut self,
        gain: i32,
        senior_level: Option<u8>,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let stats = &self.in_game()?.player.stats;
        let (level, name) = (stats.level, stats.name.clone());
        let earned = match spawn_db(move || db::family::earn(cid, gain, level, senior_level)).await
        {
            Ok(earned) => earned,
            Err(error) => {
                warn!(cid, %error, "Failed to earn Rep");
                return Ok(());
            }
        };
        for &(id, gain) in &earned {
            self.send_to_directory(DirectoryMessage::Deliver {
                to: vec![id],
                packet: build_rep_gain(gain, &name)?,
            })
            .await?;
        }
        if earned.is_empty() {
            return Ok(());
        }
        let ids = earned.into_iter().map(|(id, _)| id).collect();
        self.send_to_directory(DirectoryMessage::FamilyChanged { ids })
            .await
    }

    /// Ask the character named `name` to be the player's junior. The
    /// directory checks the map and levels.
    async fn ask_junior(&mut self, name: String) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let asked = spawn_db(move || {
            let Some(junior) = db::family::character(&name)? else {
                return Ok(Err(Refusal::NotFound));
            };
            Ok(db::family::may_add(cid, junior)?.map(|()| junior))
        })
        .await?;
        match asked {
            Ok(junior) => {
                self.send_to_directory(DirectoryMessage::FamilyInvite { from: cid, junior })
                    .await
            }
            Err(refusal) => self.send(&mut build_family_refusal(refusal)?).await,
        }
    }

    /// `junior` (named `name`) said yes. The rules are checked again, since
    /// either may have joined another family meanwhile.
    pub(super) async fn add_junior(
        &mut self,
        junior: ClientId,
        name: String,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let ids = match spawn_db(move || db::family::add_junior(cid, junior)).await {
            Ok(Ok(ids)) => ids,
            Ok(Err(refusal)) => return self.send(&mut build_family_refusal(refusal)?).await,
            // The junior's character may be gone meanwhile.
            Err(error) => {
                warn!(cid, junior, %error, "Failed to add a junior");
                return self
                    .send(&mut build_family_refusal(Refusal::CannotAdd)?)
                    .await;
            }
        };
        self.send(&mut build_join_result(true, &name)?).await?;
        let senior = self.in_game()?.player.stats.name.clone();
        self.send_to_directory(DirectoryMessage::Deliver {
            to: vec![junior],
            packet: build_join_accepted(&senior)?,
        })
        .await?;
        self.send_to_directory(DirectoryMessage::FamilyChanged { ids })
            .await
    }

    /// The family changed: who hears of logins, and the Family window. An
    /// open Family Tree asks again on the window's numbers.
    pub(super) async fn family_changed(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let relatives = spawn_db(move || db::family::relatives_of(cid)).await?;
        self.in_game_mut()?.player.family = relatives;
        self.send_family_info().await
    }
}
