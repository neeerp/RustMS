use super::ClientActor;
use crate::{
    db::spawn_db,
    error::RuntimeError,
    message::{DirectoryMessage, FieldMessage},
};
use db::guild::CreateError;
use game::guild;
use net::packet::build::world::guild::{
    build_guild_chat, build_guild_created, build_guild_error, build_guild_info,
    build_guild_mark_changed, build_guild_name_changed, GuildError,
};
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{drop, stat};
use net::{ClientId, GuildRequest};
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
        }
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
        } else if !guild::name_is_valid(&name) {
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
        player.guild = Some(guild.tag());
        player.mark_dirty();
        let (meso, avatar) = (player.meso, player.field_avatar(0, 0, 0));
        self.flush().await?;
        let mut packets = vec![
            stat::build_stat_update(false, &[StatUpdate::Meso(meso)])?,
            drop::build_meso_gain_in_chat(-guild::CREATE_COST)?,
            build_guild_created(&guild)?,
        ];
        self.send_all(&mut packets).await?;
        // Others on the map see the name and emblem under the master's.
        self.send_to_field(FieldMessage::UpdateAvatar {
            client_id: cid,
            avatar,
            announce: vec![
                build_guild_name_changed(cid, &guild.name)?,
                build_guild_mark_changed(cid, &guild.emblem)?,
            ],
        })
        .await
    }
}
