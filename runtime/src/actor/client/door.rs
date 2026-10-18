use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldKey, FieldMessage};
use net::packet::build::world::door::DoorView;
use net::packet::build::world::map::{build_blocked_map, BlockedMap};
use net::packet::handle::world::{door_warp, DoorExit};
use std::time::Duration;
use tokio::time::Instant;

/// The player's open Mystic Door: the field it stands in, and when it
/// closes.
pub(super) struct OpenDoor {
    field: FieldKey,
    closes: Instant,
}

impl ClientActor {
    /// The player cast Mystic Door at `at` on their map: close the door they
    /// had, and open this one for `lasts`.
    pub(super) async fn open_door(
        &mut self,
        town: i32,
        at: (i16, i16),
        lasts: Duration,
    ) -> Result<(), RuntimeError> {
        self.close_door().await?;
        let field = self.in_game()?.field.key;
        self.send_to_field(FieldMessage::OpenDoor {
            owner: self.client_id,
            at,
        })
        .await?;
        let door = DoorView {
            town,
            field: field.map_id,
            x: at.0,
            y: at.1,
        };
        self.send_to_directory(DirectoryMessage::DoorOpened {
            owner: self.client_id,
            door,
        })
        .await?;
        self.door = Some(OpenDoor {
            field,
            closes: Instant::now() + lasts,
        });
        Ok(())
    }

    /// When the player's door closes on its own.
    pub(super) fn door_due(&self) -> Option<Instant> {
        self.door.as_ref().map(|door| door.closes)
    }

    /// Close the player's door, if open: it fades from its field, and
    /// leaves the party's town.
    pub(super) async fn close_door(&mut self) -> Result<(), RuntimeError> {
        let Some(door) = self.door.take() else {
            return Ok(());
        };
        let owner = self.client_id;
        // The field may be another than the player's; one that is gone has
        // taken the door with it.
        let _ = self
            .services
            .fields
            .field(door.field)
            .tx
            .send(FieldMessage::CloseDoor { owner })
            .await;
        self.send_to_directory(DirectoryMessage::DoorClosed { owner })
            .await
    }

    /// The player entered someone's door (see `WorldDirectory::use_door`).
    pub(super) async fn use_door(
        &mut self,
        owner: i32,
        from_town: bool,
    ) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::UseDoor {
            from: self.client_id,
            owner,
            from_town,
        })
        .await
    }

    /// The directory let the player through a door on `from_map`; take them
    /// to `map_id` if they are still there and alive.
    pub(super) async fn door_warp(
        &mut self,
        from_map: i32,
        map_id: i32,
        exit: DoorExit,
    ) -> Result<(), RuntimeError> {
        let (channel_id, in_cash_shop) = (self.channel_id, self.in_cash_shop);
        let player = &mut self.in_game_mut()?.player;
        if player.stats.map_id != from_map || player.stats.hp <= 0 || in_cash_shop {
            let mut refused = build_blocked_map(BlockedMap::PortalClosed)?;
            return self.send(&mut refused).await;
        }
        let actions = door_warp(player, map_id, exit, channel_id)?;
        for action in actions {
            self.apply(action).await?;
        }
        Ok(())
    }
}
