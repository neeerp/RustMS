//! The chat commands (`net::gm_commands`) that need the database.

use super::ClientActor;
use crate::message::DirectoryMessage;
use crate::{db::spawn_db, error::RuntimeError};
use db::cash_shop::CashType;
use db::coupon::Reward;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::skill::build_admin_result;
use net::Discipline;
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

    /// `!ban`, `!unban`, `!dc`, and the client's `/ban` and `/block`: block
    /// or unblock the account of the character named in `discipline`, or
    /// disconnect them. A block disconnects every character of the account
    /// that is online. Nobody may discipline a GM of their own rank or
    /// higher. The slash commands hear ADMIN_RESULT, the chat commands a
    /// line.
    pub(super) async fn discipline(&mut self, discipline: Discipline) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let (gm, gm_level) = (player.stats.name.clone(), player.gm_level);
        let (target, slash) = match &discipline {
            Discipline::Block { target, slash, .. } => (target.clone(), *slash),
            Discipline::Unblock { target }
            | Discipline::Disconnect { target }
            | Discipline::Jail { target, .. }
            | Discipline::Release { target }
            | Discipline::Mute { target, .. }
            | Discipline::Unmute { target } => (target.clone(), false),
        };
        let lookup = target.clone();
        let found = spawn_db(move || {
            let Some(receiver) = db::cash_shop::gift_receiver(&lookup)? else {
                return Ok(None);
            };
            let account = db::account::get_account_by_id(receiver.account_id)?;
            let characters = db::account::character_ids(receiver.account_id)?;
            Ok(Some((receiver, account.gm_level, characters)))
        })
        .await?;
        let outcome = match found {
            None => Err(format!("There is no character named {target}.")),
            Some((receiver, level, _)) if level >= gm_level => {
                Err(format!("{} is your rank or higher.", receiver.name))
            }
            Some((receiver, _, characters)) => {
                let (account_id, name) = (receiver.account_id, receiver.name);
                Ok(match discipline {
                    Discipline::Block {
                        reason, days, note, ..
                    } => {
                        let until = days.map(|days| {
                            SystemTime::now() + Duration::from_secs(u64::from(days) * 86_400)
                        });
                        let note = (!note.is_empty()).then_some(note);
                        spawn_db(move || {
                            db::account::block(
                                account_id,
                                i16::from(reason),
                                until,
                                note.as_deref(),
                            )
                        })
                        .await?;
                        self.kick(characters).await?;
                        match days {
                            Some(days) => format!("Blocked {name}'s account for {days} days."),
                            None => format!("Blocked {name}'s account for good."),
                        }
                    }
                    Discipline::Unblock { .. } => {
                        if spawn_db(move || db::account::unblock(account_id)).await? {
                            format!("Unblocked {name}'s account.")
                        } else {
                            format!("{name}'s account is not blocked.")
                        }
                    }
                    Discipline::Disconnect { .. } => {
                        if self.kick(vec![receiver.id]).await? > 0 {
                            format!("Disconnected {name}.")
                        } else {
                            format!("{name} is not online.")
                        }
                    }
                    Discipline::Jail { minutes, .. } => {
                        let (character_id, now) = (receiver.id, SystemTime::now());
                        let time = Duration::from_secs(u64::from(minutes) * 60);
                        let until =
                            spawn_db(move || db::jail::jail(character_id, time, now)).await?;
                        self.tell_jail_changed(character_id).await?;
                        let left = until
                            .duration_since(now)
                            .unwrap_or_default()
                            .as_secs()
                            .div_ceil(60);
                        format!("Jailed {name} for {minutes} minutes ({left} in all).")
                    }
                    Discipline::Release { .. } => {
                        let character_id = receiver.id;
                        let now = SystemTime::now();
                        if spawn_db(move || db::jail::release(character_id, now)).await? {
                            self.tell_jail_changed(character_id).await?;
                            format!("Let {name} out of jail.")
                        } else {
                            format!("{name} is not in jail.")
                        }
                    }
                    Discipline::Mute {
                        minutes, reason, ..
                    } => {
                        let until =
                            SystemTime::now() + Duration::from_secs(u64::from(minutes) * 60);
                        spawn_db(move || {
                            db::account::chat_block(account_id, i16::from(reason), until)
                        })
                        .await?;
                        self.tell_chat_block_changed(characters).await?;
                        format!("Blocked {name}'s account from chatting for {minutes} minutes.")
                    }
                    Discipline::Unmute { .. } => {
                        let now = SystemTime::now();
                        if spawn_db(move || db::account::chat_unblock(account_id, now)).await? {
                            self.tell_chat_block_changed(characters).await?;
                            format!("{name}'s account may chat again.")
                        } else {
                            format!("{name}'s account is not blocked from chatting.")
                        }
                    }
                })
            }
        };
        let line = match &outcome {
            Ok(line) | Err(line) => line,
        };
        info!(target: "gm_audit", character = %gm, result = %line, "GM command");
        let mut reply = match (slash, &outcome) {
            // "You have successfully blocked access."
            (true, Ok(_)) => build_admin_result(4, 0)?,
            // "You have entered an invalid character name."
            (true, Err(_)) => build_admin_result(6, 1)?,
            (false, _) => build_broadcast(Broadcast::Pink, line)?,
        };
        self.send(&mut reply).await
    }

    /// Have the character's client, if online, read their jail time again.
    async fn tell_jail_changed(&mut self, client_id: i32) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::JailChanged { client_id })
            .await
    }

    /// Have the clients of `client_ids` that are online read their
    /// account's chat block again.
    async fn tell_chat_block_changed(&mut self, client_ids: Vec<i32>) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::ChatBlockChanged { client_ids })
            .await
    }

    /// Disconnect whichever of `client_ids` are online: how many were.
    async fn kick(&mut self, client_ids: Vec<i32>) -> Result<usize, RuntimeError> {
        let (reply, online) = tokio::sync::oneshot::channel();
        self.send_to_directory(DirectoryMessage::Kick { client_ids, reply })
            .await?;
        Ok(online.await.unwrap_or(0))
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
