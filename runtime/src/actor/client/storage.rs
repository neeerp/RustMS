use super::ClientActor;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use net::packet::build::world::{stat, storage::build_open_storage};
use net::storage::{OpenStorage, StorageRequest};
use tracing::info;

impl ClientActor {
    /// Open the account's storage if `npc_id` keeps one. It stays open until
    /// the player closes it, talks to another NPC, or changes maps.
    pub(super) async fn open_storage(&mut self, npc_id: i32) -> Result<bool, RuntimeError> {
        let Some(fees) = net::storage::fees(npc_id) else {
            return Ok(false);
        };
        let account_id = self.in_game()?.player.account_id;
        let storage = spawn_db(move || db::storage::load(account_id)).await?;
        let mut packet = build_open_storage(npc_id, &storage)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut packet).await?;
        self.storage = Some(OpenStorage {
            npc_id,
            fees,
            storage,
        });
        Ok(true)
    }

    /// Answer a request to the open storage. A change is saved with the
    /// player at once, in one transaction, so an item or meso is never in
    /// both places or neither; if saving fails, both go back as they were.
    pub(super) async fn storage_request(
        &mut self,
        request: StorageRequest,
    ) -> Result<(), RuntimeError> {
        if request == StorageRequest::Close {
            self.storage = None;
            return Ok(());
        }
        let Some(open) = self.storage.as_mut() else {
            let mut packet = stat::build_stat_update(true, &[])
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            return self.send(&mut packet).await;
        };
        let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
        let before = (open.storage.clone(), player.clone());
        let outcome = open
            .respond(player, request)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        if outcome.changed {
            let account_id = player.account_id;
            let (storage, snapshot) = (open.storage.clone(), player.clone());
            let saved =
                spawn_db(move || db::storage::save(account_id, &storage, Some(&snapshot))).await;
            let open = self.storage.as_mut().expect("still open");
            let player = &mut self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)?.player;
            if let Err(error) = saved {
                (open.storage, *player) = before;
                return Err(error);
            }
            player.mark_saved();
            info!(
                self.client_id,
                npc_id = open.npc_id,
                ?request,
                meso = open.storage.meso,
                items = open.storage.all().len(),
                "Storage changed"
            );
        }
        for mut packet in outcome.packets {
            self.send(&mut packet).await?;
        }
        Ok(())
    }
}
