//! Guild packets: `GUILD_OPERATION`, the guild mode of `MULTICHAT`, and the
//! name and emblem others see (`GUILD_NAME_CHANGED`, `GUILD_MARK_CHANGED`).

use crate::codec::stats::write_padded_name;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::guild::{Emblem, Guild};
use packet::{io::write::PktWrite, Packet};

const OP_NAME_PROMPT: u8 = 0x01;
const OP_INFO: u8 = 0x1A;
const OP_CREATED: u8 = 0x20;

/// `MULTICHAT` mode for guild chat.
pub const MULTICHAT_GUILD: u8 = 2;

/// The lowest alliance rank, which OdinMS gives every member; the client
/// reads it only in an alliance.
const ALLIANCE_RANK: i32 = 5;

/// Why founding a guild failed; the client shows each by code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildError {
    /// Heracle says "The name is already in use..." and asks again.
    NameTaken = 0x1C,
    /// "Already joined the guild."
    AlreadyInGuild = 0x21,
    /// "You cannot make a guild, due to the limitation of minimum level
    /// requirement."
    LevelTooLow = 0x23,
    /// "Admin cannot make a guild."
    Admin = 0x38,
}

fn guild_operation(op: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::GuildOperation as i16)?;
    packet.write_byte(op)?;
    Ok(packet)
}

/// The emblem as `SPAWN_PLAYER`, `GUILD_MARK_CHANGED`, and the guild data
/// carry it.
pub(crate) fn write_emblem(packet: &mut Packet, emblem: &Emblem) -> Result<(), NetworkError> {
    packet.write_short(emblem.background as i16)?;
    packet.write_byte(emblem.background_color)?;
    packet.write_short(emblem.mark as i16)?;
    packet.write_byte(emblem.mark_color)?;
    Ok(())
}

/// `GUILDDATA`: the guild, its titles, and its members, each as a 37-byte
/// record. The client takes the first member for the master.
fn write_guild(packet: &mut Packet, guild: &Guild) -> Result<(), NetworkError> {
    packet.write_int(guild.id)?;
    packet.write_str_with_length(&guild.name)?;
    for title in &guild.titles {
        packet.write_str_with_length(title)?;
    }
    packet.write_byte(guild.members.len() as u8)?;
    for member in &guild.members {
        packet.write_int(member.id)?;
    }
    for member in &guild.members {
        write_padded_name(packet, &member.name)?;
        packet.write_int(i32::from(member.job))?;
        packet.write_int(i32::from(member.level))?;
        packet.write_int(i32::from(member.rank))?;
        packet.write_int(i32::from(member.online))?;
        packet.write_int(0)?; // contribution
        packet.write_int(ALLIANCE_RANK)?;
    }
    packet.write_int(i32::from(guild.capacity))?;
    write_emblem(packet, &guild.emblem)?;
    packet.write_str_with_length(&guild.notice)?;
    packet.write_int(guild.points)?;
    packet.write_int(guild.alliance_id)?;
    Ok(())
}

/// Ask for the new guild's name; the client answers with
/// `GUILD_OPERATION` mode 2.
pub fn build_name_prompt() -> Result<Packet, NetworkError> {
    guild_operation(OP_NAME_PROMPT)
}

/// The player's guild, or none, replacing what the client knows.
pub fn build_guild_info(guild: Option<&Guild>) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_INFO)?;
    packet.write_byte(u8::from(guild.is_some()))?;
    if let Some(guild) = guild {
        write_guild(&mut packet, guild)?;
    }
    Ok(packet)
}

/// The guild was founded; Heracle congratulates the master.
pub fn build_guild_created(guild: &Guild) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_CREATED)?;
    write_guild(&mut packet, guild)?;
    Ok(packet)
}

pub fn build_guild_error(error: GuildError) -> Result<Packet, NetworkError> {
    guild_operation(error as u8)
}

pub fn build_guild_chat(name: &str, text: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MultiChat as i16)?;
    packet.write_byte(MULTICHAT_GUILD)?;
    packet.write_str_with_length(name)?;
    packet.write_str_with_length(text)?;
    Ok(packet)
}

/// The guild name others see under `cid`'s, or "" for none.
pub fn build_guild_name_changed(cid: i32, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::GuildNameChanged as i16)?;
    packet.write_int(cid)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// The emblem others see beside `cid`'s guild name.
pub fn build_guild_mark_changed(cid: i32, emblem: &Emblem) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::GuildMarkChanged as i16)?;
    packet.write_int(cid)?;
    write_emblem(&mut packet, emblem)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::guild::GuildMember;

    fn padded(text: &str, len: usize) -> Vec<u8> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.resize(len, 0);
        bytes
    }

    fn guild() -> Guild {
        let member = |id, name: &str, rank, online| GuildMember {
            id,
            name: name.into(),
            job: 100,
            level: 30,
            rank,
            online,
        };
        Guild {
            id: 4,
            name: "Testers".into(),
            titles: ["Master", "Jr. Master", "Member", "", ""].map(String::from),
            capacity: 10,
            emblem: Emblem {
                background: 1001,
                background_color: 2,
                mark: 2003,
                mark_color: 4,
            },
            notice: "Hi".into(),
            points: 7,
            alliance_id: 0,
            members: vec![member(1, "Alice", 1, true), member(2, "Bobby", 3, false)],
        }
    }

    fn expect_guild(expect: &mut Expect) {
        expect.int(4).string("Testers");
        for title in ["Master", "Jr. Master", "Member", "", ""] {
            expect.string(title);
        }
        expect.byte(2).int(1).int(2);
        expect
            .bytes(&padded("Alice", 13))
            .int(100)
            .int(30)
            .int(1)
            .int(1)
            .int(0)
            .int(5);
        expect
            .bytes(&padded("Bobby", 13))
            .int(100)
            .int(30)
            .int(3)
            .int(0)
            .int(0)
            .int(5);
        expect
            .int(10)
            .short(1001)
            .byte(2)
            .short(2003)
            .byte(4)
            .string("Hi")
            .int(7)
            .int(0);
    }

    #[test]
    fn info_carries_the_guild_data_or_none() {
        let packet = build_guild_info(Some(&guild())).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x1A)
            .byte(1);
        expect_guild(&mut expect);
        expect.end();

        let packet = build_guild_info(None).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x1A)
            .byte(0)
            .end();
    }

    #[test]
    fn created_carries_the_guild_data() {
        let packet = build_guild_created(&guild()).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GuildOperation as i16).byte(0x20);
        expect_guild(&mut expect);
        expect.end();
    }

    #[test]
    fn prompt_and_errors_are_bare_modes() {
        for (packet, mode) in [
            (build_name_prompt(), 0x01),
            (build_guild_error(GuildError::NameTaken), 0x1C),
            (build_guild_error(GuildError::AlreadyInGuild), 0x21),
            (build_guild_error(GuildError::LevelTooLow), 0x23),
            (build_guild_error(GuildError::Admin), 0x38),
        ] {
            let packet = packet.unwrap();
            let mut expect = Expect::new(&packet.bytes);
            expect
                .short(SendOpcode::GuildOperation as i16)
                .byte(mode)
                .end();
        }
    }

    #[test]
    fn chat_name_and_mark() {
        let packet = build_guild_chat("Alice", "hello").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::MultiChat as i16)
            .byte(2)
            .string("Alice")
            .string("hello")
            .end();

        let packet = build_guild_name_changed(9, "Testers").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildNameChanged as i16)
            .int(9)
            .string("Testers")
            .end();

        let packet = build_guild_mark_changed(9, &guild().emblem).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildMarkChanged as i16)
            .int(9)
            .short(1001)
            .byte(2)
            .short(2003)
            .byte(4)
            .end();
    }
}
