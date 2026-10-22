//! Families: the Family window, the Family Tree, and taking juniors
//! (report b039).

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::DirectoryMessage};
use game::family::Refusal;
use net::packet::build::world::family::{
    build_family_info, build_family_refusal, build_join_accepted, build_join_result,
    build_privilege_list,
};
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
        }
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
