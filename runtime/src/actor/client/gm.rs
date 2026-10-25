//! The chat commands (`net::gm_commands`) that need the database.

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use db::cash_shop::CashType;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use tracing::info;

impl ClientActor {
    /// `!givenx`: add `amount` to a balance of the account of the character
    /// named `target`, or of the GM's own, and tell the GM what it came to.
    /// The Cash Shop reads the balances afresh on entry and on every Check,
    /// so a player in it sees the NX on the next one.
    pub(super) async fn give_cash(
        &mut self,
        target: Option<String>,
        cash_type: CashType,
        amount: i32,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let (gm, own) = (player.stats.name.clone(), player.account_id);
        let line = match target {
            None => self.grant(own, &gm, cash_type, amount).await?,
            Some(name) => {
                let lookup = name.clone();
                match spawn_db(move || db::cash_shop::gift_receiver(&lookup)).await? {
                    Some(receiver) => {
                        self.grant(receiver.account_id, &receiver.name, cash_type, amount)
                            .await?
                    }
                    None => format!("There is no character named {name}."),
                }
            }
        };
        info!(target: "gm_audit", character = %gm, result = %line, "GM command");
        self.send(&mut build_broadcast(Broadcast::Pink, &line)?)
            .await
    }

    async fn grant(
        &mut self,
        account_id: i32,
        name: &str,
        cash_type: CashType,
        amount: i32,
    ) -> Result<String, RuntimeError> {
        let Some(balance) =
            spawn_db(move || db::cash_shop::grant(account_id, cash_type, amount)).await?
        else {
            return Ok(format!("{name}'s account is gone."));
        };
        let (kind, now) = match cash_type {
            CashType::Credit => ("NX Credit", balance.credit),
            CashType::MaplePoints => ("Maple Points", balance.points),
            CashType::Prepaid => ("NX Prepaid", balance.prepaid),
        };
        Ok(format!("{name}'s account has {now} {kind} ({amount:+})."))
    }
}
