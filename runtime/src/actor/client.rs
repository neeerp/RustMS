mod alliances;
mod buddies;
mod buff;
mod combat;
mod coupon;
mod dialogue;
mod door;
mod duey;
mod events;
mod expiry;
mod fame;
mod family;
mod guild_bbs;
mod guilds;
mod hazard;
mod hired_merchant;
mod later;
mod loot;
mod marriage;
mod mini_game;
pub(crate) mod name_change;
mod notes;
mod npc_script;
mod owl;
mod personal_shop;
mod pets;
mod quest;
mod rps;
mod script;
mod ships;
mod shop;
mod storage;
mod store_bank;
mod time_limit;
mod trade;
mod wedding;
mod wedding_day;
mod wedding_gift;

pub(crate) use wedding::leave_at_duey;

use crate::actor::field_merchants::MerchantRequest;
use crate::actor::registry::FieldRegistry;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::io::{PacketReader, PacketWriter};
use crate::message::{
    ClientHandle, DirectoryMessage, FamilyTrip, FieldHandle, FieldKey, FieldMessage, Presence,
    ServerMessage, SummonFailure, CLIENT_QUEUE_CAPACITY,
};
use db::cash_shop::CashType;
use game::ring::RingKind;
use game::{InventoryChange, InventoryType, Player};
use game_data::commodities::Commodity;
use net::cash_shop::{CashShopRequest, Expansion};
use net::dialogue::{Conversation, NpcInteraction};
use net::login_world::resolve_login_channel;
use net::packet::build;
use net::packet::build::world::field::{
    build_chalkboard, build_show_consume_effect, parse_movement_state,
};
use net::packet::build::world::map::{build_blocked_map, BlockedMap};
use net::packet::build::world::maple_tv::TvRefusal;
use net::packet::build::world::messaging::{build_find_result, FindLocation};
use net::packet::build::world::pet::PetGone;
use net::packet::handle::world::{
    spend_avatar_megaphone, spend_maple_tv, to_character, Rock, Whereabouts,
};
use net::packet::op::{RecvOpcode, SendOpcode};
use net::quests::QuestRequest;
use net::{ClientId, SpawnPosition, WorldAction, WorldContext, WorldRoute};
use packet::Packet;
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep_until, Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// How often a dirty player is written back while online.
const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(60);
/// Fallback stand position when a map has no usable spawn portal.
const FALLBACK_SPAWN: SpawnPosition = SpawnPosition {
    x: 0,
    y: 0,
    stance: 2,
};

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

/// Shared services a world client actor needs.
#[derive(Clone)]
pub struct WorldServices {
    pub directory: mpsc::Sender<DirectoryMessage>,
    pub fields: Arc<FieldRegistry>,
}

/// State that exists only after the client entered the game.
struct InGame {
    player: Player,
    field: FieldHandle,
    /// Where the player last stood on the current map, from their own
    /// movement; the field keeps the authoritative copy.
    position: (i16, i16),
}

/// Actor for one world-server connection. It is the single owner of its
/// character's state: fields and the directory only hold projections.
pub struct ClientActor {
    connection_id: u64,
    channel_id: u8,
    peer_ip: IpAddr,
    client_id: ClientId,
    reader: PacketReader,
    writer: PacketWriter,
    rx: mpsc::Receiver<ServerMessage>,
    tx: mpsc::Sender<ServerMessage>,
    kill: CancellationToken,
    services: WorldServices,
    session_id: Option<i32>,
    game: Option<InGame>,
    conversation: Option<Conversation>,
    /// A native NPC or quest script conversation.
    script: Option<npc_script::ActiveScript>,
    /// The NPC shop the player has open.
    shop: Option<net::shop::Shop>,
    /// The account's storage, while the player has it open.
    storage: Option<net::storage::OpenStorage>,
    /// The Duey window, while the player has it open.
    duey: Option<duey::DueyWindow>,
    /// Fredrick's Store Bank, while the player has it open.
    store_bank: Option<store_bank::StoreBankWindow>,
    /// The wedding registry window open, if any.
    gift_window: Option<wedding_gift::GiftWindow>,
    /// The Rock, Paper, Scissors game, while its window is open.
    rps: Option<net::rps::RpsGame>,
    /// When natural recovery of HP and of MP was last accepted; the client
    /// times them separately.
    last_heal: (Option<tokio::time::Instant>, Option<tokio::time::Instant>),
    /// When the player last sent movement; recovery needs them to rest.
    last_moved: Option<tokio::time::Instant>,
    /// A quest action waiting for its field to confirm the NPC is near.
    pending_quest: Option<(u64, QuestRequest)>,
    /// Portal scripts that already ran and won't run again this session.
    blocked_scripts: HashSet<String>,
    npc_request_id: u64,
    changing_channel: bool,
    /// The player left their field for the Cash Shop.
    in_cash_shop: bool,
    /// Wrong coupon codes in a row on this connection.
    coupon_strikes: u8,
    /// A Maple TV message is waiting to hear from the world's line; the
    /// item goes only then, so another is refused meanwhile.
    maple_tv_waiting: bool,
    /// The party the directory says the player is in; fields need it to
    /// share EXP.
    party_id: Option<i32>,
    /// What the directory last heard of the player.
    presence: Option<Presence>,
    /// The player's open Mystic Door.
    door: Option<door::OpenDoor>,
    map_timer: Option<time_limit::MapTimer>,
    /// The map that hurts the player now and then.
    hazard: Option<hazard::HazardTimer>,
    /// Packets to send later, soonest first.
    later: Vec<(Instant, Packet)>,
    /// The player's side of a trade they are in.
    trade: Option<trade::Trade>,
    /// The player's own shop, or the one they visit.
    personal_shop: Option<personal_shop::PersonalShop>,
    /// The Omok or match cards game the player sits at.
    mini_game: Option<mini_game::MiniGame>,
    /// The player's Omok and match cards records, once read.
    game_records: Option<[game::GameRecord; 2]>,
    hired_merchant: Option<hired_merchant::MerchantRoom>,
    /// The owners of the merchants the player asked to visit here and
    /// hasn't been let into yet.
    merchant_visits: Vec<i32>,
    /// The field of the merchant the owner came into from elsewhere with a
    /// Store Remote Controller, or is waiting to; merchant requests go
    /// there.
    merchant_field: Option<FieldHandle>,
    /// Heracle asked for a new guild's name; only then is one founded.
    naming_guild: bool,
    /// The player's ticket into the party event they are in.
    event: Option<crate::actor::event::EventTicket>,
    /// The player's ticket into their own wedding.
    wedding: Option<crate::actor::wedding::WeddingTicket>,
}

impl ClientActor {
    /// Set up encryption and send the handshake for a connection accepted on
    /// `channel_id`'s listener.
    pub async fn new(
        stream: TcpStream,
        peer_ip: IpAddr,
        channel_id: u8,
        services: WorldServices,
    ) -> Result<Self, RuntimeError> {
        let (recv_iv, send_iv) = {
            let mut rng = thread_rng();
            (rng.gen::<[u8; 4]>().to_vec(), rng.gen::<[u8; 4]>().to_vec())
        };

        let (read_half, write_half) = stream.into_split();
        let reader = PacketReader::new(read_half, &recv_iv);
        let mut writer = PacketWriter::new(write_half, &send_iv);
        let handshake = build::build_handshake_packet(&recv_iv, &send_iv)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        writer.send_handshake(&handshake.bytes).await?;

        let (tx, rx) = mpsc::channel(CLIENT_QUEUE_CAPACITY);
        Ok(Self {
            connection_id: NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed),
            channel_id,
            peer_ip,
            client_id: 0,
            reader,
            writer,
            rx,
            tx,
            kill: CancellationToken::new(),
            services,
            session_id: None,
            game: None,
            conversation: None,
            script: None,
            shop: None,
            storage: None,
            duey: None,
            store_bank: None,
            gift_window: None,
            rps: None,
            last_heal: (None, None),
            last_moved: None,
            pending_quest: None,
            blocked_scripts: HashSet::new(),
            npc_request_id: 0,
            changing_channel: false,
            in_cash_shop: false,
            coupon_strikes: 0,
            maple_tv_waiting: false,
            party_id: None,
            presence: None,
            door: None,
            map_timer: None,
            hazard: None,
            later: Vec::new(),
            trade: None,
            personal_shop: None,
            mini_game: None,
            game_records: None,
            hired_merchant: None,
            merchant_visits: Vec::new(),
            merchant_field: None,
            naming_guild: false,
            event: None,
            wedding: None,
        })
    }

    pub async fn run(mut self) {
        let mut autosave = interval(AUTOSAVE_INTERVAL);
        autosave.set_missed_tick_behavior(MissedTickBehavior::Delay);
        autosave.reset();
        let mut hunger = interval(pets::PET_HUNGER_INTERVAL);
        hunger.set_missed_tick_behavior(MissedTickBehavior::Delay);
        hunger.reset();
        let kill = self.kill.clone();

        let outcome = loop {
            let buffs_due = self.buffs_due();
            let door_due = self.door_due();
            let map_timer_due = self.map_timer_due();
            let hazard_due = self.hazard_due();
            let pets_due = self.pets_due();
            let items_due = self.items_due();
            let later_due = self.later_due();
            let family_buff_due = self.family_buff_due();
            let step = tokio::select! {
                biased;
                _ = kill.cancelled() => Err(RuntimeError::ClientDisconnected),
                result = self.reader.read_packet() => match result {
                    Ok(packet) => self.handle_packet(packet).await,
                    Err(error) => Err(error),
                },
                Some(message) = self.rx.recv() => self.handle_server_message(message).await,
                _ = autosave.tick() => self.save_if_dirty().await,
                _ = hunger.tick() => self.pets_grow_hungry().await,
                _ = sleep_until(pets_due.unwrap_or_else(Instant::now)), if pets_due.is_some() => {
                    self.pets_dry_up().await
                }
                _ = sleep_until(items_due.unwrap_or_else(Instant::now)), if items_due.is_some() => {
                    self.expire_items().await
                }
                _ = sleep_until(buffs_due.unwrap_or_else(Instant::now)), if buffs_due.is_some() => {
                    self.update_buffs().await
                }
                _ = sleep_until(door_due.unwrap_or_else(Instant::now)), if door_due.is_some() => {
                    self.close_door().await
                }
                _ = sleep_until(map_timer_due.unwrap_or_else(Instant::now)), if map_timer_due.is_some() => {
                    self.end_map_timer().await
                }
                _ = sleep_until(hazard_due.unwrap_or_else(Instant::now)), if hazard_due.is_some() => {
                    self.hurt_by_map().await
                }
                _ = sleep_until(later_due.unwrap_or_else(Instant::now)), if later_due.is_some() => {
                    self.send_later().await
                }
                _ = sleep_until(family_buff_due.unwrap_or_else(Instant::now)), if family_buff_due.is_some() => {
                    self.end_family_buff().await
                }
            };
            if let Err(error) = step {
                break error;
            }
            if let Err(error) = self.sync_presence().await {
                break error;
            }
        };

        match outcome {
            RuntimeError::ClientDisconnected => info!(self.client_id, "Client disconnected"),
            error => error!(self.client_id, %error, "Closing client connection"),
        }
        self.cleanup().await;
    }

    /// Write one packet to the socket. A kick interrupts a write that is
    /// blocked on a peer that stopped reading, and `PacketWriter` bounds it
    /// with a timeout either way, so the actor always reaches cleanup.
    async fn send(&mut self, packet: &mut Packet) -> Result<(), RuntimeError> {
        self.send_later_before(packet).await?;
        self.write(packet).await
    }

    async fn write(&mut self, packet: &mut Packet) -> Result<(), RuntimeError> {
        let kill = self.kill.clone();
        tokio::select! {
            biased;
            _ = kill.cancelled() => Err(RuntimeError::ClientDisconnected),
            result = self.writer.send_packet(packet) => result,
        }
    }

    fn handle(&self) -> ClientHandle {
        ClientHandle::new(
            self.client_id,
            self.connection_id,
            self.tx.clone(),
            self.kill.clone(),
        )
    }

    async fn handle_packet(&mut self, mut packet: Packet) -> Result<(), RuntimeError> {
        let opcode = packet.opcode();
        let Some(route) = net::world_route(opcode) else {
            debug!(opcode, "Ignoring unhandled world opcode");
            return Ok(());
        };
        // In the Cash Shop the player is off the map; only leaving, the shop
        // itself, its coupons, Name Change's checks and thanking for gifts
        // count.
        if self.in_cash_shop
            && opcode != RecvOpcode::ChangeMap as i16
            && opcode != RecvOpcode::CashShopOperation as i16
            && opcode != RecvOpcode::CheckCash as i16
            && opcode != RecvOpcode::CouponCode as i16
            && opcode != RecvOpcode::NoteAction as i16
            && opcode != RecvOpcode::NameChangeRules as i16
            && opcode != RecvOpcode::CheckCharName as i16
        {
            debug!(opcode, "Ignoring opcode in the Cash Shop");
            return Ok(());
        }
        // Checked before the handler moves the player.
        if opcode == RecvOpcode::ChangeMap as i16 && self.gate_closed(&packet) {
            return self
                .send(&mut build_blocked_map(BlockedMap::PortalClosed)?)
                .await;
        }

        let result = match (route, self.game.as_mut()) {
            (WorldRoute::PreGame(handler), None) => handler.handle(&mut packet),
            (WorldRoute::InGame(handler), Some(game)) => {
                let mut ctx = WorldContext {
                    client_id: self.client_id,
                    channel_id: self.channel_id,
                    player: &mut game.player,
                };
                handler.handle(&mut packet, &mut ctx)
            }
            _ => {
                warn!(opcode, "Opcode not valid in the current session stage");
                return Ok(());
            }
        };

        let result = result.map_err(|e| {
            error!(opcode, error = %e, "Handler error");
            RuntimeError::Handler(e.to_string())
        })?;
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }

    async fn apply(&mut self, action: WorldAction) -> Result<(), RuntimeError> {
        match action {
            WorldAction::Reply(mut packet) => {
                // A remote visit ends before the client leaves the map.
                if packet.opcode() == SendOpcode::SetField as i16 {
                    self.stop_remote_merchant().await?;
                }
                self.send(&mut packet).await?
            }
            WorldAction::ReplyLater { after, packet } => self.reply_later(after, packet),
            WorldAction::EnterGame { character_id } => self.enter_game(character_id).await?,
            WorldAction::ChangeField { map_id, spawn } => self.change_field(map_id, spawn).await?,
            WorldAction::FieldChat(packet) => {
                self.send_to_field(FieldMessage::Chat {
                    from: self.client_id,
                    packet,
                })
                .await?
            }
            WorldAction::FieldMove {
                packet,
                start,
                movement_bytes,
            } => {
                self.last_moved = Some(tokio::time::Instant::now());
                if let Some(game) = self.game.as_mut() {
                    if let Some((x, y, _)) =
                        parse_movement_state(&movement_bytes, (start.0, start.1, 0))
                    {
                        game.position = (x, y);
                    }
                }
                self.send_to_field(FieldMessage::Move {
                    from: self.client_id,
                    packet,
                    start,
                    movement_bytes,
                })
                .await?
            }
            WorldAction::NpcInteraction(NpcInteraction::Start { object_id, .. }) => {
                self.conversation = None;
                self.script = None;
                self.shop = None;
                self.storage = None;
                self.store_bank = None;
                self.npc_request_id = self.npc_request_id.wrapping_add(1);
                self.send_to_field(FieldMessage::NpcInteraction {
                    from: self.client_id,
                    object_id,
                    request_id: self.npc_request_id,
                })
                .await?
            }
            WorldAction::NpcInteraction(NpcInteraction::WaterOfLife) => {
                let player = &self.in_game()?.player;
                if let Some(script) = net::npc_scripts::NpcScript::water_of_life(player) {
                    self.start_script(script).await?
                }
            }
            WorldAction::NpcInteraction(NpcInteraction::Respond {
                message_type,
                response,
                selection,
                text,
            }) => {
                if !self
                    .respond_to_script(message_type, response, selection, text)
                    .await?
                {
                    self.respond_to_npc(message_type, response).await?
                }
            }
            WorldAction::NaturalHeal { hp, mp } => self.natural_heal(hp, mp).await?,
            WorldAction::Field(request) => {
                self.send_to_field(FieldMessage::FromClient {
                    from: self.client_id,
                    request,
                })
                .await?
            }
            WorldAction::Died => self.died().await?,
            WorldAction::GainExp(amount) => self.gained_exp(amount).await?,
            WorldAction::Quest(request) => self.request_quest_action(request).await?,
            WorldAction::Shop(request) => self.shop_action(request).await?,
            WorldAction::MiuMiu { slot } => self.miu_miu(slot).await?,
            WorldAction::ScriptedItem { script } => self.use_scripted_item(&script).await?,
            WorldAction::Storage(request) => self.storage_request(request).await?,
            WorldAction::Duey(_) if self.in_cash_shop => {}
            WorldAction::Duey(request) => self.duey_request(request).await?,
            WorldAction::WeddingGift(request) => self.gift_request(request).await?,
            WorldAction::Rps(request) => self.rps_request(request).await?,
            WorldAction::Trade(_) if self.in_cash_shop => {}
            WorldAction::Trade(request) if self.hired_merchant.is_some() => {
                self.merchant_room_request(request).await?
            }
            WorldAction::Trade(request) if self.personal_shop.is_some() => {
                self.shop_room_request(request).await?
            }
            WorldAction::Trade(request) if self.mini_game.is_some() => {
                self.game_room_request(request).await?
            }
            WorldAction::Trade(request) => self.trade_request(request).await?,
            WorldAction::MiniGame(_) if self.in_cash_shop => {}
            WorldAction::MiniGame(request) => self.mini_game_request(request).await?,
            WorldAction::Owl(_) if self.in_cash_shop => {}
            WorldAction::Owl(request) => self.owl_request(request).await?,
            WorldAction::PersonalShop(_) if self.in_cash_shop => {}
            WorldAction::PersonalShop(request) if self.hired_merchant.is_some() => {
                self.merchant_shop_request(request).await?
            }
            WorldAction::PersonalShop(request) => self.personal_shop_request(request).await?,
            WorldAction::HiredMerchant(_) if self.in_cash_shop => {}
            WorldAction::HiredMerchant(request) => self.hired_merchant_request(request).await?,
            WorldAction::StoreBank(_) if self.in_cash_shop => {}
            WorldAction::StoreBank(request) => self.store_bank_request(request).await?,
            WorldAction::ScriptPortal { portal } => self.touch_script_portal(&portal).await?,
            WorldAction::LookChanged(announce) => {
                let player = &self.in_game()?.player;
                let avatar = player.field_avatar(0, 0, 0);
                let look = player.look();
                self.send_to_field(FieldMessage::UpdateAvatar {
                    client_id: self.client_id,
                    avatar,
                    announce: vec![announce],
                })
                .await?;
                self.send_to_directory(DirectoryMessage::LookChanged {
                    client_id: self.client_id,
                    look,
                })
                .await?
            }
            WorldAction::BuffsChanged(announce) => self.show_buffs(announce).await?,
            WorldAction::ChalkboardChanged(announce)
            | WorldAction::ItemEffectChanged(announce)
            | WorldAction::BerserkChanged(announce) => {
                let avatar = self.in_game()?.player.field_avatar(0, 0, 0);
                self.send_to_field(FieldMessage::UpdateAvatar {
                    client_id: self.client_id,
                    avatar,
                    announce: vec![announce],
                })
                .await?
            }
            WorldAction::HiddenChanged => {
                let avatar = self.in_game()?.player.field_avatar(0, 0, 0);
                self.send_to_field(FieldMessage::UpdateAvatar {
                    client_id: self.client_id,
                    avatar,
                    announce: Vec::new(),
                })
                .await?
            }
            WorldAction::PetsChanged(changes) => {
                self.pets_changed(changes, PetGone::Dismissed).await?
            }
            WorldAction::Whisper(delivery) => {
                self.send_to_directory(DirectoryMessage::Whisper {
                    from: self.client_id,
                    delivery,
                })
                .await?
            }
            WorldAction::Party(request) => {
                self.send_to_directory(DirectoryMessage::Party {
                    from: self.client_id,
                    request,
                })
                .await?
            }
            WorldAction::PartySkill(used) => {
                let origin = self.in_game()?.position;
                self.send_to_directory(DirectoryMessage::PartySkill {
                    from: self.client_id,
                    used,
                    origin,
                })
                .await?
            }
            WorldAction::Buddy(request) => self.buddy_request(request).await?,
            WorldAction::Note(request) => self.note_request(request).await?,
            WorldAction::Guild(request) => self.guild_request(request).await?,
            WorldAction::Family(request) => self.family_request(request).await?,
            WorldAction::Ring(request) => self.ring_request(request).await?,
            WorldAction::WeddingTalk { step } => self.wedding_talk(step),
            WorldAction::WeddingBless => self.bless_couple().await?,
            WorldAction::Alliance(request) => self.alliance_request(request).await?,
            WorldAction::GiveFame { target, up } => self.give_fame(target, up).await?,
            WorldAction::OpenDoor { town, at, lasts } => self.open_door(town, at, lasts).await?,
            WorldAction::UseDoor { owner, from_town } => self.use_door(owner, from_town).await?,
            WorldAction::RockToCharacter { name, rock } => {
                self.send_to_directory(DirectoryMessage::RockLocate {
                    from: self.client_id,
                    name,
                    rock,
                })
                .await?
            }
            WorldAction::MorphAnother(potion) => {
                self.send_to_directory(DirectoryMessage::MorphAnother {
                    from: self.client_id,
                    potion,
                })
                .await?
            }
            WorldAction::Find { target_name } => {
                self.send_to_directory(DirectoryMessage::Find {
                    from: self.client_id,
                    target_name,
                })
                .await?
            }
            WorldAction::Broadcast { scope, packet } => {
                self.send_to_directory(DirectoryMessage::Broadcast {
                    from: self.client_id,
                    scope,
                    packet,
                })
                .await?
            }
            WorldAction::AvatarMegaphone {
                slot,
                item_id,
                banner,
            } => {
                let (reply, queued) = tokio::sync::oneshot::channel();
                self.send_to_directory(DirectoryMessage::AvatarBanner { banner, reply })
                    .await?;
                let queued = queued.await.unwrap_or(false);
                let player = &mut self.in_game_mut()?.player;
                let mut packet = spend_avatar_megaphone(player, slot, item_id, queued)?;
                self.send(&mut packet).await?
            }
            WorldAction::MapleTv { slot, tv } => {
                if self.maple_tv_waiting {
                    let mut unlock = net::packet::build::world::stat::build_stat_update(true, &[])?;
                    return self.send(&mut unlock).await;
                }
                self.maple_tv_waiting = true;
                let item_id = tv.item_id;
                let (reply, queued) = tokio::sync::oneshot::channel();
                self.send_to_directory(DirectoryMessage::MapleTv {
                    tv,
                    partner: None,
                    reply,
                })
                .await?;
                // Waited for apart: the line may wait on the receiver's
                // actor, which may be this one or be waiting on this one.
                let me = self.handle();
                tokio::spawn(async move {
                    // A dropped reply: the receiver went before adding
                    // their look.
                    let queued = queued.await.unwrap_or(Err(TvRefusal::WrongName));
                    me.send(ServerMessage::MapleTvQueued {
                        slot,
                        item_id,
                        queued,
                    });
                });
            }
            WorldAction::Messenger(request) => {
                self.send_to_directory(DirectoryMessage::Messenger {
                    from: self.client_id,
                    request,
                })
                .await?
            }
            WorldAction::ChangeChannel { channel_id } => self.change_channel(channel_id).await?,
            WorldAction::EnterCashShop => self.enter_cash_shop().await?,
            WorldAction::LeaveCashShop => {
                if self.in_cash_shop {
                    // Like a channel change to this channel: the client
                    // reconnects and enters the map it left.
                    self.change_channel(self.channel_id).await?
                }
            }
            WorldAction::CashShop(request) => {
                if self.in_cash_shop {
                    self.cash_shop(request).await?
                }
            }
            WorldAction::CancelNameChange { slot } => self.cancel_name_change(slot).await?,
            WorldAction::Disconnect => return Err(RuntimeError::ClientDisconnected),
        }
        Ok(())
    }

    /// Claim the login server's single-use ticket for this channel, load the
    /// character, and join the world.
    async fn enter_game(&mut self, character_id: i32) -> Result<(), RuntimeError> {
        if self.game.is_some() || self.session_id.is_some() {
            warn!(character_id, "Ignoring repeated world entry");
            return Ok(());
        }

        let (channel, ip) = (i16::from(self.channel_id), self.peer_ip.into());
        let session =
            match spawn_db(move || db::session::claim_world_transition(character_id, channel, ip))
                .await
            {
                Ok(session) => session,
                Err(RuntimeError::Database(db::Error::NotFound)) => {
                    warn!(
                        character_id,
                        "World entry without a valid transition ticket"
                    );
                    return Err(RuntimeError::NotLoggedIn);
                }
                Err(error) => return Err(error),
            };
        self.session_id = Some(session.id);

        let mut player = spawn_db(move || db::player::load_player(character_id)).await?;
        if player.account_id != session.account_id {
            return Err(RuntimeError::NotLoggedIn);
        }
        // Heart Keys are a wedding's own, and nobody logs in to one: a
        // crash may have saved them with the player in it.
        if game::wedding::wedding_map(player.stats.map_id) {
            game::wedding::take_keys(&mut player);
        }
        ships::leave_forced_return_map(&mut player);
        // The client learns of pets that dried up meanwhile with the rest.
        net::packet::handle::world::pets_dry_up(&mut player, SystemTime::now());
        // Items that expired meanwhile are gone before the client sees them;
        // it says which once in the game.
        let expired = net::items::expire_items(&mut player, SystemTime::now());
        // No buff lasts past a logout or a channel change, and gear may have
        // gone: HP and MP come back within the maximum without them.
        player.fit_hp_mp();
        self.client_id = character_id;

        let mut char_info = build::world::char::build_char_info(&player, self.channel_id)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        let mut keymap = build::world::keymap::build_keymap(&player.keymap)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        // Sent even with no macros: the client keeps the last list it had.
        let mut macros = build::world::skill_macro::build_skill_macros(&player.skill_macros)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        let [mut auto_hp, mut auto_mp] =
            build::world::keymap::build_auto_potions(player.auto_potions)
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        let mut quick_slots = build::world::quick_slots::build_quick_slot_init(player.quick_slots)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        // The key map, the pet's potions and the quick slots reach only a
        // field's stage, so a v83 client drops them before SET_FIELD.
        self.send(&mut char_info).await?;
        self.send(&mut keymap).await?;
        self.send(&mut macros).await?;
        self.send(&mut auto_hp).await?;
        self.send(&mut auto_mp).await?;
        // Sent even with none set, as flag 0 puts back the defaults; the
        // client otherwise keeps the last character's layout.
        self.send(&mut quick_slots).await?;
        // The character data carries no master level for Blessing of the
        // Fairy, so its tooltip's "[Master Level : 20]" line needs it again.
        let blessing = game::skill::fairy_blessing(player.stats.job)
            .and_then(|skill_id| player.skills.get(skill_id));
        if let Some(record) = blessing {
            let mut update = build::world::skill::build_update_skill(record)
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            self.send(&mut update).await?;
        }
        if !expired.item_ids.is_empty() {
            let mut lines = build::world::inventory::build_items_expired(&expired.item_ids)
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            self.send(&mut lines).await?;
        }
        if !expired.released.is_empty() {
            let mut lines = build::world::inventory::build_seals_expired(&expired.released)
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            self.send(&mut lines).await?;
        }

        let presence = Presence::of(&player);
        self.presence = Some(presence.clone());
        self.send_to_directory(DirectoryMessage::Register {
            handle: self.handle(),
            presence,
            channel_id: self.channel_id,
        })
        .await?;

        let key = FieldKey::new(self.channel_id, player.stats.map_id);
        let spawn = net::spawn_position(player.stats.map_id, player.stats.spawn_point)
            .unwrap_or(FALLBACK_SPAWN);
        let field = self.services.fields.field(key);
        self.game = Some(InGame {
            player,
            field,
            position: (spawn.x, spawn.y),
        });
        self.join_current_field(spawn).await?;
        self.show_mount().await?;
        self.load_buddies().await?;
        self.load_guild().await?;
        self.load_family().await?;
        if self.in_game()?.player.family_buff.is_some() {
            self.show_family_buff().await?;
        }
        self.load_notes().await?;
        self.announce_parcels().await?;
        info!(
            character_id,
            channel_id = self.channel_id,
            "Entered the game"
        );
        Ok(())
    }

    /// Move field membership after the player changed maps.
    async fn change_field(
        &mut self,
        map_id: i32,
        spawn: Option<SpawnPosition>,
    ) -> Result<(), RuntimeError> {
        self.conversation = None;
        self.script = None;
        self.shop = None;
        self.storage = None;
        self.store_bank = None;
        // A new field takes the Duey window with it, and the client doesn't
        // say so (report b029).
        self.duey = None;
        self.gift_window = None;
        self.rps = None;
        self.pending_quest = None;
        self.naming_guild = false;
        self.npc_request_id = self.npc_request_id.wrapping_add(1);
        self.in_game_mut()?.player.chair = 0;

        self.leave_trade().await?;
        self.leave_personal_shop().await?;
        self.leave_mini_game().await?;
        self.stop_remote_merchant().await?;
        self.leave_hired_merchant().await?;
        self.keep_mounts_off(map_id).await?;
        self.drop_beacon().await?;
        let staying = self.in_game()?.field.key == self.field_key(map_id);
        self.send_to_field(FieldMessage::Leave {
            client_id: self.client_id,
            staying,
        })
        .await?;
        self.leaving_for(map_id).await?;
        let key = self.field_key(map_id);
        let field = self.services.fields.field(key);
        let game = self.in_game_mut()?;
        game.field = field;
        let spawn = spawn
            .or_else(|| net::spawn_position(map_id, game.player.stats.spawn_point))
            .unwrap_or(FALLBACK_SPAWN);
        self.join_current_field(spawn).await
    }

    async fn join_current_field(&mut self, spawn: SpawnPosition) -> Result<(), RuntimeError> {
        let handle = self.handle();
        let game = self.in_game_mut()?;
        game.position = (spawn.x, spawn.y);
        // The client puts a chalkboard away in the Free Market rooms.
        if net::personal_shop::allowed_on(game.player.stats.map_id) {
            game.player.chalkboard = None;
        }
        let avatar = game.player.field_avatar(spawn.x, spawn.y, spawn.stance);
        let party = self.party_id;
        self.send_to_field(FieldMessage::Join {
            handle,
            avatar: avatar.clone(),
            party,
        })
        .await?;
        self.set_wall_clock().await?;
        self.start_map_timer().await?;
        self.show_event_clock().await?;
        self.show_event_gate().await?;
        self.start_hazard()?;
        self.bring_summons(spawn).await?;
        self.bring_pets(&avatar).await?;
        if let Some(note) = &avatar.chalkboard {
            // The player's client shows its own board only when told.
            self.send(&mut build_chalkboard(avatar.id, Some(note))?)
                .await?;
        }
        if avatar.consume_effect != 0 {
            // So is its own item buff's aura: v83 makes the user anew.
            self.send(&mut build_show_consume_effect(
                avatar.id,
                avatar.consume_effect,
            )?)
            .await?;
        }
        self.show_ghost().await?;
        self.run_map_enter_script().await
    }

    /// Hand the session to another channel: persist, reissue the ticket, and
    /// tell the client where to reconnect.
    async fn change_channel(&mut self, channel_id: u8) -> Result<(), RuntimeError> {
        let world_id = self.in_game()?.player.world_id;
        let channel = resolve_login_channel(world_id, channel_id)
            .map_err(|e| RuntimeError::Handler(format!("Unknown channel: {e}")))?;
        let session_id = self.session_id.ok_or(RuntimeError::NotLoggedIn)?;

        self.remember_spawn_point();
        self.cancel_buffs().await?;
        self.close_door().await?;
        self.leave_trade().await?;
        self.leave_personal_shop().await?;
        self.leave_mini_game().await?;
        self.leave_hired_merchant().await?;
        self.flush().await?;
        spawn_db(move || db::session::begin_channel_transition(session_id, i16::from(channel_id)))
            .await?;
        self.changing_channel = true;

        let mut redirect = build::world::channel::build_channel_change(channel.host, channel.port)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut redirect).await?;
        Err(RuntimeError::ClientDisconnected)
    }

    /// Take the player off their field and into the Cash Shop. The session
    /// stays on this channel, so whispers still reach them.
    async fn enter_cash_shop(&mut self) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        self.conversation = None;
        self.script = None;
        self.pending_quest = None;
        self.naming_guild = false;
        // The shop is a stage of its own, and takes the Duey window with it
        // (report b029).
        self.duey = None;
        self.store_bank = None;
        self.gift_window = None;
        self.rps = None;
        self.npc_request_id = self.npc_request_id.wrapping_add(1);
        self.leave_trade().await?;
        self.leave_personal_shop().await?;
        self.leave_mini_game().await?;
        self.leave_hired_merchant().await?;
        // The shop can't answer a proposal (report b053).
        self.send_to_directory(DirectoryMessage::EnteredCashShop {
            from: self.client_id,
        })
        .await?;
        self.send_to_field(FieldMessage::Leave {
            client_id: self.client_id,
            staying: false,
        })
        .await?;
        self.in_cash_shop = true;
        // The shop sends the whole inventory, and the packets were for windows
        // the shop closes.
        self.later.clear();
        self.flush().await?;

        let player = &self.in_game()?.player;
        let (account_id, character_id) = (player.account_id, player.stats.id);
        let (account, balance, (items, gifts), wish_list, best_sellers) = spawn_db(move || {
            Ok((
                db::account::get_account_by_id(account_id)?,
                db::cash_shop::balance(account_id)?,
                db::cash_shop::open_cash_inventory(account_id)?,
                db::cash_shop::wish_list(character_id)?,
                db::cash_shop::best_sellers(BEST_SELLERS_CONSIDERED)?,
            ))
        })
        .await?;
        let player = &self.in_game()?.player;
        let best = net::cash_shop::best_items(&best_sellers);
        let mut packets = [
            build::world::cash_shop::build_open_cash_shop(player, &account.user_name, &best)?,
            build::world::cash_shop::build_cash_inventory(
                &items,
                account.storage_slots,
                account.character_slots,
            )?,
            build::world::cash_shop::build_cash_gifts(&gifts)?,
            build::world::cash_shop::build_wish_list(&wish_list, false)?,
            build::world::cash_shop::build_cash_balance(
                balance.credit,
                balance.points,
                balance.prepaid,
            )?,
        ];
        self.send_all(&mut packets).await?;
        info!(self.client_id, "Entered the Cash Shop");
        Ok(())
    }

    /// Buy a commodity into the account's Cash Inventory, or save the wish
    /// list. The handler already checked the commodity is for sale.
    async fn cash_shop(&mut self, request: CashShopRequest) -> Result<(), RuntimeError> {
        use build::world::cash_shop as cs;
        let player = &self.in_game()?.player;
        let (account_id, character_id) = (player.account_id, player.stats.id);
        let mut packets = match request {
            CashShopRequest::Buy {
                cash_type,
                commodity,
            } => {
                let package = net::cash_shop::package(&commodity);
                let contents = package.clone().unwrap_or_else(|| vec![&commodity]);
                let items: Vec<_> = contents
                    .iter()
                    .map(|item| db::cash_shop::NewCashItem {
                        account_id,
                        item_id: item.item_id,
                        sn: item.sn,
                        quantity: item.count,
                        expires_at: expiry(item.item_id, item.period),
                    })
                    .collect();
                let (sn, price) = (commodity.sn, commodity.price);
                let bought =
                    spawn_db(move || db::cash_shop::buy(account_id, cash_type, sn, price, &items))
                        .await?;
                match bought {
                    Some((bought, balance)) => {
                        info!(
                            self.client_id,
                            sn,
                            item_id = commodity.item_id,
                            price,
                            "Bought from the Cash Shop"
                        );
                        let bought = match (&package, bought.as_slice()) {
                            (None, [item]) => cs::build_bought_cash_item(item)?,
                            _ => cs::build_bought_package(&bought)?,
                        };
                        vec![
                            bought,
                            cs::build_cash_balance(
                                balance.credit,
                                balance.points,
                                balance.prepaid,
                            )?,
                        ]
                    }
                    None => vec![cs::build_cash_shop_failure(
                        cs::CashShopFailure::NotEnoughCash,
                    )?],
                }
            }
            CashShopRequest::Gift {
                commodity,
                to,
                message,
            } => self.send_gift(account_id, commodity, to, message).await?,
            CashShopRequest::BuyRing {
                cash_type,
                commodity,
                to,
                message,
            } => self.buy_ring(cash_type, commodity, to, message).await?,
            CashShopRequest::SetWishList(sns) => {
                let saved = sns.clone();
                spawn_db(move || db::cash_shop::set_wish_list(character_id, &saved)).await?;
                vec![cs::build_wish_list(&sns, true)?]
            }
            CashShopRequest::TakeOut { cash_id, slot } => {
                self.take_out_cash_item(account_id, cash_id, slot).await?
            }
            CashShopRequest::PutBack { cash_id, kind } => {
                self.put_back_cash_item(account_id, cash_id, kind).await?
            }
            CashShopRequest::Expand {
                cash_type,
                expansion: Expansion::Bag(kind),
                slots,
                price,
            } => self.expand_bag(cash_type, kind, slots, price).await?,
            CashShopRequest::Expand {
                cash_type,
                expansion: Expansion::Storage,
                slots,
                price,
            } => {
                self.expand_storage(account_id, cash_type, slots, price)
                    .await?
            }
            CashShopRequest::BuyCharacterSlot {
                cash_type,
                commodity,
            } => {
                self.buy_character_slot(account_id, cash_type, commodity)
                    .await?
            }
            CashShopRequest::Coupon(code) => self.redeem_coupon(code).await?,
            CashShopRequest::NameChangeRules => self.name_change_rules().await?,
            CashShopRequest::CheckNewName(name) => self.check_new_name(name).await?,
            CashShopRequest::RequestNameChange {
                commodity,
                new_name,
            } => self.request_name_change(commodity, new_name).await?,
            CashShopRequest::BuyWithMesos(commodity) => {
                self.buy_with_mesos(account_id, commodity).await?
            }
            CashShopRequest::Balance => {
                let balance = spawn_db(move || db::cash_shop::balance(account_id)).await?;
                vec![cs::build_cash_balance(
                    balance.credit,
                    balance.points,
                    balance.prepaid,
                )?]
            }
        };
        self.send_all(&mut packets).await
    }

    /// Pay for `commodity` and put it in the Cash Inventory of the account
    /// of the character named `to`, who sees it on next entering the shop.
    async fn send_gift(
        &mut self,
        account_id: i32,
        commodity: Commodity,
        to: String,
        message: String,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        let sender = self.in_game()?.player.stats.name.clone();
        let receiver = spawn_db(move || db::cash_shop::gift_receiver(&to)).await?;
        let failure = match &receiver {
            None => Some(cs::CashShopFailure::NoSuchCharacter),
            Some(receiver) if receiver.account_id == account_id => {
                Some(cs::CashShopFailure::OwnAccount)
            }
            Some(receiver) if !commodity.for_gender(receiver.gender) => {
                Some(cs::CashShopFailure::Gender)
            }
            Some(_) => None,
        };
        let Some(receiver) = receiver.filter(|_| failure.is_none()) else {
            return Ok(vec![cs::build_cash_shop_failure(
                failure.unwrap_or(cs::CashShopFailure::Unknown),
            )?]);
        };
        let gift = db::cash_shop::NewGift {
            account_id: receiver.account_id,
            item_id: commodity.item_id,
            sn: commodity.sn,
            quantity: commodity.count,
            expires_at: expiry(commodity.item_id, commodity.period),
            gift_from: sender,
            gift_message: Some(message),
        };
        let price = commodity.price;
        let Some(balance) = spawn_db(move || db::cash_shop::gift(account_id, price, &gift)).await?
        else {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::NotEnoughCash,
            )?]);
        };
        info!(
            self.client_id,
            sn = commodity.sn,
            to = receiver.name,
            price,
            "Sent a gift from the Cash Shop"
        );
        Ok(vec![
            cs::build_gift_sent(&receiver.name, commodity.item_id, commodity.count, price)?,
            cs::build_cash_balance(balance.credit, balance.points, balance.prepaid)?,
        ])
    }

    /// Pay for a crush or friendship ring and its partner, which goes to the
    /// character named `to` as a gift (report b061).
    async fn buy_ring(
        &mut self,
        cash_type: CashType,
        commodity: Commodity,
        to: String,
        message: String,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        let player = &self.in_game()?.player;
        let (account_id, buyer_id) = (player.account_id, player.stats.id);
        let buyer = player.stats.name.clone();
        let receiver = spawn_db(move || db::cash_shop::gift_receiver(&to)).await?;
        let failure = match &receiver {
            None => Some(cs::CashShopFailure::NoSuchCharacter),
            Some(receiver) if receiver.account_id == account_id => {
                Some(cs::CashShopFailure::OwnAccount)
            }
            Some(receiver) if !commodity.for_gender(receiver.gender) => {
                Some(cs::CashShopFailure::Gender)
            }
            Some(_) => None,
        };
        let Some(receiver) = receiver.filter(|_| failure.is_none()) else {
            return Ok(vec![cs::build_cash_shop_failure(
                failure.unwrap_or(cs::CashShopFailure::Unknown),
            )?]);
        };
        let expires_at = expiry(commodity.item_id, commodity.period);
        let ring = db::cash_shop::NewCashItem {
            account_id,
            item_id: commodity.item_id,
            sn: commodity.sn,
            quantity: commodity.count,
            expires_at,
        };
        let gift = db::cash_shop::NewGift {
            account_id: receiver.account_id,
            item_id: commodity.item_id,
            sn: commodity.sn,
            quantity: commodity.count,
            expires_at,
            gift_from: buyer,
            gift_message: Some(message),
        };
        let (price, receiver_id) = (commodity.price, receiver.id);
        let bought = spawn_db(move || {
            db::cash_shop::buy_ring(cash_type, price, (&ring, buyer_id), (&gift, receiver_id))
        })
        .await?;
        let Some((bought, balance)) = bought else {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::NotEnoughCash,
            )?]);
        };
        info!(
            self.client_id,
            sn = commodity.sn,
            to = receiver.name,
            price,
            "Bought a ring pair from the Cash Shop"
        );
        Ok(vec![
            cs::build_bought_ring(&bought, &receiver.name)?,
            cs::build_cash_balance(balance.credit, balance.points, balance.prepaid)?,
        ])
    }

    /// Pay for `slots` more slots in the bag of `kind`, up to 96.
    async fn expand_bag(
        &mut self,
        cash_type: CashType,
        kind: InventoryType,
        slots: u8,
        price: i32,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        let player = &self.in_game()?.player;
        let limit = player.inventory.slot_limit(kind) + slots;
        if limit > net::cash_shop::MAX_BAG_SLOTS {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::Unknown,
            )?]);
        }
        let mut snapshot = player.clone();
        snapshot.inventory_mut().set_slot_limit(kind, limit);
        let paid =
            spawn_db(move || db::cash_shop::expand_inventory(cash_type, price, &snapshot)).await?;
        let Some(balance) = paid else {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::NotEnoughCash,
            )?]);
        };
        let player = &mut self.in_game_mut()?.player;
        player.inventory_mut().set_slot_limit(kind, limit);
        player.mark_saved();
        info!(self.client_id, ?kind, limit, price, "Expanded a bag");
        Ok(vec![
            cs::build_inventory_slots(kind, limit)?,
            cs::build_cash_balance(balance.credit, balance.points, balance.prepaid)?,
        ])
    }

    /// Pay for `slots` more slots in the account's storage, up to 48.
    async fn expand_storage(
        &mut self,
        account_id: i32,
        cash_type: CashType,
        slots: u8,
        price: i32,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        let account = spawn_db(move || db::account::get_account_by_id(account_id)).await?;
        let limit = account.storage_slots + i16::from(slots);
        if limit > net::cash_shop::MAX_STORAGE_SLOTS {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::Unknown,
            )?]);
        }
        let paid =
            spawn_db(move || db::cash_shop::expand_storage(account_id, cash_type, price, limit))
                .await?;
        let Some(balance) = paid else {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::NotEnoughCash,
            )?]);
        };
        info!(self.client_id, limit, price, "Expanded the storage");
        Ok(vec![
            cs::build_storage_slots(limit)?,
            cs::build_cash_balance(balance.credit, balance.points, balance.prepaid)?,
        ])
    }

    /// Pay mesos for the quest item `commodity` and put it in the bags; it
    /// is saved with the character, as an NPC shop's goods are. v83 sees the
    /// meso and bag changes first, then where the item went.
    async fn buy_with_mesos(
        &mut self,
        account_id: i32,
        commodity: Commodity,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        let player = &mut self.in_game_mut()?.player;
        let changes = match net::cash_shop::buy_with_mesos(player, &commodity) {
            Ok(changes) => changes,
            Err(failure) => return Ok(vec![cs::build_cash_shop_failure(failure)?]),
        };
        let meso = player.meso;
        let slot = changes
            .iter()
            .rev()
            .find_map(|change| match change {
                InventoryChange::Add { slot, .. } | InventoryChange::Quantity { slot, .. } => {
                    Some(*slot)
                }
                _ => None,
            })
            .unwrap_or(0);
        info!(
            self.client_id,
            sn = commodity.sn,
            item_id = commodity.item_id,
            price = commodity.price,
            "Bought from the Cash Shop with mesos"
        );
        let balance = spawn_db(move || db::cash_shop::balance(account_id)).await?;
        Ok(vec![
            build::world::stat::build_stat_update(
                false,
                &[build::world::stat::StatUpdate::Meso(meso)],
            )?,
            build::world::inventory::build_inventory_operation(true, &changes)?,
            cs::build_bought_with_mesos(commodity.item_id, commodity.count, slot)?,
            cs::build_cash_balance(balance.credit, balance.points, balance.prepaid)?,
        ])
    }

    /// Pay for the Extra Character Slot Coupon `commodity` and give the
    /// account a character slot, which its next character list shows.
    async fn buy_character_slot(
        &mut self,
        account_id: i32,
        cash_type: CashType,
        commodity: Commodity,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        use db::cash_shop::SlotPurchase;
        let (sn, price) = (commodity.sn, commodity.price);
        let max = net::cash_shop::MAX_CHARACTER_SLOTS;
        let bought = spawn_db(move || {
            db::cash_shop::buy_character_slot(account_id, cash_type, sn, price, max)
        })
        .await?;
        let failure = match bought {
            SlotPurchase::Bought { slots, balance } => {
                info!(self.client_id, slots, price, "Bought a character slot");
                return Ok(vec![
                    cs::build_character_slots(slots)?,
                    cs::build_cash_balance(balance.credit, balance.points, balance.prepaid)?,
                ]);
            }
            // v83 asks for none past 15 (0x1ed), so this one is stale.
            SlotPurchase::Full => cs::CashShopFailure::Unknown,
            SlotPurchase::Short => cs::CashShopFailure::NotEnoughCash,
        };
        Ok(vec![cs::build_cash_shop_failure(failure)?])
    }

    /// Move a Cash Inventory item into its bag, at `slot` if that is free.
    async fn take_out_cash_item(
        &mut self,
        account_id: i32,
        cash_id: i64,
        slot: i16,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        let cash_item = spawn_db(move || db::cash_shop::cash_item(account_id, cash_id)).await?;
        let Some(mut item) = cash_item.as_ref().and_then(net::cash_shop::inventory_item) else {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::Unknown,
            )?]);
        };
        if item.pet.is_some() {
            if let Some(pet) = spawn_db(move || db::pet::find(cash_id)).await? {
                item.pet = Some(pet);
            }
        }
        let ring = match game::ring::kind(item.item_id) {
            Some(RingKind::Crush | RingKind::Friendship) => {
                spawn_db(move || db::ring::find(cash_id)).await?
            }
            _ => None,
        };
        let player = &mut self.in_game_mut()?.player;
        // The client refuses a second one-of-a-kind item itself (0xb94).
        if net::items::refuses_only(&player.inventory, item.item_id, item.quantity.into()) {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::InventoryFull,
            )?]);
        }
        let before = player.inventory.clone();
        let slot = match player.inventory_mut().place(item.clone(), slot) {
            Ok((_, slot)) => slot,
            Err(_) => {
                return Ok(vec![cs::build_cash_shop_failure(
                    cs::CashShopFailure::InventoryFull,
                )?])
            }
        };
        let snapshot = player.clone();
        let taken = spawn_db(move || db::cash_shop::take_out(account_id, cash_id, &snapshot)).await;
        let player = &mut self.in_game_mut()?.player;
        match taken {
            Ok(true) => {
                player.mark_saved();
                player.rings.extend(ring);
                info!(
                    self.client_id,
                    cash_id,
                    item_id = item.item_id,
                    slot,
                    "Took out a cash item"
                );
                Ok(vec![cs::build_taken_out(slot, &item)?])
            }
            Ok(false) => {
                *player.inventory_mut() = before;
                Ok(vec![cs::build_cash_shop_failure(
                    cs::CashShopFailure::Unknown,
                )?])
            }
            Err(error) => {
                *player.inventory_mut() = before;
                Err(error)
            }
        }
    }

    /// Move a character's cash item back to the account's Cash Inventory.
    async fn put_back_cash_item(
        &mut self,
        account_id: i32,
        cash_id: i64,
        kind: InventoryType,
    ) -> Result<Vec<Packet>, RuntimeError> {
        use build::world::cash_shop as cs;
        let player = &mut self.in_game_mut()?.player;
        let before = player.inventory.clone();
        let Some((_, item)) = player.inventory_mut().pull_cash_item(kind, cash_id) else {
            return Ok(vec![cs::build_cash_shop_failure(
                cs::CashShopFailure::Unknown,
            )?]);
        };
        let pets_before = player.pets.clone();
        player.pets_mut().retain(|&serial| serial != cash_id);
        let cash_item = net::cash_shop::cash_item(account_id, cash_id, &item);
        let snapshot = player.clone();
        let saved = cash_item.clone();
        let put = spawn_db(move || db::cash_shop::put_back(&saved, &snapshot)).await;
        let player = &mut self.in_game_mut()?.player;
        match put {
            Ok(()) => {
                player.mark_saved();
                player.rings.retain(|ring| ring.id != cash_id);
                info!(
                    self.client_id,
                    cash_id,
                    item_id = item.item_id,
                    "Put back a cash item"
                );
                Ok(vec![cs::build_put_back(&cash_item)?])
            }
            Err(error) => {
                *player.inventory_mut() = before;
                player.pets = pets_before;
                Err(error)
            }
        }
    }

    async fn handle_server_message(&mut self, message: ServerMessage) -> Result<(), RuntimeError> {
        match message {
            ServerMessage::NpcConversation {
                npc_id,
                field,
                request_id,
            } => {
                if request_id == self.npc_request_id {
                    self.start_npc_conversation(npc_id, field).await?;
                }
            }
            ServerMessage::QuestNpcChecked { request_id, nearby } => {
                self.quest_npc_checked(request_id, nearby).await?
            }
            ServerMessage::MobKilled {
                mob_id,
                object_id,
                exp,
                party_bonus,
                looter,
                x,
                y,
                showdown,
            } => {
                self.mob_killed(mob_id, exp, party_bonus).await?;
                self.count_quest_kill(mob_id).await?;
                if looter {
                    self.drop_loot(mob_id, showdown, object_id, x, y).await?
                }
            }
            ServerMessage::Stole {
                mob_id,
                object_id,
                x,
                y,
            } => self.drop_stolen(mob_id, object_id, x, y).await?,
            ServerMessage::PartyChanged { party_id } => {
                self.party_id = party_id;
                self.send_to_field(FieldMessage::SetParty {
                    client_id: self.client_id,
                    party: party_id,
                })
                .await?;
                self.left_party_in_event().await?
            }
            ServerMessage::EventStarted(ticket) => self.event_started(ticket).await?,
            ServerMessage::EventEnded { id } => self.event_ended(id).await?,
            ServerMessage::EventFailed { id } => self.event_failed(id).await,
            ServerMessage::LeaderHandedOver { to } => self.handed_over(to).await,
            ServerMessage::StageCleared { id, stage } => self.stage_cleared(id, stage).await?,
            ServerMessage::ClearEventStage(stage) => {
                self.tell_event(net::events::EventCommand::ClearStage(stage))
                    .await
            }
            ServerMessage::BossSummoned => self.tell_closing_time(Some(true)).await?,
            ServerMessage::MobBuff(item_id) => {
                let alive = self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.player.stats.hp > 0);
                if alive && !self.in_cash_shop {
                    self.take_item_effect(item_id, false).await?;
                }
            }
            ServerMessage::RestartEventClock => {
                self.tell_event(net::events::EventCommand::RestartClock)
                    .await
            }
            ServerMessage::EventClock { id, clock } => self.event_clock(id, clock).await?,
            ServerMessage::EventMoved {
                id,
                from,
                to,
                clock,
            } => self.event_moved(id, from, to, clock).await?,
            ServerMessage::WeddingStarted(ticket) => self.wedding_started(ticket).await?,
            ServerMessage::WeddingMoved { id, to, clock } => {
                self.wedding_moved(id, to, clock).await?
            }
            ServerMessage::WeddingEnded { id, failed } => self.wedding_ended(id, failed).await?,
            ServerMessage::Married {
                id,
                record,
                blessings,
                clock,
            } => self.married(id, record, blessings, clock).await?,
            ServerMessage::WeddingPriest { id, line } => self.wedding_priest(id, line).await?,
            ServerMessage::JoinGuild { guild_id } => self.join_guild(guild_id).await?,
            ServerMessage::GuildChanged => self.guild_changed().await?,
            ServerMessage::Proposed { from, name } => self.proposed(from, name).await?,
            ServerMessage::ProposalAccepted { proposer, item_id } => {
                self.proposal_accepted(proposer, item_id).await?
            }
            ServerMessage::Engaged(record) => self.engaged(record).await?,
            ServerMessage::CoupleEnded(record) => self.couple_ended(record).await?,
            ServerMessage::Wedding(news) => self.wedding_news(news).await?,
            ServerMessage::PartnerMap { map_id, partner } => {
                self.partner_map(map_id, partner).await?
            }
            ServerMessage::AddJunior { junior, name } => self.add_junior(junior, name).await?,
            ServerMessage::FamilyChanged => self.family_changed().await?,
            ServerMessage::Separate {
                senior,
                junior,
                levels,
            } => self.separate(senior, junior, levels).await?,
            ServerMessage::EarnRep { gain, senior_level } => {
                self.earn_rep(gain, senior_level).await?
            }
            ServerMessage::JoinAlliance { alliance_id } => self.join_alliance(alliance_id).await?,
            ServerMessage::PartySkill {
                used,
                origin,
                map_id,
            } => self.party_skill_reached(used, origin, map_id).await?,
            ServerMessage::PartyItem { item_id, map_id } => {
                self.party_item_reached(item_id, map_id).await?
            }
            ServerMessage::ReactorBroken {
                field,
                reactor_id,
                object_id,
                x,
                y,
            } => {
                // Unless they have gone elsewhere since.
                if self.in_game()?.field.key == field {
                    self.drop_reactor_loot(reactor_id, object_id, x, y).await?
                }
            }
            ServerMessage::PickedUp { drop } => self.picked_up(drop).await?,
            ServerMessage::Caught {
                slot,
                item_id,
                create,
            } => self.caught(slot, item_id, create).await?,
            ServerMessage::PetPickedUp { drop, pet } => self.pet_picked_up(drop, pet).await?,
            ServerMessage::Locate { reply_to } => self.locate(&reply_to),
            ServerMessage::RockLocate { reply_to, rock } => {
                let found = self.whereabouts();
                reply_to.send(ServerMessage::RockLocated { rock, found });
            }
            ServerMessage::RockLocated { rock, found } => self.rock_located(rock, found).await?,
            ServerMessage::MorphAnother { reply_to, potion } => {
                // The user is answered even when this client's packets fail.
                let morphed = self.morph_by_another(potion.item_id).await;
                reply_to.send(ServerMessage::MorphedAnother {
                    potion,
                    morphed: matches!(morphed, Ok(true)),
                });
                morphed?;
            }
            ServerMessage::MapleTvPartner { tv, reply } => {
                // Not in game (yet): the dropped reply refuses it.
                if let Ok(in_game) = self.in_game() {
                    let partner = Some(in_game.player.look());
                    self.send_to_directory(DirectoryMessage::MapleTv { tv, partner, reply })
                        .await?;
                }
            }
            ServerMessage::MapleTvQueued {
                slot,
                item_id,
                queued,
            } => {
                self.maple_tv_waiting = false;
                let player = &mut self.in_game_mut()?.player;
                for mut packet in spend_maple_tv(player, slot, item_id, queued)? {
                    self.send(&mut packet).await?;
                }
            }
            ServerMessage::MorphedAnother { potion, morphed } => {
                self.morphed_another(potion, morphed).await?
            }
            ServerMessage::FamilyLocate { reply_to, trip } => {
                let found = self.whereabouts();
                let summoned = matches!(trip, FamilyTrip::Summoned { .. });
                if !reply_to.send(ServerMessage::FamilyLocated { trip, found }) && summoned {
                    // The member left after saying yes.
                    let why = SummonFailure::NotFound;
                    self.summon_refund(String::new(), why).await?;
                }
            }
            ServerMessage::FamilyLocated { trip, found } => {
                self.family_located(trip, found).await?
            }
            ServerMessage::SummonRefund { name, why } => self.summon_refund(name, why).await?,
            ServerMessage::FamilyBuff(buff) => self.family_buff(buff).await?,
            ServerMessage::FamilyBondingCount(online) => self.family_bonding(online).await?,
            ServerMessage::CharInfoRequested { requester } => {
                let alliance = match self.alliance_id()? {
                    Some(alliance_id) => spawn_db(move || db::alliance::name(alliance_id))
                        .await?
                        .unwrap_or_default(),
                    None => String::new(),
                };
                let packet =
                    build::world::info::build_character_info(&self.in_game()?.player, &alliance)
                        .map_err(|e| RuntimeError::Handler(e.to_string()))?;
                requester.send_packet(packet);
            }
            ServerMessage::Famed {
                giver,
                giver_name,
                up,
            } => self.take_fame(giver, giver_name, up).await?,
            ServerMessage::FameMissed { target } => self.fame_missed(target).await?,
            ServerMessage::SummonBroken { skill_id } => self.summon_broken(skill_id).await?,
            ServerMessage::Beholder(support) => self.beholder_support(support).await?,
            ServerMessage::Diseased { disease, map_id } => self.diseased(disease, map_id).await?,
            ServerMessage::Countered { damage, map_id } => self.countered(damage, map_id).await?,
            ServerMessage::Absorbed { hp, mp, map_id } => self.absorbed(hp, mp, map_id).await?,
            ServerMessage::Dispelled { map_id } => self.dispelled(map_id).await?,
            ServerMessage::Banished { to, map_id } => self.banished(to, map_id).await?,
            ServerMessage::DoorWarp {
                from_map,
                map_id,
                exit,
            } => self.door_warp(from_map, map_id, exit).await?,
            ServerMessage::ShipWarp {
                from_map,
                map_id,
                portal,
            } => self.ship_warp(from_map, map_id, portal).await?,
            ServerMessage::Trade { room, event } => self.trade_event(room, event).await?,
            ServerMessage::PersonalShop { room, event } => self.shop_event(room, event).await?,
            ServerMessage::MiniGame { room, event } => self.game_event(room, event).await?,
            ServerMessage::HiredMerchant { owner, event } => {
                self.merchant_event(owner, event).await?
            }
            ServerMessage::SendPacket(mut packet) => self.send(&mut packet).await?,
            ServerMessage::Announce(packet) => {
                self.apply(WorldAction::Broadcast {
                    scope: net::BroadcastScope::World,
                    packet,
                })
                .await?
            }
            ServerMessage::QuestDrop {
                quest,
                item_id,
                mut packet,
            } => {
                let player = &self.in_game()?.player;
                if net::loot::wants(player, quest, item_id) {
                    self.send(&mut packet).await?;
                }
            }
        }
        Ok(())
    }

    /// Where the player stands, unless they are out of the field.
    fn whereabouts(&self) -> Option<Whereabouts> {
        self.game
            .as_ref()
            .filter(|_| !self.in_cash_shop)
            .map(|game| Whereabouts {
                map_id: game.player.stats.map_id,
                x: game.position.0,
                y: game.position.1,
            })
    }

    /// Take the player to the character their teleport rock looked for.
    async fn rock_located(
        &mut self,
        rock: Rock,
        found: Option<Whereabouts>,
    ) -> Result<(), RuntimeError> {
        let channel_id = self.channel_id;
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        let result = to_character(&mut game.player, rock, found, channel_id)?;
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }

    fn locate(&self, reply_to: &ClientHandle) {
        let Some(game) = self.game.as_ref() else {
            return;
        };
        let location = FindLocation::Map(game.player.stats.map_id);
        match build_find_result(game.player.name(), location) {
            Ok(packet) => {
                reply_to.send_packet(packet);
            }
            Err(e) => error!(client_id = self.client_id, "find result: {e}"),
        }
    }

    /// Send to the current field, restarting it once if its actor died.
    async fn send_to_field(&mut self, message: FieldMessage) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        match game.field.tx.send(message).await {
            Ok(()) => Ok(()),
            // An event's copy closed; the event already sent its members out.
            Err(_) if !self.services.fields.is_open(game.field.key) => Ok(()),
            Err(mpsc::error::SendError(message)) => {
                warn!(self.client_id, field = ?game.field.key, "Field actor gone; rejoining");
                let key = game.field.key;
                let spawn = net::spawn_position(key.map_id, game.player.stats.spawn_point)
                    .unwrap_or(FALLBACK_SPAWN);
                let avatar = game.player.field_avatar(spawn.x, spawn.y, spawn.stance);
                let field = self.services.fields.field(key);
                self.in_game_mut()?.field = field.clone();
                let rejoin = FieldMessage::Join {
                    handle: self.handle(),
                    avatar,
                    party: self.party_id,
                };
                field
                    .tx
                    .send(rejoin)
                    .await
                    .map_err(|_| RuntimeError::ChannelSend)?;
                // The new field reads the merchants back, but perhaps
                // before this one opened or sold.
                let merchant_news = matches!(
                    message,
                    FieldMessage::Merchant {
                        request: MerchantRequest::Place { .. }
                            | MerchantRequest::Settle { sale: Some(_), .. },
                        ..
                    }
                );
                if merchant_news
                    || !matches!(
                        message,
                        FieldMessage::Join { .. }
                            | FieldMessage::Leave { .. }
                            | FieldMessage::Trade { .. }
                            | FieldMessage::PersonalShop { .. }
                            | FieldMessage::MiniGame { .. }
                            | FieldMessage::Merchant { .. }
                    )
                {
                    field
                        .tx
                        .send(message)
                        .await
                        .map_err(|_| RuntimeError::ChannelSend)?;
                }
                // The trading room or shop died with the field; a
                // merchant the player manages from here stands elsewhere.
                self.abandon_trade().await?;
                self.abandon_personal_shop().await?;
                self.abandon_mini_game().await?;
                Box::pin(self.stop_remote_merchant()).await?;
                self.abandon_hired_merchant().await
            }
        }
    }

    /// Keep the directory's view of the player (map, level, job, HP)
    /// current, for party windows, HP bars, and the guild.
    async fn sync_presence(&mut self) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        let presence = Presence::of(&game.player);
        if self.presence.as_ref() == Some(&presence) {
            return Ok(());
        }
        self.presence = Some(presence.clone());
        self.send_to_directory(DirectoryMessage::Presence {
            client_id: self.client_id,
            presence,
        })
        .await
    }

    async fn send_to_directory(&self, message: DirectoryMessage) -> Result<(), RuntimeError> {
        self.services
            .directory
            .send(message)
            .await
            .map_err(|_| RuntimeError::ChannelSend)
    }

    fn in_game(&self) -> Result<&InGame, RuntimeError> {
        self.game.as_ref().ok_or(RuntimeError::NotLoggedIn)
    }

    fn in_game_mut(&mut self) -> Result<&mut InGame, RuntimeError> {
        self.game.as_mut().ok_or(RuntimeError::NotLoggedIn)
    }

    /// Persist the player if it has unsaved changes. Every mutation marks
    /// the player dirty, so a clean player already matches the database;
    /// skipping it also avoids racing the next channel's load after a
    /// channel change.
    pub(super) async fn flush(&mut self) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if !game.player.is_dirty() {
            return Ok(());
        }
        let mut snapshot = game.player.clone();
        // What the player put up in a trade or shop is still theirs.
        if let Some(trade) = &self.trade {
            trade.add_back(&mut snapshot);
        }
        if let Some(shop) = &self.personal_shop {
            shop.add_back(&mut snapshot);
        }
        spawn_db(move || db::player::save_player(&snapshot)).await?;
        game.player.mark_saved();
        Ok(())
    }

    /// Save the spawn point nearest where the player stands, so they come
    /// back there after a channel change or logout rather than at the
    /// portal they entered the map by.
    fn remember_spawn_point(&mut self) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        let (x, y) = game.position;
        if let Some(spawn_point) = net::closest_spawn_point(game.player.stats.map_id, x, y) {
            game.player.set_spawn_point(spawn_point);
        }
    }

    async fn save_if_dirty(&mut self) -> Result<(), RuntimeError> {
        if let Err(error) = self.flush().await {
            // Keep playing; the next autosave or logout retries.
            error!(self.client_id, %error, "Autosave failed");
        }
        Ok(())
    }

    async fn cleanup(&mut self) {
        if self.game.is_some() {
            // Before the save: the player gets their offer back, or the
            // partner's, and their unsold stock.
            if let Err(error) = self.leave_trade().await {
                warn!(self.client_id, %error, "Failed to finish trade");
            }
            if let Err(error) = self.leave_personal_shop().await {
                warn!(self.client_id, %error, "Failed to close personal shop");
            }
            if let Err(error) = self.leave_mini_game().await {
                warn!(self.client_id, %error, "Failed to leave mini game");
            }
            if let Err(error) = self.leave_hired_merchant().await {
                warn!(self.client_id, %error, "Failed to leave hired merchant");
            }
            if !self.in_cash_shop {
                let _ = self
                    .send_to_field(FieldMessage::Leave {
                        client_id: self.client_id,
                        staying: false,
                    })
                    .await;
            }
            if let Err(error) = self.close_door().await {
                warn!(self.client_id, %error, "Failed to close Mystic Door");
            }
            self.forget_event();
            if !self.changing_channel {
                if let Err(error) = self.announce_offline().await {
                    warn!(self.client_id, %error, "Failed to tell buddies of logout");
                }
            }
            let _ = self
                .send_to_directory(DirectoryMessage::Unregister {
                    client_id: self.client_id,
                    connection_id: self.connection_id,
                    changing_channel: self.changing_channel,
                })
                .await;
            self.remember_spawn_point();
            // A channel change keeps them: they go only when the player
            // leaves the game.
            if let (Some(game), false) = (self.game.as_mut(), self.changing_channel) {
                if net::items::drop_logout_items(&mut game.player) {
                    game.player.fit_hp_mp();
                }
            }
            if let Err(error) = self.flush().await {
                error!(self.client_id, %error, "Failed to save player on disconnect");
            } else if !self.changing_channel {
                self.apply_name_change().await;
            }
        }

        if let (Some(session_id), false) = (self.session_id, self.changing_channel) {
            if let Err(error) = spawn_db(move || db::session::delete_session(session_id)).await {
                error!(self.client_id, %error, "Failed to close session");
            }
        }
    }
}

/// How many of the best sellers to look through for five on sale for each
/// gender.
const BEST_SELLERS_CONSIDERED: i64 = 50;

/// When the item `item_id` bought now for `period` days expires; `None` for
/// 0, which never does. A pet sold for 0 days lives its `info/life` days.
/// A few rate cards last hours instead, as their tooltips (v83's
/// FUN_008edfc7) and descriptions say.
fn expiry(item_id: i32, period: i32) -> Option<SystemTime> {
    let minutes = match item_id {
        5211060 => Some(120),
        5211048 | 5360042 => Some(240),
        5211049 => Some(90),
        _ => None,
    };
    if let Some(minutes) = minutes.filter(|_| period > 0) {
        return Some(SystemTime::now() + Duration::from_secs(minutes * 60));
    }
    let days = match period {
        0 if game::pet::is_pet(item_id) => {
            net::items::info(item_id).map_or(0, |info| info.pet_life)
        }
        _ => period,
    };
    (days > 0).then(|| SystemTime::now() + Duration::from_secs(days as u64 * 86_400))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio::time::{timeout, Instant};

    /// A peer that stops reading fills the socket, the actor blocks writing,
    /// its queue fills, and the kick must still end the actor (`run` always
    /// finishes with `cleanup`).
    #[tokio::test]
    async fn kicking_a_client_blocked_on_a_full_socket_ends_the_actor() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, addr) = listener.accept().await.unwrap();

        let (directory, _directory_rx) = mpsc::channel(16);
        let services = WorldServices {
            directory,
            fields: FieldRegistry::new(Duration::from_millis(100)),
        };
        let actor = ClientActor::new(stream, addr.ip(), 0, services)
            .await
            .unwrap();
        let handle = actor.handle();
        let running = tokio::spawn(actor.run());

        // Large packets so the kernel buffers fill quickly; the peer never reads.
        let payload = vec![0u8; 30_000];
        let deadline = Instant::now() + Duration::from_secs(30);
        while handle.send_packet(Packet::new(&payload)) {
            assert!(Instant::now() < deadline, "socket never filled");
            tokio::task::yield_now().await;
        }

        // The full queue kicked the client; the actor must stop promptly even
        // though it is mid-write to a socket nobody is reading.
        timeout(Duration::from_secs(2), running)
            .await
            .expect("actor stayed blocked on the socket write after being kicked")
            .expect("actor task panicked");
        drop(peer);
    }
}
