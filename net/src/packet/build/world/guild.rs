//! Guild packets: `GUILD_OPERATION`, the guild mode of `MULTICHAT`, the
//! name and emblem others see (`GUILD_NAME_CHANGED`, `GUILD_MARK_CHANGED`),
//! and members' level-ups and new jobs (`NOTIFY_LEVELUP`,
//! `NOTIFY_JOB_CHANGE`).

use crate::codec::stats::write_padded_name;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::guild::{Emblem, Guild, GuildMember, Standing};
use packet::{io::write::PktWrite, Packet};

const OP_NAME_PROMPT: u8 = 0x01;
const OP_INVITE: u8 = 0x05;
const OP_EMBLEM_MAKER: u8 = 0x11;
const OP_INFO: u8 = 0x1A;
const OP_CREATED: u8 = 0x20;
const OP_JOINED: u8 = 0x27;
const OP_LEFT: u8 = 0x2C;
const OP_EXPELLED: u8 = 0x2F;
const OP_DISBANDED: u8 = 0x32;
const OP_CAPACITY_CHANGED: u8 = 0x3A;
const OP_MEMBER_LEVEL_JOB: u8 = 0x3C;
const OP_MEMBER_ONLINE: u8 = 0x3D;
const OP_TITLES_CHANGED: u8 = 0x3E;
const OP_RANK_CHANGED: u8 = 0x40;
const OP_EMBLEM_CHANGED: u8 = 0x42;
const OP_NOTICE_CHANGED: u8 = 0x44;
const OP_RANKING: u8 = 0x49;

/// `MULTICHAT` mode for guild chat.
pub const MULTICHAT_GUILD: u8 = 2;

/// `NOTIFY_LEVELUP`'s type for a guild member; any other says "Family".
const LEVEL_UP_GUILD: u8 = 2;
/// `NOTIFY_JOB_CHANGE`'s type for a guild member (1 is family).
const JOB_CHANGE_GUILD: u8 = 0;

/// Why founding, joining, disbanding, or growing a guild failed; the client
/// shows each by code.
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
    /// Heracle says "The problem has happened during the process of
    /// disbanding the guild...".
    DisbandFailed = 0x34,
    /// Heracle says "The problem has happened during the process of
    /// increasing the guild...".
    GrowFailed = 0x3B,
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
    packet.write_int(i32::from(member.alliance_rank))?;
    Ok(())
}

/// `GUILDDATA`: the guild, its titles, and its members, each as a 37-byte
/// record. The client takes the first member for the master.
pub(crate) fn write_guild(packet: &mut Packet, guild: &Guild) -> Result<(), NetworkError> {
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

/// Open the emblem maker (Lea), which only the master's client does; it
/// answers with `GUILD_OPERATION` mode 0x0F.
pub fn build_emblem_maker() -> Result<Packet, NetworkError> {
    guild_operation(OP_EMBLEM_MAKER)
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

/// The guild is gone. The master's client has Heracle say so; the others
/// write "The guild has been disbanded." to the chat log.
pub fn build_guild_disbanded(guild_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_DISBANDED)?;
    packet.write_int(guild_id)?;
    Ok(packet)
}

/// The guild now takes `capacity` members. Only the master's client says
/// so, in Heracle's box.
pub fn build_capacity_changed(guild_id: i32, capacity: u8) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_CAPACITY_CHANGED)?;
    packet.write_int(guild_id)?;
    packet.write_byte(capacity)?;
    Ok(packet)
}

/// Member `member_id` now holds `rank`. The client writes "[name]'s
/// position has been changed to [title]." to the chat log.
pub fn build_rank_changed(guild_id: i32, member_id: i32, rank: u8) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_RANK_CHANGED)?;
    packet.write_int(guild_id)?;
    packet.write_int(member_id)?;
    packet.write_byte(rank)?;
    Ok(packet)
}

/// The ranks' new titles. The master's client says "It is saved.".
pub fn build_titles_changed(guild_id: i32, titles: &[String; 5]) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_TITLES_CHANGED)?;
    packet.write_int(guild_id)?;
    for title in titles {
        packet.write_str_with_length(title)?;
    }
    Ok(packet)
}

/// The guild's new emblem. Each member's client puts it beside their own
/// name; the master's says "It is completed.".
pub fn build_emblem_changed(guild_id: i32, emblem: &Emblem) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_EMBLEM_CHANGED)?;
    packet.write_int(guild_id)?;
    write_emblem(&mut packet, emblem)?;
    Ok(packet)
}

/// The guild's new notice, which the client also writes to the chat log.
pub fn build_notice_changed(guild_id: i32, notice: &str) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_NOTICE_CHANGED)?;
    packet.write_int(guild_id)?;
    packet.write_str_with_length(notice)?;
    Ok(packet)
}

/// The guild ranking window, as the board `npc_id` shows it: each guild's
/// name, points, and emblem, the mark before the frame, each part as an
/// int.
pub fn build_guild_ranking(npc_id: i32, guilds: &[Standing]) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_RANKING)?;
    packet.write_int(npc_id)?;
    packet.write_int(guilds.len() as i32)?;
    for guild in guilds {
        packet.write_str_with_length(&guild.name)?;
        packet.write_int(guild.points)?;
        packet.write_int(i32::from(guild.emblem.mark))?;
        packet.write_int(i32::from(guild.emblem.mark_color))?;
        packet.write_int(i32::from(guild.emblem.background))?;
        packet.write_int(i32::from(guild.emblem.background_color))?;
    }
    Ok(packet)
}

/// Member `member_id` is now at `level` with `job`. The client changes the
/// guild list without a word.
pub fn build_member_level_job(
    guild_id: i32,
    member_id: i32,
    level: u8,
    job: i16,
) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_MEMBER_LEVEL_JOB)?;
    packet.write_int(guild_id)?;
    packet.write_int(member_id)?;
    packet.write_int(i32::from(level))?;
    packet.write_int(i32::from(job))?;
    Ok(packet)
}

/// Member `member_id` entered or left the game. Entering shows the buddy
/// list's "<name> is logged in." card, whoever it names.
pub fn build_member_online(
    guild_id: i32,
    member_id: i32,
    online: bool,
) -> Result<Packet, NetworkError> {
    let mut packet = guild_operation(OP_MEMBER_ONLINE)?;
    packet.write_int(guild_id)?;
    packet.write_int(member_id)?;
    packet.write_byte(u8::from(online))?;
    Ok(packet)
}

/// "<Guild> [name] has reached Lv. [level]." in the chat log.
pub fn build_guild_level_up(level: u8, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NotifyLevelUp as i16)?;
    packet.write_byte(LEVEL_UP_GUILD)?;
    packet.write_int(i32::from(level))?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// "<Guild> [name] has advanced to a(an) [job]." in the chat log. The
/// client's text has no '>' after the group, so it goes with the name.
pub fn build_guild_job_change(job: i16, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NotifyJobChange as i16)?;
    packet.write_byte(JOB_CHANGE_GUILD)?;
    packet.write_int(i32::from(job))?;
    packet.write_str_with_length(&format!("> {name}"))?;
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
            alliance_rank: 5,
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
    fn rank_titles_and_notice_changes() {
        let packet = build_rank_changed(4, 2, 3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x40)
            .int(4)
            .int(2)
            .byte(3)
            .end();

        let packet = build_titles_changed(4, &guild().titles).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x3E)
            .int(4)
            .string("Master")
            .string("Jr. Master")
            .string("Member")
            .string("")
            .string("")
            .end();

        let packet = build_notice_changed(4, "hi").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x44)
            .int(4)
            .string("hi")
            .end();
    }

    #[test]
    fn member_online_level_and_job() {
        let packet = build_member_online(4, 2, true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x3D)
            .int(4)
            .int(2)
            .byte(1)
            .end();

        let packet = build_member_level_job(4, 2, 31, 110).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x3C)
            .int(4)
            .int(2)
            .int(31)
            .int(110)
            .end();

        let packet = build_guild_level_up(31, "Bobby").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x69).byte(2).int(31).string("Bobby").end();

        let packet = build_guild_job_change(110, "Bobby").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x6B).byte(0).int(110).string("> Bobby").end();
    }

    #[test]
    fn disbanded_and_capacity() {
        let packet = build_guild_disbanded(4).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x32)
            .int(4)
            .end();

        let packet = build_capacity_changed(4, 15).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x3A)
            .int(4)
            .byte(15)
            .end();

        for (error, mode) in [
            (GuildError::DisbandFailed, 0x34),
            (GuildError::GrowFailed, 0x3B),
        ] {
            let packet = build_guild_error(error).unwrap();
            let mut expect = Expect::new(&packet.bytes);
            expect
                .short(SendOpcode::GuildOperation as i16)
                .byte(mode)
                .end();
        }
    }

    #[test]
    fn emblem_maker_and_changed() {
        let packet = build_emblem_maker().unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x11)
            .end();

        let packet = build_emblem_changed(4, &guild().emblem).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildOperation as i16)
            .byte(0x42)
            .int(4)
            .short(1001)
            .byte(2)
            .short(2003)
            .byte(4)
            .end();
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
