use super::ClientActor;
use crate::{
    db::spawn_db,
    error::RuntimeError,
    message::{ClientHandle, FieldMessage},
};
use db::fame::Refusal;
use net::packet::build::world::fame::{build_fame_response, FameResponse};
use net::packet::build::world::stat::{self, StatUpdate};
use net::FieldRequest;

/// The most fame (or the least, negated) a character can have
/// (OdinMS-family limit).
const FAME_LIMIT: i16 = 30000;

impl ClientActor {
    /// This player raises (or drops) `target`'s fame: once a day, and once a
    /// month for the same character. Their field hands it to `target`.
    pub(super) async fn give_fame(&mut self, target: i32, up: bool) -> Result<(), RuntimeError> {
        let giver = self.client_id;
        let refusal = match spawn_db(move || db::fame::give(giver, target)).await? {
            Ok(()) => None,
            Err(Refusal::NotFound) => Some(FameResponse::NotFound),
            Err(Refusal::NotToday) => Some(FameResponse::NotToday),
            Err(Refusal::NotThisMonth) => Some(FameResponse::NotThisMonth),
        };
        if let Some(refusal) = refusal {
            return self.send(&mut build_fame_response(&refusal)?).await;
        }
        let giver_name = self.in_game()?.player.stats.name.clone();
        self.send_to_field(FieldMessage::FromClient {
            from: giver,
            request: FieldRequest::Fame {
                target,
                giver_name,
                up,
            },
        })
        .await
    }

    /// The fame given `target` reached nobody: it doesn't count.
    pub(super) async fn fame_missed(&mut self, target: i32) -> Result<(), RuntimeError> {
        let giver = self.client_id;
        spawn_db(move || db::fame::forget(giver, target)).await?;
        self.send(&mut build_fame_response(&FameResponse::NotFound)?)
            .await
    }

    /// `giver_name` raised (or dropped) this player's fame: it changes by
    /// one, and each hears of it.
    pub(super) async fn take_fame(
        &mut self,
        giver: ClientHandle,
        giver_name: String,
        up: bool,
    ) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let fame = player.stats.fame + if up { 1 } else { -1 };
        if fame.abs() > FAME_LIMIT {
            giver.send_packet(build_fame_response(&FameResponse::Failed)?);
            return Ok(());
        }
        player.stats.fame = fame;
        player.mark_dirty();
        let target = player.stats.name.clone();
        let mut packets = vec![
            stat::build_stat_update(false, &[StatUpdate::Fame(fame)])?,
            build_fame_response(&FameResponse::Received {
                giver: giver_name,
                up,
            })?,
        ];
        giver.send_packet(build_fame_response(&FameResponse::Gave {
            target,
            up,
            fame,
        })?);
        self.send_all(&mut packets).await
    }
}
