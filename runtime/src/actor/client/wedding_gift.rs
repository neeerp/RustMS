//! The wedding registry's windows (`net::wedding_gift`, reports b084 and
//! b085). In a wedding the couple each send a wishlist through Travis or
//! Debbie, and a guest gives either of them one gift through Pila; the
//! gifts wait in the database until their receiver takes them from Pila
//! outside a wedding. The client throws on an answer it didn't ask for, so
//! each give or take gets exactly one.

use super::ClientActor;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use db::wedding_gift::Gift;
use game_data::server_content::keys;
use net::handler::GiftRequest;
use net::npc_scripts::NpcScript;
use net::packet::build::world::marriage::{
    build_gift_already_given, build_gift_failed, build_gift_not_taken, build_gift_sent,
    build_gift_taken, build_gifts, build_registry, gift_bag,
};
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// The registry window the player has open.
#[derive(Debug)]
pub(super) enum GiftWindow {
    /// The give window on `receiver`'s `wishes`.
    Give { receiver: i32, wishes: Vec<String> },
    /// The receive window on the player's gifts, as it lists them.
    Receive { gifts: Vec<Gift> },
}

impl ClientActor {
    /// One of the couple sent their wishlist (RING_ACTION mode 9). Only
    /// the first is kept.
    pub(super) fn wishlist(&mut self, wishes: Vec<String>) -> Result<(), RuntimeError> {
        let id = self.in_game()?.player.stats.id;
        let Some(ticket) = self.wedding_here().filter(|ticket| !ticket.guest) else {
            warn!(
                self.client_id,
                "Wishlist from outside a wedding of one's own"
            );
            return Ok(());
        };
        if !ticket.registry().wish(id, wishes) {
            debug!(self.client_id, "Second wishlist ignored");
        }
        Ok(())
    }

    /// Whether the player, in a wedding of their own, has sent their
    /// wishlist; `None` for a guest or outside a wedding.
    pub(super) fn wished(&self) -> Option<bool> {
        let id = self.game.as_ref()?.player.stats.id;
        let ticket = self.wedding_here()?;
        Some(ticket.registry().wished(id)).filter(|_| !ticket.guest)
    }

    /// Pila's script: in a wedding, whose wishlist to give from; outside
    /// one, the player's gifts if they have any.
    pub(super) async fn pila(&mut self) -> Result<(), RuntimeError> {
        let in_wedding = self.wedding_here().is_some();
        let gifts = if in_wedding {
            false
        } else {
            let id = self.in_game()?.player.stats.id;
            match spawn_db(move || db::wedding_gift::any(id)).await {
                Ok(gifts) => gifts,
                Err(error) => {
                    warn!(self.client_id, %error, "Couldn't look up wedding gifts");
                    false
                }
            }
        };
        self.start_script(NpcScript::pila(in_wedding, gifts)).await
    }

    /// Open the give window on the groom's, or the `bride`'s, wishlist of
    /// the player's wedding, unless it is their own or they have given
    /// from it.
    pub(super) async fn open_registry(&mut self, bride: bool) -> Result<(), RuntimeError> {
        let giver = self.in_game()?.player.stats.id;
        let Some(ticket) = self.wedding_here() else {
            return Ok(());
        };
        let receiver = if bride {
            ticket.couple.1
        } else {
            ticket.couple.0
        };
        let opened = {
            let registry = ticket.registry();
            if receiver == giver {
                Err(keys::WEDDING_PILA_OWN)
            } else if registry.gave(giver, receiver) {
                Err(keys::WEDDING_PILA_GIVEN)
            } else {
                Ok(registry.wishes(receiver).to_vec())
            }
        };
        match opened {
            Ok(wishes) => {
                self.send(&mut build_registry(&wishes)?).await?;
                self.gift_window = Some(GiftWindow::Give { receiver, wishes });
                Ok(())
            }
            Err(key) => self.start_script(NpcScript::pila_says(key)).await,
        }
    }

    /// Open the receive window on the player's gifts.
    pub(super) async fn open_gifts(&mut self) -> Result<(), RuntimeError> {
        let id = self.in_game()?.player.stats.id;
        let gifts = match spawn_db(move || db::wedding_gift::list(id)).await {
            Ok(gifts) => gifts,
            Err(error) => {
                warn!(self.client_id, %error, "Couldn't load wedding gifts");
                return Ok(());
            }
        };
        self.send(&mut build_gifts(gifts.iter().map(|gift| &gift.item))?)
            .await?;
        self.gift_window = Some(GiftWindow::Receive { gifts });
        Ok(())
    }

    /// WEDDING_ACTION from the window the player has open.
    pub(super) async fn gift_request(&mut self, request: GiftRequest) -> Result<(), RuntimeError> {
        match (self.gift_window.take(), request) {
            (_, GiftRequest::Close) => Ok(()),
            (
                Some(GiftWindow::Give { receiver, wishes }),
                GiftRequest::Give {
                    slot,
                    item_id,
                    quantity,
                },
            ) => {
                let sent = self
                    .give_gift(receiver, &wishes, slot, item_id, quantity)
                    .await?;
                // The client closes the window once the gift went.
                if !sent {
                    self.gift_window = Some(GiftWindow::Give { receiver, wishes });
                }
                Ok(())
            }
            (Some(GiftWindow::Receive { mut gifts }), GiftRequest::Take { kind, index }) => {
                let taken = self.take_gift(&mut gifts, kind, index).await;
                self.gift_window = Some(GiftWindow::Receive { gifts });
                taken
            }
            (window, request) => {
                debug!(
                    self.client_id,
                    ?request,
                    ?window,
                    "Gift request out of place"
                );
                self.gift_window = window;
                Ok(())
            }
        }
    }

    /// Give `receiver` what the give window named, saving the gift with the
    /// player in one transaction. Whether it went.
    async fn give_gift(
        &mut self,
        receiver: i32,
        wishes: &[String],
        slot: i16,
        item_id: i32,
        quantity: i16,
    ) -> Result<bool, RuntimeError> {
        let giver = self.in_game()?.player.stats.id;
        let first = match self.wedding_here() {
            Some(ticket) if [ticket.couple.0, ticket.couple.1].contains(&receiver) => {
                Some(ticket.registry().give(giver, receiver))
            }
            _ => None,
        };
        match first {
            None => {
                self.send(&mut build_gift_failed()?).await?;
                return Ok(false);
            }
            Some(false) => {
                self.send(&mut build_gift_already_given()?).await?;
                return Ok(false);
            }
            Some(true) => {}
        }
        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let given = match net::wedding_gift::give(player, slot, item_id, quantity) {
            Ok(given) => given,
            Err(error) => {
                *player = before;
                self.ungive(giver, receiver);
                return Err(error.into());
            }
        };
        let Some((gift, mut taken)) = given else {
            self.ungive(giver, receiver);
            self.send(&mut build_gift_failed()?).await?;
            return Ok(false);
        };
        let (snapshot, item) = (player.clone(), gift.clone());
        let saved =
            spawn_db(move || db::wedding_gift::give(receiver, &item, &snapshot, SystemTime::now()))
                .await;
        if let Err(error) = saved {
            warn!(self.client_id, %error, "Couldn't save a wedding gift");
            self.in_game_mut()?.player = before;
            self.ungive(giver, receiver);
            self.send(&mut build_gift_failed()?).await?;
            return Ok(false);
        }
        info!(
            self.client_id,
            receiver,
            item_id = gift.item_id,
            quantity = gift.quantity,
            "Wedding gift given"
        );
        self.send(&mut taken).await?;
        self.send(&mut build_gift_sent(wishes, &gift)?).await?;
        Ok(true)
    }

    fn ungive(&self, giver: i32, receiver: i32) {
        if let Some(ticket) = &self.wedding {
            ticket.registry().ungive(giver, receiver);
        }
    }

    /// Take the gift in the receive window's `index`th row of bag `kind`
    /// from `gifts`, saving the player with it gone in one transaction.
    async fn take_gift(
        &mut self,
        gifts: &mut Vec<Gift>,
        kind: u8,
        index: u8,
    ) -> Result<(), RuntimeError> {
        let Some(at) = net::wedding_gift::shown(gifts, kind, index) else {
            return self.send(&mut build_gift_not_taken()?).await;
        };
        let gift = gifts[at].clone();
        let Some(bag) = gift_bag(&gift.item) else {
            return self.send(&mut build_gift_not_taken()?).await;
        };
        let id = self.in_game()?.player.stats.id;
        let player = &mut self.in_game_mut()?.player;
        let before = player.clone();
        let received = match net::wedding_gift::receive(player, &gift.item) {
            Ok(received) => received,
            Err(error) => {
                *player = before;
                return Err(error.into());
            }
        };
        let Some(mut added) = received else {
            return self.send(&mut build_gift_not_taken()?).await;
        };
        let snapshot = player.clone();
        let gift_id = gift.id;
        let saved = spawn_db(move || db::wedding_gift::take(id, gift_id, &snapshot)).await;
        match saved {
            Ok(true) => {
                info!(self.client_id, gift_id, "Wedding gift taken");
                self.send(&mut added).await?;
            }
            Ok(false) => {
                // Gone already: only the list changes.
                self.in_game_mut()?.player = before;
            }
            Err(error) => {
                warn!(self.client_id, %error, "Couldn't take a wedding gift");
                self.in_game_mut()?.player = before;
                return self.send(&mut build_gift_not_taken()?).await;
            }
        }
        gifts.remove(at);
        let listed = gifts.iter().map(|gift| &gift.item);
        self.send(&mut build_gift_taken(bag, listed)?).await
    }
}
