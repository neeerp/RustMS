//! Families: the Family window, the Family Tree, and taking juniors
//! (report b039).

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::DirectoryMessage};
use db::family::Separation;
use game::family::Refusal;
use net::packet::build::world::family::{
    build_family_info, build_family_refusal, build_join_accepted, build_join_result,
    build_privilege_list, build_rep_gain, build_separated, build_separation_fee,
};
use net::packet::build::world::stat::{self, StatUpdate};
use net::{ClientId, FamilyRequest};
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
                let chart = spawn_db(move || {
                    db::family::character(&name)?
                        .map(db::family::chart)
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
        }
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
