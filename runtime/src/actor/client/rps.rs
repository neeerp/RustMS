use super::quest::pay_mesos;
use super::ClientActor;
use crate::error::RuntimeError;
use game::InventoryType;
use net::packet::build::world::inventory;
use net::packet::build::world::rps::{
    build_rps, build_rps_open, build_rps_round, CLOSE, NEXT_ROUND, NO_MESOS, NO_SLOT, START,
    TIME_OVER,
};
use net::rps::{RpsGame, RpsRequest, RpsStep, FEE, HANDS, RPS_ADMIN};
use rand::Rng;
use tracing::warn;

impl ClientActor {
    /// Open the Rock, Paper, Scissors window if `npc_id` is its admin. The
    /// client closes the window with its X without a word, so a talk always
    /// starts a new game; the window is modal, so it can't still be open.
    pub(super) async fn open_rps(&mut self, npc_id: i32) -> Result<bool, RuntimeError> {
        if npc_id != RPS_ADMIN {
            return Ok(false);
        }
        self.send(&mut build_rps_open(npc_id)?).await?;
        self.rps = Some(RpsGame::new());
        Ok(true)
    }

    pub(super) async fn rps_request(&mut self, request: RpsRequest) -> Result<(), RuntimeError> {
        let Some(game) = self.rps.as_mut() else {
            return Ok(());
        };
        let hand = rand::thread_rng().gen_range(0, HANDS);
        match game.request(request, hand) {
            RpsStep::Ignore => Ok(()),
            RpsStep::Charge => self.charge_rps().await,
            RpsStep::Round {
                hand,
                streak,
                mesos,
                certificate,
            } => {
                self.send(&mut build_rps_round(hand, streak)?).await?;
                self.reward_rps(mesos, certificate).await
            }
            RpsStep::TimeOver => self.send(&mut build_rps(TIME_OVER)?).await,
            RpsStep::NextRound => self.send(&mut build_rps(NEXT_ROUND)?).await,
            RpsStep::Close { certificate } => {
                self.rps = None;
                self.reward_rps(0, certificate).await?;
                self.send(&mut build_rps(CLOSE)?).await
            }
        }
    }

    /// Take the fee for a game, which needs a free etc. slot for the
    /// certificate it may earn.
    async fn charge_rps(&mut self) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let inventory = &player.inventory;
        let refusal = if player.meso < FEE {
            Some(NO_MESOS)
        } else if inventory.items(InventoryType::Etc).count()
            >= usize::from(inventory.slot_limit(InventoryType::Etc))
        {
            Some(NO_SLOT)
        } else {
            None
        };
        if let Some(mode) = refusal {
            if let Some(game) = self.rps.as_mut() {
                game.refused();
            }
            return self.send(&mut build_rps(mode)?).await;
        }
        let mut next = player.clone();
        let mut packets = Vec::new();
        pay_mesos(&mut next, -FEE, &mut packets)?;
        if !self.commit_player(next, None, &mut packets).await? {
            return Ok(());
        }
        self.send_all(&mut packets).await?;
        if let Some(game) = self.rps.as_mut() {
            game.charged();
        }
        self.send(&mut build_rps(START)?).await
    }

    /// Pay `mesos` and give `certificate`.
    async fn reward_rps(
        &mut self,
        mesos: i32,
        certificate: Option<i32>,
    ) -> Result<(), RuntimeError> {
        if mesos == 0 && certificate.is_none() {
            return Ok(());
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        pay_mesos(&mut next, mesos, &mut packets)?;
        if let Some(item_id) = certificate {
            match net::items::give(&mut next.inventory, item_id, 1) {
                Ok(changes) => {
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    packets.push(inventory::build_item_gain_in_chat(item_id, 1)?);
                }
                Err(error) => {
                    warn!(self.client_id, item_id, %error, "No room for the RPS certificate")
                }
            }
        }
        if self.commit_player(next, None, &mut packets).await? {
            self.send_all(&mut packets).await?;
        }
        Ok(())
    }
}
