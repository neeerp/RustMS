use crate::assertions::{
    assert_handshake, assert_login_success, assert_opcode, assert_redirect_target,
    assert_server_list_kind,
};
use crate::config::{HarnessConfig, MultiHarnessConfig};
use crate::connection::MapleTestConnection;
use crate::error::HarnessError;
use crate::packets::{
    build_accept_tos, build_char_list_request, build_char_select, build_check_char_name,
    build_create_char, build_login_credentials, build_login_started, build_player_logged_in,
    build_server_list_request, build_set_gender, decode_char_list, decode_char_name_response,
    decode_keymap, decode_last_connected_world, decode_login_status, decode_new_character,
    decode_recommended_worlds, decode_server_redirect, decode_set_field, opcode_name,
    CharacterSummary, CharacterTemplate, LoginStatusPacket, ServerListPacket,
};
use net::packet::op::SendOpcode;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntryResult {
    pub character_id: i32,
    pub character_name: String,
    pub map_id: i32,
    pub login_addr: String,
    pub world_addr: String,
}

pub struct WorldSession {
    pub connection: MapleTestConnection,
    pub character_id: i32,
    pub character_name: String,
    pub map_id: i32,
    pub login_addr: String,
    pub world_addr: String,
    pub level: u8,
    pub exp: i32,
    pub str: i16,
    pub max_hp: i16,
    pub ap: i16,
    pub sp: i16,
    /// Skill levels by skill id.
    pub skills: std::collections::BTreeMap<i32, i32>,
    pub quests: std::collections::BTreeMap<i16, u8>,
    /// Progress strings (kill counts) of started quests.
    pub quest_progress: std::collections::BTreeMap<i16, String>,
    /// Equipped item ids by equipped slot, from the world-entry SET_FIELD.
    pub equipped: std::collections::BTreeMap<i16, i32>,
    /// How many of each item the bags hold.
    pub items: std::collections::BTreeMap<i32, i32>,
    pub meso: i32,
    pub monster_book_cover: i32,
    /// Copies of each Monster Book card, by card id.
    pub monster_book: std::collections::BTreeMap<i32, u8>,
    /// `(kind, action)` for each of the 90 keys, from the world-entry KEYMAP.
    pub keymap: Vec<(u8, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectResult {
    pub character_id: i32,
    pub redirect_ip: Ipv4Addr,
    pub redirect_port: u16,
}

pub async fn login_to_world(config: &HarnessConfig) -> Result<WorldEntryResult, HarnessError> {
    let session = login_to_world_session(config).await?;
    Ok(WorldEntryResult {
        character_id: session.character_id,
        character_name: session.character_name,
        map_id: session.map_id,
        login_addr: session.login_addr,
        world_addr: session.world_addr,
    })
}

pub async fn login_to_redirect(config: &HarnessConfig) -> Result<RedirectResult, HarnessError> {
    let (_, redirect) = login_until_redirect(config).await?;
    Ok(redirect)
}

pub async fn login_to_world_session(config: &HarnessConfig) -> Result<WorldSession, HarnessError> {
    let (selected, redirect) = login_until_redirect(config).await?;

    // Like a real client, connect wherever the login server redirected us.
    let world_addr = SocketAddr::new(redirect.redirect_ip.into(), redirect.redirect_port);
    let mut world_conn = MapleTestConnection::connect(world_addr, "world handshake").await?;
    assert_handshake(
        "world handshake",
        world_conn.endpoint(),
        world_conn.handshake().version,
        world_conn.handshake().locale,
    )?;

    world_conn
        .send_packet(
            build_player_logged_in(selected.id)
                .map_err(|message| HarnessError::protocol("world entry", world_addr, message))?,
            "world entry",
        )
        .await?;

    let mut keymap = None;
    let mut world_entry = None;

    for _ in 0..8 {
        let envelope = read_packet(&mut world_conn, "world entry").await?;
        match envelope.opcode() {
            x if x == SendOpcode::KeyMap as i16 => {
                keymap = Some(decode_keymap(&envelope.packet).map_err(|message| {
                    HarnessError::protocol("world entry", world_conn.endpoint(), message)
                })?);
            }
            x if x == SendOpcode::SetField as i16 => {
                let set_field = decode_set_field(&envelope.packet).map_err(|message| {
                    HarnessError::protocol("world entry", world_conn.endpoint(), message)
                })?;
                if set_field.character_id != selected.id {
                    return Err(HarnessError::protocol(
                        "world entry",
                        world_conn.endpoint(),
                        format!(
                            "expected SetField character id {} but got {}",
                            selected.id, set_field.character_id
                        ),
                    ));
                }
                if set_field.character_name != selected.name {
                    return Err(HarnessError::protocol(
                        "world entry",
                        world_conn.endpoint(),
                        format!(
                            "expected SetField character name `{}` but got `{}`",
                            selected.name, set_field.character_name
                        ),
                    ));
                }
                world_entry = Some(set_field);
            }
            opcode => {
                return Err(HarnessError::protocol(
                    "world entry",
                    world_conn.endpoint(),
                    format!(
                        "unexpected opcode {} ({}) during world entry",
                        opcode,
                        opcode_name(opcode)
                    ),
                ));
            }
        }

        if keymap.is_some() && world_entry.is_some() {
            break;
        }
    }

    let world_entry = world_entry.ok_or_else(|| {
        HarnessError::protocol(
            "world entry",
            world_conn.endpoint(),
            "did not receive SetField during world entry".to_string(),
        )
    })?;

    let Some(keymap) = keymap else {
        return Err(HarnessError::protocol(
            "world entry",
            world_conn.endpoint(),
            "did not receive KeyMap during world entry".to_string(),
        ));
    };

    drain_spawn_npcs(&mut world_conn).await;

    Ok(WorldSession {
        connection: world_conn,
        character_id: world_entry.character_id,
        level: world_entry.level,
        exp: world_entry.exp,
        str: world_entry.str,
        max_hp: world_entry.max_hp,
        ap: world_entry.ap,
        sp: world_entry.sp,
        skills: world_entry.skills,
        quests: world_entry.quests,
        quest_progress: world_entry.quest_progress,
        equipped: world_entry.equipped,
        items: world_entry.items,
        meso: world_entry.meso,
        monster_book_cover: world_entry.monster_book_cover,
        monster_book: world_entry.monster_book,
        keymap,
        character_name: world_entry.character_name,
        map_id: world_entry.map_id,
        login_addr: config.login_addr.to_string(),
        world_addr: format!("{}:{}", redirect.redirect_ip, redirect.redirect_port),
    })
}

/// Ask the login server whether `name` is available for a new character.
///
/// Logs in and ensures the configured character exists first, so callers can
/// probe variations of an existing name.
pub async fn check_char_name_available(
    config: &HarnessConfig,
    name: &str,
) -> Result<bool, HarnessError> {
    let (mut login_conn, _) = login_until_char_select(config).await?;

    login_conn
        .send_packet(
            build_check_char_name(name).map_err(|message| {
                HarnessError::protocol("check char name", login_conn.endpoint(), message)
            })?,
            "check char name",
        )
        .await?;

    let response = read_packet(&mut login_conn, "check char name").await?;
    let decoded = decode_char_name_response(&response.packet).map_err(|message| {
        HarnessError::protocol("check char name", login_conn.endpoint(), message)
    })?;
    if decoded.name != name {
        return Err(HarnessError::protocol(
            "check char name",
            login_conn.endpoint(),
            format!(
                "expected char-name response for `{}` but got `{}`",
                name, decoded.name
            ),
        ));
    }
    Ok(decoded.available)
}

async fn login_until_redirect(
    config: &HarnessConfig,
) -> Result<(CharacterSummary, RedirectResult), HarnessError> {
    let (mut login_conn, selected) = login_until_char_select(config).await?;

    login_conn
        .send_packet(
            build_char_select(selected.id).map_err(|message| {
                HarnessError::protocol("char select", config.login_addr, message)
            })?,
            "char select",
        )
        .await?;

    let redirect_packet = read_packet(&mut login_conn, "server redirect").await?;
    let redirect = decode_server_redirect(&redirect_packet.packet).map_err(|message| {
        HarnessError::protocol("server redirect", login_conn.endpoint(), message)
    })?;
    assert_redirect_target(config, login_conn.endpoint(), redirect.ip, redirect.port)?;
    if redirect.character_id != selected.id {
        return Err(HarnessError::protocol(
            "server redirect",
            login_conn.endpoint(),
            format!(
                "expected redirect character id {} but got {}",
                selected.id, redirect.character_id
            ),
        ));
    }
    Ok((
        selected,
        RedirectResult {
            character_id: redirect.character_id,
            redirect_ip: redirect.ip,
            redirect_port: redirect.port,
        },
    ))
}

async fn login_until_char_select(
    config: &HarnessConfig,
) -> Result<(MapleTestConnection, CharacterSummary), HarnessError> {
    let mut login_conn = MapleTestConnection::connect(config.login_addr, "login handshake").await?;
    assert_handshake(
        "login handshake",
        login_conn.endpoint(),
        login_conn.handshake().version,
        login_conn.handshake().locale,
    )?;

    login_conn
        .send_packet(build_login_started(), "login start")
        .await?;
    login_conn
        .send_packet(
            build_login_credentials(&config.username, &config.password).map_err(|message| {
                HarnessError::protocol("login credentials", config.login_addr, message)
            })?,
            "login credentials",
        )
        .await?;

    let login_status = resolve_login_prompts(&mut login_conn, config).await?;
    if login_status.gender != Some(config.gender) && !matches!(login_status.gender, Some(0 | 1)) {
        return Err(HarnessError::protocol(
            "login credentials",
            login_conn.endpoint(),
            format!(
                "login completed with unresolved gender state {:?}",
                login_status.gender
            ),
        ));
    }

    login_conn
        .send_packet(build_server_list_request(), "server list request")
        .await?;

    let world_details = read_packet(&mut login_conn, "server list response").await?;
    assert_server_list_kind(
        "server list response",
        &world_details,
        login_conn.endpoint(),
        ServerListPacket::WorldDetails,
    )?;

    let world_end = read_packet(&mut login_conn, "server list terminator").await?;
    assert_server_list_kind(
        "server list terminator",
        &world_end,
        login_conn.endpoint(),
        ServerListPacket::EndOfList,
    )?;

    let last_connected = read_packet(&mut login_conn, "last connected world").await?;
    assert_opcode(
        "last connected world",
        &last_connected,
        login_conn.endpoint(),
        SendOpcode::LastConnectedWorld as i16,
    )?;
    let _ = decode_last_connected_world(&last_connected.packet).map_err(|message| {
        HarnessError::protocol("last connected world", login_conn.endpoint(), message)
    })?;

    let recommended = read_packet(&mut login_conn, "recommended worlds").await?;
    assert_opcode(
        "recommended worlds",
        &recommended,
        login_conn.endpoint(),
        SendOpcode::RecommendedWorlds as i16,
    )?;
    decode_recommended_worlds(&recommended.packet).map_err(|message| {
        HarnessError::protocol("recommended worlds", login_conn.endpoint(), message)
    })?;

    let selected = load_or_create_character(&mut login_conn, config).await?;
    Ok((login_conn, selected))
}

async fn drain_spawn_npcs(connection: &mut MapleTestConnection) {
    while let Ok(Ok(envelope)) = timeout(
        Duration::from_millis(100),
        connection.read_packet("drain spawn npcs"),
    )
    .await
    {
        // Map setup: NPC spawns and the map entry banner.
        // ...and the map's mobs, drops, and reactors.
        if ![
            SendOpcode::SpawnNpc as i16,
            SendOpcode::FieldEffect as i16,
            SendOpcode::SpawnMonster as i16,
            SendOpcode::SpawnMonsterControl as i16,
            SendOpcode::DropItemFromMapObject as i16,
            SendOpcode::SpawnReactor as i16,
        ]
        .contains(&envelope.opcode())
        {
            connection.push_back_packet(envelope);
            break;
        }
    }
}

/// Log in again right after dropping a session. The server closes the old
/// session asynchronously, so an immediate login may still be refused as a
/// duplicate (status 7); retry for a few seconds.
pub async fn relogin(config: &HarnessConfig) -> WorldSession {
    let mut last_error = None;
    for _ in 0..20 {
        match login_to_world_session(config).await {
            Ok(session) => return session,
            Err(error) => last_error = Some(error),
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("relogin failed: {last_error:?}");
}

pub async fn login_two_players_to_world(
    config: &MultiHarnessConfig,
) -> Result<(WorldSession, WorldSession), HarnessError> {
    let sender = config.player("sender")?;
    let recipient = config.player("recipient")?;

    let (sender_result, recipient_result) = tokio::join!(
        login_to_world_session(&sender),
        login_to_world_session(&recipient)
    );

    match (sender_result, recipient_result) {
        (Ok(sender_session), Ok(recipient_session)) => Ok((sender_session, recipient_session)),
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error),
    }
}

async fn read_packet(
    connection: &mut MapleTestConnection,
    phase: &'static str,
) -> Result<crate::connection::PacketEnvelope, HarnessError> {
    timeout(Duration::from_secs(5), connection.read_packet(phase))
        .await
        .map_err(|_| {
            HarnessError::protocol(
                phase,
                connection.endpoint(),
                format!("timed out waiting for {}", phase),
            )
        })?
}

async fn resolve_login_prompts(
    login_conn: &mut MapleTestConnection,
    config: &HarnessConfig,
) -> Result<LoginStatusPacket, HarnessError> {
    let first_packet = read_packet(login_conn, "login credentials").await?;
    let mut login_status = decode_login_status(&first_packet.packet).map_err(|message| {
        HarnessError::protocol("login credentials", login_conn.endpoint(), message)
    })?;

    if login_status.status == 23 {
        login_conn
            .send_packet(
                build_accept_tos().map_err(|message| {
                    HarnessError::protocol("accept tos", login_conn.endpoint(), message)
                })?,
                "accept tos",
            )
            .await?;

        let tos_response = read_packet(login_conn, "accept tos").await?;
        login_status = decode_login_status(&tos_response.packet).map_err(|message| {
            HarnessError::protocol("accept tos", login_conn.endpoint(), message)
        })?;
    }

    if login_status.status != 0 {
        return Err(HarnessError::protocol(
            "login credentials",
            login_conn.endpoint(),
            format!(
                "expected successful login status 0 but got {}",
                login_status.status
            ),
        ));
    }

    if !matches!(login_status.gender, Some(0 | 1)) {
        login_conn
            .send_packet(
                build_set_gender(config.gender).map_err(|message| {
                    HarnessError::protocol("set gender", login_conn.endpoint(), message)
                })?,
                "set gender",
            )
            .await?;

        let gender_response = read_packet(login_conn, "set gender").await?;
        login_status = assert_login_success(&gender_response, login_conn.endpoint())?;
        if login_status.gender != Some(config.gender) {
            return Err(HarnessError::protocol(
                "set gender",
                login_conn.endpoint(),
                format!(
                    "expected account gender {} after set-gender flow but got {:?}",
                    config.gender, login_status.gender
                ),
            ));
        }
    }

    Ok(login_status)
}

async fn load_or_create_character(
    login_conn: &mut MapleTestConnection,
    config: &HarnessConfig,
) -> Result<CharacterSummary, HarnessError> {
    let char_list = request_char_list(login_conn, config, "char list request", "char list").await?;
    if let Some(character) = find_character(&char_list.characters, &config.character_name) {
        return Ok(character);
    }

    login_conn
        .send_packet(
            build_create_char(
                &config.character_name,
                CharacterTemplate::default_for_gender(config.gender),
            )
            .map_err(|message| {
                HarnessError::protocol("create char", login_conn.endpoint(), message)
            })?,
            "create char",
        )
        .await?;

    let create_response_packet = read_packet(login_conn, "create char").await?;
    let created = decode_new_character(&create_response_packet.packet)
        .map_err(|message| HarnessError::protocol("create char", login_conn.endpoint(), message))?;
    if created.status != 0 {
        return Err(HarnessError::protocol(
            "create char",
            login_conn.endpoint(),
            format!(
                "server returned new-character status {} for `{}`",
                created.status, config.character_name
            ),
        ));
    }
    if created.character.name != config.character_name {
        return Err(HarnessError::protocol(
            "create char",
            login_conn.endpoint(),
            format!(
                "expected created character `{}` but got `{}`",
                config.character_name, created.character.name
            ),
        ));
    }

    let char_list =
        request_char_list(login_conn, config, "char list refresh", "char list refresh").await?;
    find_character(&char_list.characters, &config.character_name).ok_or_else(|| {
        let names = char_list
            .characters
            .iter()
            .map(|character| character.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        HarnessError::fixture(
            "char list refresh",
            format!(
                "created character `{}` but it was not returned by the refreshed char list; available characters: [{}]",
                config.character_name, names
            ),
        )
    })
}

async fn request_char_list(
    login_conn: &mut MapleTestConnection,
    config: &HarnessConfig,
    request_phase: &'static str,
    response_phase: &'static str,
) -> Result<crate::packets::CharListPacket, HarnessError> {
    login_conn
        .send_packet(
            build_char_list_request(config.world_id, config.channel_id).map_err(|message| {
                HarnessError::protocol(request_phase, login_conn.endpoint(), message)
            })?,
            request_phase,
        )
        .await?;

    let char_list_packet = read_packet(login_conn, response_phase).await?;
    decode_char_list(&char_list_packet.packet)
        .map_err(|message| HarnessError::protocol(response_phase, login_conn.endpoint(), message))
}

fn find_character(
    characters: &[CharacterSummary],
    character_name: &str,
) -> Option<CharacterSummary> {
    characters
        .iter()
        .find(|character| character.name == character_name)
        .cloned()
}
