use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use db::report::{Filed, Report};
use net::packet::build::world::claim::{
    build_claim_available_time, build_claim_result, build_claim_status, ClaimResult,
};
use net::packet::build::world::stat::{self, StatUpdate};
use net::packet::handle::world::CLAIM_FEE;
use tracing::warn;

/// How many reports a player may make in seven days (Cosmic's
/// `possibleReports`; v83 only says "this week").
pub(super) const CLAIMS_A_WEEK: i64 = 10;
/// The most of a report's text that is kept; the client sends at most 1,600
/// characters of conversation.
const MOST_KEPT: usize = 2000;

impl ClientActor {
    /// Tell the client reports are taken, all day; without it, its report
    /// button says the system is down.
    pub(super) async fn open_claims(&mut self) -> Result<(), RuntimeError> {
        self.send(&mut build_claim_available_time(0, 0)?).await?;
        self.send(&mut build_claim_status(true)?).await
    }

    /// The player reports `victim`: kept, for 300 mesos, unless they have
    /// none left this week or the name is no one else's.
    pub(super) async fn claim(
        &mut self,
        conversation: bool,
        victim: String,
        reason: u8,
        description: String,
        chat_log: String,
    ) -> Result<(), RuntimeError> {
        if self.in_game()?.player.meso < CLAIM_FEE {
            return self
                .send(&mut build_claim_result(ClaimResult::NoMesos)?)
                .await;
        }
        let reporter_id = self.client_id;
        let filed = spawn_db(move || {
            db::report::file(
                &Report {
                    reporter_id,
                    victim: &victim,
                    kind: i16::from(conversation),
                    reason: i16::from(reason),
                    description: kept(&description),
                    chat_log: kept(&chat_log),
                },
                CLAIMS_A_WEEK,
            )
        })
        .await;
        let left = match filed {
            Ok(Filed::Kept { left }) => left,
            Ok(Filed::NotFound) => {
                return self
                    .send(&mut build_claim_result(ClaimResult::NotFound)?)
                    .await
            }
            Ok(Filed::NoneLeft) => {
                return self
                    .send(&mut build_claim_result(ClaimResult::NoneLeft)?)
                    .await
            }
            Err(error) => {
                warn!(reporter_id, %error, "Failed to keep a report");
                return self
                    .send(&mut build_claim_result(ClaimResult::Later)?)
                    .await;
            }
        };
        let player = &mut self.in_game_mut()?.player;
        player.meso -= CLAIM_FEE;
        player.mark_dirty();
        let meso = player.meso;
        self.flush().await?;
        let mut packets = vec![
            stat::build_stat_update(false, &[StatUpdate::Meso(meso)])?,
            build_claim_result(ClaimResult::Kept {
                left: i32::try_from(left).unwrap_or(0),
            })?,
        ];
        self.send_all(&mut packets).await
    }
}

/// `text` cut to the most that is kept.
fn kept(text: &str) -> &str {
    match text.char_indices().nth(MOST_KEPT) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_is_cut_on_a_character() {
        assert_eq!(kept("short"), "short");
        let long = "é".repeat(MOST_KEPT + 5);
        assert_eq!(kept(&long).chars().count(), MOST_KEPT);
    }
}
