//! Guild packets: `GUILD_OPERATION`, the guild mode of `MULTICHAT`, and the
//! name and emblem others see (`GUILD_NAME_CHANGED`, `GUILD_MARK_CHANGED`).

use crate::codec::stats::write_padded_name;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::guild::{Emblem, Guild, GuildMember};
use packet::{io::write::PktWrite, Packet};

const OP_NAME_PROMPT: u8 = 0x01;
const OP_INVITE: u8 = 0x05;
const OP_INFO: u8 = 0x1A;
const OP_CREATED: u8 = 0x20;
const OP_JOINED: u8 = 0x27;
const OP_LEFT: u8 = 0x2C;
const OP_EXPELLED: u8 = 0x2F;

/// `MULTICHAT` mode for guild chat.
pub const MULTICHAT_GUILD: u8 = 2;

/// The lowest alliance rank, which OdinMS gives every member; the client
/// reads it only in an alliance.
const ALLIANCE_RANK: i32 = 5;

/// Why founding or joining a guild failed; the client shows each by code.
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
    /// The invitee is already in a guild: "Already joined the guild."
    InviteeInGuild = 0x28,
    /// To the invitee: "The guild, you are trying to join, has already
    /// reached the max number of users."
    Full = 0x29,
    /// "The character cannot be found in the current channel."
    NotInChannel = 0x2A,
}

/// Why an invitation was turned down. The invitee's client sends the code
/// in `DENY_GUILD_REQUEST`, and the inviter's shows it as the same mode of
/// `GUILD_OPERATION`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InviteRefusal {
    /// "%s is currently not accepting guild invite message."
    Busy = 0x35,
    /// "'%s' is taking care of another invitation."
    AnotherInvitation = 0x36,
    /// "%s has denied your guild invitation."
    Declined = 0x37,
}

impl InviteRefusal {
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0x35 => Some(Self::Busy),
            0x36 => Some(Self::AnotherInvitation),
            0x37 => Some(Self::Declined),
            _ => None,
        }
    }
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

/// A member's 37-byte record.
fn write_member(packet: &mut Packet, member: &GuildMember) -> Result<(), NetworkError> {
    write_padded_name(packet, &member.name)?;
    packet.write_int(i32::from(member.job))?;
    packet.write_int(i32::from(member.level))?;
    packet.write_int(i32::from(member.rank))?;
    packet.write_int(i32::from(member.online))?;
    packet.write_int(0)?; // contribution
    packet.write_int(ALLIANCE_RANK)?;
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
        write_member(packet, member)?;
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

/// `inviter` asks the player to join guild `guild_id`; the client answers
/// with `GUILD_OPERATION` mode 6 or `DENY_GUILD_REQUEST`.
pub fn build_guild_invite(guild_id: i32, inviter: &str) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_INVITE)?;
    packet.write_int(guild_id)?;
    packet.write_str_with_length(inviter)?;
    Ok(packet)
}

/// `name` turned the player's invitation down.
pub fn build_invite_refused(refusal: InviteRefusal, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(refusal as u8)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// `member` joined. The members add them to their list; the newcomer's
/// client skips the record and asks for the whole guild.
pub fn build_member_joined(guild_id: i32, member: &GuildMember) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_JOINED)?;
    packet.write_int(guild_id)?;
    packet.write_int(member.id)?;
    write_member(&mut packet, member)?;
    Ok(packet)
}

/// Member `member_id` left the guild.
pub fn build_member_left(
    guild_id: i32,
    member_id: i32,
    name: &str,
) -> Result<Packet, NetworkError> {
    departure(OP_LEFT, guild_id, member_id, name)
}

/// Member `member_id` was put out of the guild.
pub fn build_member_expelled(
    guild_id: i32,
    member_id: i32,
    name: &str,
) -> Result<Packet, NetworkError> {
    departure(OP_EXPELLED, guild_id, member_id, name)
}

fn departure(op: u8, guild_id: i32, member_id: i32, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(op)?;
    packet.write_int(guild_id)?;
    packet.write_int(member_id)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
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
            (build_guild_error(GuildError::InviteeInGuild), 0x28),
            (build_guild_error(GuildError::Full), 0x29),
            (build_guild_error(GuildError::NotInChannel), 0x2A),
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
    fn invite_and_refusals() {
        let packet = build_guild_invite(4, "Alice").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x05)
            .int(4)
            .string("Alice")
            .end();

        for (refusal, mode) in [
            (InviteRefusal::Busy, 0x35),
            (InviteRefusal::AnotherInvitation, 0x36),
            (InviteRefusal::Declined, 0x37),
        ] {
            assert_eq!(InviteRefusal::from_code(mode), Some(refusal));
            let packet = build_invite_refused(refusal, "Bobby").unwrap();
            let mut expect = Expect::new(&packet.bytes);
            expect
                .short(SendOpcode::GuildOperation as i16)
                .byte(mode)
                .string("Bobby")
                .end();
        }
    }

    #[test]
    fn joined_carries_the_member_and_departures_the_name() {
        let packet = build_member_joined(4, &guild().members[1]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x27)
            .int(4)
            .int(2)
            .bytes(&padded("Bobby", 13))
            .int(100)
            .int(30)
            .int(3)
            .int(0)
            .int(0)
            .int(5)
            .end();

        for (packet, mode) in [
            (build_member_left(4, 2, "Bobby"), 0x2C),
            (build_member_expelled(4, 2, "Bobby"), 0x2F),
        ] {
            let packet = packet.unwrap();
            let mut expect = Expect::new(&packet.bytes);
            expect
                .short(SendOpcode::GuildOperation as i16)
                .byte(mode)
                .int(4)
                .int(2)
                .string("Bobby")
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
