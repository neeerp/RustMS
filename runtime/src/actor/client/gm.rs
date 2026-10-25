//! The chat commands (`net::gm_commands`) that need the database.

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use db::cash_shop::CashType;
use db::coupon::Reward;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use rand::{thread_rng, Rng};
use std::time::{Duration, SystemTime};
use tracing::info;

/// What a made code is drawn from: letters and digits, less those easily
/// taken for others (0 O 1 I).
const CODE_CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CODE_LEN: usize = 16;

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

    /// `!coupon`: make the coupon and tell the GM its code.
    pub(super) async fn make_coupon(
        &mut self,
        code: Option<String>,
        days: Option<u32>,
        rewards: Vec<Reward>,
    ) -> Result<(), RuntimeError> {
        let gm = self.in_game()?.player.stats.name.clone();
        let expires_at =
            days.map(|days| SystemTime::now() + Duration::from_secs(u64::from(days) * 86_400));
        let made = spawn_db(move || -> db::Result<Option<String>> {
            let code = match code {
                Some(code) if db::coupon::find(&code)?.is_some() => return Ok(None),
                Some(code) => code,
                None => loop {
                    let mut rng = thread_rng();
                    let code: String = (0..CODE_LEN)
                        .map(|_| CODE_CHARS[rng.gen_range(0, CODE_CHARS.len())] as char)
                        .collect();
                    if db::coupon::find(&code)?.is_none() {
                        break code;
                    }
                },
            };
            db::coupon::create(&code, expires_at, &rewards)?;
            Ok(Some(code))
        })
        .await?;
        let line = match (made, days) {
            (None, _) => "That code is taken.".to_string(),
            (Some(code), None) => format!("Made coupon {code}."),
            (Some(code), Some(days)) => format!("Made coupon {code}, good for {days} days."),
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
