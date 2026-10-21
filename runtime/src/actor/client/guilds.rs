use super::ClientActor;
use crate::{
    db::spawn_db,
    error::RuntimeError,
    message::{DirectoryMessage, FieldMessage},
};
use db::guild::{CreateError, JoinError};
use game::guild::{self, Emblem, GuildMember, GuildTag};
use game_data::forbidden_names::ForbiddenNames;
use net::error::NetworkError;
use net::packet::build::world::guild::{
    build_guild_chat, build_guild_created, build_guild_error, build_guild_info,
    build_guild_mark_changed, build_guild_name_changed, build_member_expelled, build_member_joined,
    build_member_left, build_notice_changed, build_rank_changed, build_titles_changed, GuildError,
};
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{drop, stat};
use net::{ClientId, GuildRequest};
use packet::Packet;
use tracing::warn;

impl ClientActor {
    /// On entering the game (a channel change too): the guild window, with
    /// who is online. The client starts with no guild, so nothing is sent
    /// for none.
    pub(super) async fn load_guild(&mut self) -> Result<(), RuntimeError> {
        match self.in_game()?.player.guild.as_ref().map(|tag| tag.id) {
            Some(guild_id) => self.show_guild(guild_id).await,
            None => Ok(()),
        }
    }

    pub(super) async fn guild_request(
        &mut self,
        request: GuildRequest,
    ) -> Result<(), RuntimeError> {
        let guild_id = self.in_game()?.player.guild.as_ref().map(|tag| tag.id);
        match request {
            GuildRequest::Reload => match guild_id {
                Some(guild_id) => self.show_guild(guild_id).await,
                None => self.send(&mut build_guild_info(None)?).await,
            },
            GuildRequest::Create { name } => self.create_guild(name).await,
            GuildRequest::Chat { text } => {
                let Some(guild_id) = guild_id else {
                    return Ok(());
                };
                let cid = self.client_id;
                let to: Vec<ClientId> = spawn_db(move || db::guild::member_ids(guild_id))
                    .await?
                    .into_iter()
                    .filter(|&id| id != cid)
                    .collect();
                let name = self.in_game()?.player.stats.name.clone();
                let packet = build_guild_chat(&name, &text)?;
                self.send_to_directory(DirectoryMessage::Deliver { to, packet })
                    .await
            }
            GuildRequest::Invite { name } => match guild_id {
                Some(guild_id) => self.invite_to_guild(guild_id, name).await,
                None => Ok(()),
            },
            GuildRequest::Accept { guild_id } => {
                self.send_to_directory(DirectoryMessage::GuildAccept {
                    from: self.client_id,
                    guild_id,
                })
                .await
            }
            GuildRequest::Deny { inviter, refusal } => {
                self.send_to_directory(DirectoryMessage::GuildDeny {
                    from: self.client_id,
                    inviter,
                    refusal,
                })
                .await
            }
            GuildRequest::Leave => match guild_id {
                Some(guild_id) => self.leave_guild(guild_id).await,
                None => Ok(()),
            },
            GuildRequest::Expel { member_id } => match guild_id {
                Some(guild_id) => self.expel_from_guild(guild_id, member_id).await,
                None => Ok(()),
            },
            GuildRequest::SetTitles { titles } => {
                let Some(guild_id) = guild_id else {
                    return Ok(());
                };
                let packet = build_titles_changed(guild_id, &titles)?;
                let forbidden = forbidden_names()?;
                let cid = self.client_id;
                let to = spawn_db(move || {
                    db::guild::set_titles(guild_id, cid, &titles, |title| forbidden.forbids(title))
                })
                .await?;
                self.tell_guild(to, packet, "Guild titles").await
            }
            GuildRequest::SetRank { member_id, rank } => {
                let Some(guild_id) = guild_id else {
                    return Ok(());
                };
                let cid = self.client_id;
                let to =
                    spawn_db(move || db::guild::set_rank(guild_id, cid, member_id, rank)).await?;
                let packet = build_rank_changed(guild_id, member_id, rank)?;
                self.tell_guild(to, packet, "Guild rank change").await
            }
            GuildRequest::SetNotice { notice } => {
                let Some(guild_id) = guild_id else {
                    return Ok(());
                };
                let packet = build_notice_changed(guild_id, &notice)?;
                let cid = self.client_id;
                let to = spawn_db(move || db::guild::set_notice(guild_id, cid, &notice)).await?;
                self.tell_guild(to, packet, "Guild notice").await
            }
        }
    }

    /// A change the player made to the guild went through when `to` lists
    /// the members, who all hear of it. The client checks the player's rank
    /// and what the change may be first.
    async fn tell_guild(
        &mut self,
        to: Option<Vec<i32>>,
        packet: Packet,
        what: &str,
    ) -> Result<(), RuntimeError> {
        match to {
            Some(to) => {
                self.send_to_directory(DirectoryMessage::Deliver { to, packet })
                    .await
            }
            None => {
                warn!(self.client_id, "{what} the client should not send; ignored");
                Ok(())
            }
        }
    }

    /// The client checks the inviter's rank and the guild's room first.
    async fn invite_to_guild(&mut self, guild_id: i32, name: String) -> Result<(), RuntimeError> {
        let guild = spawn_db(move || db::guild::load(guild_id)).await?;
        if !guild.may_invite(self.client_id) {
            warn!(
                self.client_id,
                "Guild invitation the client should not send; ignored"
            );
            return Ok(());
        }
        self.send_to_directory(DirectoryMessage::GuildInvite {
            from: self.client_id,
            guild_id,
            name,
        })
        .await
    }

    /// The directory confirmed the invitation: join at the lowest rank.
    /// Every member online hears it; the newcomer's client then asks for the
    /// whole guild.
    pub(super) async fn join_guild(&mut self, guild_id: i32) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let rank = match spawn_db(move || db::guild::add_member(guild_id, cid)).await? {
            Ok(rank) => rank,
            Err(JoinError::Full) => {
                return self.send(&mut build_guild_error(GuildError::Full)?).await
            }
            Err(JoinError::AlreadyInGuild) => {
                return self
                    .send(&mut build_guild_error(GuildError::InviteeInGuild)?)
                    .await
            }
        };
        let guild = spawn_db(move || db::guild::load(guild_id)).await?;
        let stats = &self.in_game()?.player.stats;
        let member = GuildMember {
            id: cid,
            name: stats.name.clone(),
            job: stats.job,
            level: stats.level,
            rank,
            online: true,
        };
        self.send_to_directory(DirectoryMessage::Deliver {
            to: guild.members.iter().map(|member| member.id).collect(),
            packet: build_member_joined(guild_id, &member)?,
        })
        .await?;
        self.set_guild(Some(guild.tag())).await
    }

    /// The master can't leave; the client doesn't offer it.
    async fn leave_guild(&mut self, guild_id: i32) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        if !spawn_db(move || db::guild::remove_member(guild_id, cid)).await? {
            warn!(cid, "The guild master can't leave the guild; ignored");
            return Ok(());
        }
        let mut to = spawn_db(move || db::guild::member_ids(guild_id)).await?;
        to.push(cid);
        let name = self.in_game()?.player.stats.name.clone();
        self.send_to_directory(DirectoryMessage::Deliver {
            to,
            packet: build_member_left(guild_id, cid, &name)?,
        })
        .await?;
        self.set_guild(None).await
    }

    /// Put a member out, if the player's rank allows it (`Guild::may_expel`).
    async fn expel_from_guild(
        &mut self,
        guild_id: i32,
        member_id: i32,
    ) -> Result<(), RuntimeError> {
        let guild = spawn_db(move || db::guild::load(guild_id)).await?;
        let name = match guild.member(member_id) {
            Some(member) if guild.may_expel(self.client_id, member_id) => member.name.clone(),
            _ => {
                warn!(
                    self.client_id,
                    member_id, "Guild expulsion not allowed; ignored"
                );
                return Ok(());
            }
        };
        if !spawn_db(move || db::guild::remove_member(guild_id, member_id)).await? {
            return Ok(());
        }
        self.send_to_directory(DirectoryMessage::Deliver {
            to: guild.members.iter().map(|member| member.id).collect(),
            packet: build_member_expelled(guild_id, member_id, &name)?,
        })
        .await?;
        self.send_to_directory(DirectoryMessage::GuildChanged {
            client_id: member_id,
        })
        .await
    }

    /// Someone else changed the player's guild: read it again.
    pub(super) async fn guild_changed(&mut self) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let tag = spawn_db(move || db::guild::tag(cid)).await?;
        self.set_guild(tag).await
    }

    /// Others on the map see the guild's name and emblem under the player's,
    /// or none.
    async fn set_guild(&mut self, tag: Option<GuildTag>) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let (name, emblem) = tag
            .as_ref()
            .map_or((String::new(), Emblem::default()), |tag| {
                (tag.name.clone(), tag.emblem)
            });
        let player = &mut self.in_game_mut()?.player;
        player.guild = tag;
        let avatar = player.field_avatar(0, 0, 0);
        self.send_to_field(FieldMessage::UpdateAvatar {
            client_id: cid,
            avatar,
            announce: vec![
                build_guild_name_changed(cid, &name)?,
                build_guild_mark_changed(cid, &emblem)?,
            ],
        })
        .await
    }

    /// The directory, which knows who is online, sends the guild.
    async fn show_guild(&mut self, guild_id: i32) -> Result<(), RuntimeError> {
        let guild = spawn_db(move || db::guild::load(guild_id)).await?;
        self.send_to_directory(DirectoryMessage::GuildInfo {
            owner: self.client_id,
            guild,
        })
        .await
    }

    /// Found a guild with the name the player gave Heracle. Everything the
    /// script checked is checked again; a taken name asks again.
    async fn create_guild(&mut self, name: String) -> Result<(), RuntimeError> {
        if !self.naming_guild {
            warn!(
                self.client_id,
                "Guild name without Heracle's prompt; ignored"
            );
            return Ok(());
        }
        let player = &self.in_game()?.player;
        let refusal = if player.gm_level > 0 {
            Some(GuildError::Admin)
        } else if player.guild.is_some() {
            Some(GuildError::AlreadyInGuild)
        } else if player.stats.level < guild::MIN_LEVEL {
            Some(GuildError::LevelTooLow)
        } else if !guild::name_is_valid(&name) || forbidden_names()?.forbids(&name) {
            // The client checks names too; ask again as for a taken one.
            Some(GuildError::NameTaken)
        } else {
            None
        };
        if refusal.is_none() && player.meso < guild::CREATE_COST {
            // Heracle checked before asking; the client has no message for it.
            warn!(
                self.client_id,
                "Mesos for the guild went since Heracle asked"
            );
            self.naming_guild = false;
            return Ok(());
        }
        let (world_id, cid) = (player.world_id, self.client_id);
        let created = match refusal {
            Some(error) => Err(error),
            None => spawn_db(move || db::guild::create(world_id, &name, cid))
                .await?
                .map_err(|error| match error {
                    CreateError::NameTaken => GuildError::NameTaken,
                    CreateError::AlreadyInGuild => GuildError::AlreadyInGuild,
                }),
        };
        let guild_id = match created {
            Ok(guild_id) => guild_id,
            Err(error) => {
                self.naming_guild = error == GuildError::NameTaken;
                return self.send(&mut build_guild_error(error)?).await;
            }
        };
        self.naming_guild = false;

        let mut guild = spawn_db(move || db::guild::load(guild_id)).await?;
        for member in &mut guild.members {
            member.online = member.id == cid;
        }
        let player = &mut self.in_game_mut()?.player;
        player.meso -= guild::CREATE_COST;
        player.mark_dirty();
        let meso = player.meso;
        self.flush().await?;
        let mut packets = vec![
            stat::build_stat_update(false, &[StatUpdate::Meso(meso)])?,
            drop::build_meso_gain_in_chat(-guild::CREATE_COST)?,
            build_guild_created(&guild)?,
        ];
        self.send_all(&mut packets).await?;
        self.set_guild(Some(guild.tag())).await
    }
}

/// The words guild names and titles may not hold, as for character names.
fn forbidden_names() -> Result<&'static ForbiddenNames, RuntimeError> {
    net::names::forbidden_names().map_err(|error| NetworkError::PacketHandlerError(error).into())
}
