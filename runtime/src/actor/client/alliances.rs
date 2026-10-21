use super::ClientActor;
use crate::{
    db::spawn_db,
    error::RuntimeError,
    message::{AllianceNews, DirectoryMessage},
};
use db::alliance::{CreateError, GuildGone, JoinError, Removed};
use game::alliance;
use game::guild::Guild;
use game_data::server_content::keys;
use net::npc_scripts::{AllianceView, NpcScript};
use net::packet::build::world::alliance::{
    build_alliance_changed, build_alliance_chat, build_alliance_disbanded, build_master_changed,
    build_notice_changed, build_rank_changed, build_titles_changed,
};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{drop, stat};
use net::{AllianceRequest, ClientId};
use packet::Packet;
use tracing::warn;

impl ClientActor {
    /// The alliance the player's guild is in.
    pub(super) fn alliance_id(&self) -> Result<Option<i32>, RuntimeError> {
        Ok(self
            .in_game()?
            .player
            .guild
            .as_ref()
            .map(|tag| tag.alliance_id)
            .filter(|&id| id != 0))
    }

    pub(super) async fn alliance_request(
        &mut self,
        request: AllianceRequest,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        match request {
            AllianceRequest::Accept { alliance_id } => {
                return self
                    .send_to_directory(DirectoryMessage::AllianceAccept {
                        from: cid,
                        alliance_id,
                    })
                    .await
            }
            AllianceRequest::Deny { inviter } => {
                let Some(tag) = self.in_game()?.player.guild.as_ref() else {
                    return Ok(());
                };
                let guild_name = tag.name.clone();
                return self
                    .send_to_directory(DirectoryMessage::AllianceDeny {
                        from: cid,
                        inviter,
                        guild_name,
                    })
                    .await;
            }
            _ => {}
        }
        let Some(alliance_id) = self.alliance_id()? else {
            return Ok(());
        };
        match request {
            AllianceRequest::Accept { .. } | AllianceRequest::Deny { .. } => Ok(()),
            AllianceRequest::Info => self.show_alliance(alliance_id).await,
            AllianceRequest::Leave => {
                let removed = spawn_db(move || db::alliance::leave(alliance_id, cid)).await?;
                self.guild_removed(removed, false).await
            }
            AllianceRequest::Expel { guild_id } => {
                let removed =
                    spawn_db(move || db::alliance::expel(alliance_id, cid, guild_id)).await?;
                self.guild_removed(removed, true).await
            }
            AllianceRequest::Invite { guild_name } => {
                self.invite_guild(alliance_id, guild_name).await
            }
            AllianceRequest::ChangeMaster { member_id } => {
                let before =
                    spawn_db(move || db::alliance::change_master(alliance_id, cid, member_id))
                        .await?;
                let Some(before) = before else {
                    return self.ignored("Alliance master change");
                };
                let name = member_name(&before, member_id);
                let pink = net::content::text().get(keys::ALLIANCE_MASTER, &[("name", &name)]);
                let packets = vec![
                    build_master_changed(alliance_id, cid, member_id)?,
                    build_broadcast(Broadcast::Pink, &pink)?,
                ];
                self.tell_alliance(before.member_ids().collect(), packets)
                    .await
            }
            AllianceRequest::SetTitles { titles } => {
                let packet = build_titles_changed(alliance_id, &titles)?;
                let forbidden = net::names::forbidden_names()
                    .map_err(|error| RuntimeError::Handler(error.to_string()))?;
                let before = spawn_db(move || {
                    db::alliance::set_titles(alliance_id, cid, &titles, |title| {
                        forbidden.forbids(title)
                    })
                })
                .await?;
                let Some(before) = before else {
                    return self.ignored("Alliance titles");
                };
                self.tell_alliance(before.member_ids().collect(), vec![packet])
                    .await
            }
            AllianceRequest::SetRank { member_id, up } => {
                let changed =
                    spawn_db(move || db::alliance::set_rank(alliance_id, cid, member_id, up))
                        .await?;
                let Some((rank, before)) = changed else {
                    return self.ignored("Alliance rank change");
                };
                let name = member_name(&before, member_id);
                let title = &before.titles[usize::from(rank - 1)];
                let pink = net::content::text()
                    .get(keys::ALLIANCE_RANK, &[("name", &name), ("title", title)]);
                let packets = vec![
                    build_rank_changed(member_id, rank)?,
                    build_broadcast(Broadcast::Pink, &pink)?,
                ];
                self.tell_alliance(before.member_ids().collect(), packets)
                    .await
            }
            AllianceRequest::SetNotice { notice } => {
                let packet = build_notice_changed(alliance_id, &notice)?;
                let before =
                    spawn_db(move || db::alliance::set_notice(alliance_id, cid, &notice)).await?;
                let Some(before) = before else {
                    return self.ignored("Alliance notice");
                };
                self.tell_alliance(before.member_ids().collect(), vec![packet])
                    .await
            }
            AllianceRequest::Chat { text } => {
                let to: Vec<ClientId> = spawn_db(move || db::alliance::member_ids(alliance_id))
                    .await?
                    .into_iter()
                    .filter(|&id| id != cid)
                    .collect();
                let name = self.in_game()?.player.stats.name.clone();
                let packet = build_alliance_chat(&name, &text)?;
                self.send_to_directory(DirectoryMessage::Deliver { to, packet })
                    .await
            }
        }
    }

    /// A request the client shouldn't have sent, or one overtaken by
    /// another member's change.
    fn ignored(&self, what: &str) -> Result<(), RuntimeError> {
        warn!(self.client_id, "{what} not allowed; ignored");
        Ok(())
    }

    /// Send these packets to whichever of `to` are online.
    async fn tell_alliance(
        &mut self,
        to: Vec<ClientId>,
        packets: Vec<Packet>,
    ) -> Result<(), RuntimeError> {
        for packet in packets {
            self.send_to_directory(DirectoryMessage::Deliver {
                to: to.clone(),
                packet,
            })
            .await?;
        }
        Ok(())
    }

    /// Tell the player of a problem in pink, as Cosmic does.
    async fn pink(&mut self, key: &str, vars: &[(&str, &str)]) -> Result<(), RuntimeError> {
        let text = net::content::text().get(key, vars);
        self.send(&mut build_broadcast(Broadcast::Pink, &text)?)
            .await
    }

    /// The client asks for the alliance after the guild window shows it is
    /// in one, and when its guild joins one.
    async fn show_alliance(&mut self, alliance_id: i32) -> Result<(), RuntimeError> {
        match spawn_db(move || db::alliance::load(alliance_id)).await? {
            Some(alliance) => {
                self.send_to_directory(DirectoryMessage::AllianceNews {
                    to: vec![self.client_id],
                    news: AllianceNews::Info(alliance),
                })
                .await
            }
            // Gone too soon for the player to hear.
            None => self.guild_changed().await,
        }
    }

    /// Only the master invites, while there is room; the client allows it
    /// no one else. The guild's master must be online to hear it.
    async fn invite_guild(&mut self, alliance_id: i32, name: String) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let alliance = spawn_db(move || db::alliance::load(alliance_id)).await?;
        let Some(alliance) = alliance.filter(|alliance| alliance.is_master(cid)) else {
            return self.ignored("Alliance invitation");
        };
        if !alliance.may_invite(cid) {
            return self.pink(keys::ALLIANCE_FULL, &[]).await;
        }
        let world_id = self.in_game()?.player.world_id;
        // No guild has a name that can't be one; nothing else reaches the
        // database.
        let found = if game::guild::name_is_valid(&name) {
            let name = name.clone();
            spawn_db(move || db::alliance::find_guild(world_id, &name)).await?
        } else {
            None
        };
        let vars = [("guild", name.as_str())];
        match found {
            None => self.pink(keys::ALLIANCE_NO_GUILD, &vars).await,
            Some((_, in_alliance, _)) if in_alliance != 0 => {
                self.pink(keys::ALLIANCE_IN_ALLIANCE, &vars).await
            }
            Some((_, _, master)) => {
                self.send_to_directory(DirectoryMessage::AllianceInvite {
                    from: cid,
                    alliance_id,
                    master,
                    guild_name: name,
                })
                .await
            }
        }
    }

    /// The directory confirmed the invitation the player, a guild master,
    /// accepted: their guild joins. Everyone in the alliance hears it; the
    /// joining guild's clients then ask for the whole alliance.
    pub(super) async fn join_alliance(&mut self, alliance_id: i32) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let alliance = match spawn_db(move || db::alliance::add_guild(alliance_id, cid)).await? {
            Ok(alliance) => alliance,
            Err(JoinError::Full) => return self.pink(keys::ALLIANCE_FULL, &[]).await,
            Err(error) => {
                warn!(
                    cid,
                    alliance_id,
                    ?error,
                    "Alliance invitation overtaken; ignored"
                );
                return Ok(());
            }
        };
        let Some((guild, _)) = alliance.member(cid) else {
            return Ok(());
        };
        let (guild_id, guild_members) = (guild.id, member_ids(guild));
        self.send_to_directory(DirectoryMessage::AllianceNews {
            to: alliance.member_ids().collect(),
            news: AllianceNews::GuildJoined { alliance, guild_id },
        })
        .await?;
        self.retag(guild_members).await
    }

    /// The guild left or was expelled: the alliance and the guild hear so,
    /// the guild's clients drop the alliance, and the others read in pink
    /// which guild went.
    async fn guild_removed(
        &mut self,
        removed: Option<Removed>,
        expelled: bool,
    ) -> Result<(), RuntimeError> {
        let Some(Removed { alliance, guild }) = removed else {
            return self.ignored("Alliance departure");
        };
        let remaining: Vec<ClientId> = alliance.member_ids().collect();
        let guild_members = member_ids(&guild);
        let key = if expelled {
            keys::ALLIANCE_EXPELLED
        } else {
            keys::ALLIANCE_LEFT
        };
        let pink = net::content::text().get(key, &[("guild", &guild.name)]);
        let to = remaining.iter().chain(&guild_members).copied().collect();
        self.send_to_directory(DirectoryMessage::AllianceNews {
            to,
            news: AllianceNews::GuildLeft {
                alliance,
                guild,
                expelled,
            },
        })
        .await?;
        self.tell_alliance(remaining, vec![build_broadcast(Broadcast::Pink, &pink)?])
            .await?;
        self.retag(guild_members).await
    }

    /// These characters' guild changed alliance: each reads their tag
    /// again, the player at once.
    async fn retag(&mut self, members: Vec<ClientId>) -> Result<(), RuntimeError> {
        for client_id in members {
            if client_id == self.client_id {
                self.guild_changed().await?;
            } else {
                self.send_to_directory(DirectoryMessage::GuildChanged { client_id })
                    .await?;
            }
        }
        Ok(())
    }

    /// A change to the members of the player's guild, `guild_id`, which is
    /// in alliance `alliance_id`: the alliance's lists show it.
    pub(super) async fn tell_alliance_of_guild(
        &mut self,
        alliance_id: i32,
        guild_id: i32,
    ) -> Result<(), RuntimeError> {
        let Some(alliance) = spawn_db(move || db::alliance::load(alliance_id)).await? else {
            return Ok(());
        };
        self.send_to_directory(DirectoryMessage::AllianceNews {
            to: alliance.member_ids().collect(),
            news: AllianceNews::GuildChanged { alliance, guild_id },
        })
        .await
    }

    /// The player left guild `guild_id`, or was put out of it, while it was
    /// in alliance `alliance_id`: their client drops the alliance, and the
    /// alliance's lists drop them.
    pub(super) async fn left_allied_guild(
        &mut self,
        member: ClientId,
        alliance_id: i32,
        guild_id: i32,
    ) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::Deliver {
            to: vec![member],
            packet: build_alliance_disbanded(alliance_id)?,
        })
        .await?;
        self.tell_alliance_of_guild(alliance_id, guild_id).await
    }

    /// The player's guild, in alliance `alliance_id`, was disbanded; its
    /// members were `members`. Without its master's guild the alliance
    /// goes too; otherwise the rest see the guild go.
    pub(super) async fn allied_guild_disbanded(
        &mut self,
        alliance_id: i32,
        guild_id: i32,
        guild_name: &str,
        members: &[ClientId],
    ) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::Deliver {
            to: members.to_vec(),
            packet: build_alliance_disbanded(alliance_id)?,
        })
        .await?;
        match spawn_db(move || db::alliance::guild_gone(alliance_id)).await? {
            Some(GuildGone::Disbanded(alliance)) => self.alliance_gone(&alliance).await,
            Some(GuildGone::Remains(alliance)) => {
                let to: Vec<ClientId> = alliance.member_ids().collect();
                let pink = net::content::text().get(keys::ALLIANCE_LEFT, &[("guild", guild_name)]);
                // Only 0x10 drops a guild from the clients' lists; of a guild
                // not their own they read just its id.
                let guild = Guild {
                    id: guild_id,
                    name: guild_name.to_string(),
                    titles: game::guild::DEFAULT_TITLES.map(String::from),
                    capacity: 0,
                    emblem: Default::default(),
                    notice: String::new(),
                    points: 0,
                    alliance_id: 0,
                    members: Vec::new(),
                };
                self.send_to_directory(DirectoryMessage::AllianceNews {
                    to: to.clone(),
                    news: AllianceNews::GuildLeft {
                        alliance,
                        guild,
                        expelled: false,
                    },
                })
                .await?;
                self.tell_alliance(to, vec![build_broadcast(Broadcast::Pink, &pink)?])
                    .await
            }
            None => Ok(()),
        }
    }

    /// The alliance, as it was, is disbanded: its members' clients drop it,
    /// its invitations lapse, and each member's tag is read again.
    async fn alliance_gone(
        &mut self,
        alliance: &game::alliance::Alliance,
    ) -> Result<(), RuntimeError> {
        let members: Vec<ClientId> = alliance.member_ids().collect();
        self.send_to_directory(DirectoryMessage::Deliver {
            to: members.clone(),
            packet: build_alliance_disbanded(alliance.id)?,
        })
        .await?;
        self.send_to_directory(DirectoryMessage::AllianceDisbanded {
            alliance_id: alliance.id,
        })
        .await?;
        self.retag(members).await
    }

    /// Lenario's script needs the player's guild, its alliance, and whether
    /// they lead their party.
    pub(super) async fn talk_to_lenario(&mut self) -> Result<(), RuntimeError> {
        let guild = match self.in_game()?.player.guild.as_ref().map(|tag| tag.id) {
            Some(guild_id) => spawn_db(move || db::guild::load(guild_id)).await?,
            None => None,
        };
        let alliance = match guild.as_ref().map(|guild| guild.alliance_id) {
            Some(alliance_id) if alliance_id != 0 => {
                spawn_db(move || db::alliance::load(alliance_id)).await?
            }
            _ => None,
        };
        let party_leader = self.party_here().await?.is_some();
        let view = AllianceView {
            guild: guild.as_ref(),
            alliance: alliance.as_ref(),
            party_leader,
        };
        let script = NpcScript::lenario(&self.in_game()?.player, view);
        self.start_script(script).await
    }

    /// The party the player leads, its members on their map, the player
    /// first.
    async fn party_here(&mut self) -> Result<Option<Vec<ClientId>>, RuntimeError> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.send_to_directory(DirectoryMessage::PartyHere {
            from: self.client_id,
            reply,
        })
        .await?;
        answer.await.map_err(|_| RuntimeError::ChannelSend)
    }

    /// Lenario says how it went.
    async fn lenario_says(&mut self, key: &str, vars: &[(&str, &str)]) -> Result<(), RuntimeError> {
        let text = net::content::text().get(key, vars);
        self.start_script(NpcScript::lenario_says(text)).await
    }

    /// Charge the player for Lenario's work.
    async fn pay_lenario(&mut self, cost: i32) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        player.meso -= cost;
        player.mark_dirty();
        let meso = player.meso;
        self.flush().await?;
        let mut packets = vec![
            stat::build_stat_update(false, &[StatUpdate::Meso(meso)])?,
            drop::build_meso_gain_in_chat(-cost)?,
        ];
        self.send_all(&mut packets).await
    }

    /// Found the alliance the player named for Lenario: the player, a guild
    /// master leading a party, with one other guild's master from it beside
    /// them. Everything the script checked is checked again; a name taken
    /// meanwhile asks again.
    pub(super) async fn create_alliance(&mut self, name: String) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let (world_id, cid) = (player.world_id, self.client_id);
        let allowed = player
            .guild
            .as_ref()
            .is_some_and(|tag| tag.alliance_id == 0)
            && player.meso >= alliance::CREATE_COST;
        if !allowed {
            return self.lenario_says(keys::LENARIO_FAILED, &[]).await;
        }
        let forbidden = net::names::forbidden_names()
            .map_err(|error| RuntimeError::Handler(error.to_string()))?;
        if !alliance::name_is_valid(&name) || forbidden.forbids(&name) {
            return self.start_script(NpcScript::lenario_name_again()).await;
        }
        let Some(here) = self.party_here().await? else {
            return self.lenario_says(keys::LENARIO_NOT_LEADER, &[]).await;
        };
        let created = {
            let name = name.clone();
            spawn_db(move || db::alliance::create(world_id, &name, cid, &here[1..])).await?
        };
        let alliance = match created {
            Ok(alliance) => alliance,
            Err(CreateError::NameTaken) => {
                return self.start_script(NpcScript::lenario_name_again()).await
            }
            Err(CreateError::NotAllowed) => {
                return self.lenario_says(keys::LENARIO_CREATE_PARTY, &[]).await
            }
        };
        self.pay_lenario(alliance::CREATE_COST).await?;
        let members: Vec<ClientId> = alliance.member_ids().collect();
        self.send_to_directory(DirectoryMessage::AllianceNews {
            to: members.clone(),
            news: AllianceNews::Info(alliance),
        })
        .await?;
        self.retag(members).await?;
        self.lenario_says(keys::LENARIO_CREATED, &[("name", &name)])
            .await
    }

    /// Make room for one more guild than `from`, the capacity Lenario
    /// quoted, and charge the master, before the alliance hears.
    pub(super) async fn grow_alliance(&mut self, from: u8) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let can_pay = self.in_game()?.player.meso >= alliance::GROW_COST;
        let grown = match self.alliance_id()? {
            Some(alliance_id) if can_pay => {
                spawn_db(move || db::alliance::grow(alliance_id, cid, from)).await?
            }
            _ => None,
        };
        let Some(mut alliance) = grown else {
            return self.lenario_says(keys::LENARIO_FAILED, &[]).await;
        };
        self.pay_lenario(alliance::GROW_COST).await?;
        alliance.capacity = from + 1;
        let packet = build_alliance_changed(&alliance)?;
        self.tell_alliance(alliance.member_ids().collect(), vec![packet])
            .await?;
        let capacity = alliance.capacity.to_string();
        self.lenario_says(keys::LENARIO_GROWN, &[("capacity", &capacity)])
            .await
    }

    /// Disband the alliance, if the player is its master.
    pub(super) async fn disband_alliance(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let disbanded = match self.alliance_id()? {
            Some(alliance_id) => spawn_db(move || db::alliance::disband(alliance_id, cid)).await?,
            None => None,
        };
        let Some(alliance) = disbanded else {
            return self.lenario_says(keys::LENARIO_FAILED, &[]).await;
        };
        self.alliance_gone(&alliance).await?;
        self.lenario_says(keys::LENARIO_DISBANDED, &[]).await
    }
}

fn member_ids(guild: &Guild) -> Vec<ClientId> {
    guild.members.iter().map(|member| member.id).collect()
}

fn member_name(alliance: &game::alliance::Alliance, member_id: i32) -> String {
    alliance
        .member(member_id)
        .map(|(_, member)| member.name.clone())
        .unwrap_or_default()
}
