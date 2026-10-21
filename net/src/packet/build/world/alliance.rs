//! Alliance packets: `ALLIANCE_OPERATION` (the client's handler 0xa39f4e,
//! report b023) and the alliance mode of `MULTICHAT`.
//!
//! The client keeps the alliance's other guilds in a list apart from its
//! own guild, so the packets that carry guilds carry `GUILDDATA` as the
//! guild packets do.

use super::guild::write_guild;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::alliance::Alliance;
use game::guild::Guild;
use packet::{io::write::PktWrite, Packet};

const OP_INVITE: u8 = 0x03;
const OP_MEMBER_ONLINE: u8 = 0x0E;
const OP_INFO: u8 = 0x0F;
const OP_GUILD_LEFT: u8 = 0x10;
const OP_GUILD_JOINED: u8 = 0x12;
const OP_CHANGED: u8 = 0x17;
const OP_MEMBER_LEVEL_JOB: u8 = 0x18;
const OP_MASTER_CHANGED: u8 = 0x19;
const OP_TITLES_CHANGED: u8 = 0x1A;
const OP_RANK_CHANGED: u8 = 0x1B;
const OP_NOTICE_CHANGED: u8 = 0x1C;
const OP_DISBANDED: u8 = 0x1D;
const OP_GUILD_CHANGED: u8 = 0x1E;

/// `MULTICHAT` mode for alliance chat.
pub const MULTICHAT_ALLIANCE: u8 = 3;

fn alliance_operation(op: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::AllianceOperation as i16)?;
    packet.write_byte(op)?;
    Ok(packet)
}

/// The alliance as the client decodes it (0x4e475a): its guild ids come
/// before the capacity.
fn write_alliance(packet: &mut Packet, alliance: &Alliance) -> Result<(), NetworkError> {
    packet.write_int(alliance.id)?;
    packet.write_str_with_length(&alliance.name)?;
    for title in &alliance.titles {
        packet.write_str_with_length(title)?;
    }
    packet.write_byte(alliance.guilds.len() as u8)?;
    for guild in &alliance.guilds {
        packet.write_int(guild.id)?;
    }
    packet.write_int(i32::from(alliance.capacity))?;
    packet.write_str_with_length(&alliance.notice)?;
    Ok(())
}

/// The card inviting the master of a guild into `alliance_id`, from
/// `inviter`. The client answers with `ALLIANCE_OPERATION` op 4 or with
/// `DENY_ALLIANCE_REQUEST`.
pub fn build_alliance_invite(alliance_id: i32, inviter: &str) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_INVITE)?;
    packet.write_int(alliance_id)?;
    packet.write_str_with_length(inviter)?;
    packet.write_str_with_length("")?;
    Ok(packet)
}

/// The whole alliance with every guild in it, replacing what the client
/// had.
pub fn build_alliance_info(alliance: &Alliance) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_INFO)?;
    write_alliance(&mut packet, alliance)?;
    for guild in &alliance.guilds {
        write_guild(&mut packet, guild)?;
    }
    Ok(packet)
}

/// `guild` left the alliance, which is now `alliance`, or was `expelled`.
/// Its members' clients take `guild`, out of the alliance, for their own
/// and say why in the chat log; the others drop it from their list.
pub fn build_guild_left(
    alliance: &Alliance,
    guild: &Guild,
    expelled: bool,
) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_GUILD_LEFT)?;
    write_alliance(&mut packet, alliance)?;
    packet.write_int(guild.id)?;
    write_guild(&mut packet, guild)?;
    packet.write_byte(u8::from(expelled))?;
    Ok(packet)
}

/// `guild` joined `alliance`, which holds it now. Every member's client
/// says so in the chat log, and the new guild's ask for the alliance
/// (`ALLIANCE_OPERATION` op 1).
pub fn build_guild_joined(alliance: &Alliance, guild: &Guild) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_GUILD_JOINED)?;
    write_alliance(&mut packet, alliance)?;
    packet.write_int(guild.id)?;
    write_guild(&mut packet, guild)?;
    Ok(packet)
}

/// The alliance's name, titles, guild ids, capacity, or notice changed;
/// its guilds did not.
pub fn build_alliance_changed(alliance: &Alliance) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_CHANGED)?;
    write_alliance(&mut packet, alliance)?;
    Ok(packet)
}

/// A member of another guild in the alliance entered or left the game.
/// Entering shows a "<name> is logged in." card.
pub fn build_member_online(
    alliance_id: i32,
    guild_id: i32,
    member_id: i32,
    online: bool,
) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_MEMBER_ONLINE)?;
    packet.write_int(alliance_id)?;
    packet.write_int(guild_id)?;
    packet.write_int(member_id)?;
    packet.write_byte(u8::from(online))?;
    Ok(packet)
}

/// A member of another guild in the alliance has a new level or job.
pub fn build_member_level_job(
    alliance_id: i32,
    guild_id: i32,
    member_id: i32,
    level: u8,
    job: i16,
) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_MEMBER_LEVEL_JOB)?;
    packet.write_int(alliance_id)?;
    packet.write_int(guild_id)?;
    packet.write_int(member_id)?;
    packet.write_int(i32::from(level))?;
    packet.write_int(i32::from(job))?;
    Ok(packet)
}

/// `to` is the alliance's master instead of `from`, who keeps rank 2.
pub fn build_master_changed(alliance_id: i32, from: i32, to: i32) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_MASTER_CHANGED)?;
    packet.write_int(alliance_id)?;
    packet.write_int(from)?;
    packet.write_int(to)?;
    Ok(packet)
}

pub fn build_titles_changed(
    alliance_id: i32,
    titles: &[String; 5],
) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_TITLES_CHANGED)?;
    packet.write_int(alliance_id)?;
    for title in titles {
        packet.write_str_with_length(title)?;
    }
    Ok(packet)
}

/// Member `member_id` has alliance rank `rank`. The client finds them by
/// id alone.
pub fn build_rank_changed(member_id: i32, rank: u8) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_RANK_CHANGED)?;
    packet.write_int(member_id)?;
    packet.write_byte(rank)?;
    Ok(packet)
}

pub fn build_notice_changed(alliance_id: i32, notice: &str) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_NOTICE_CHANGED)?;
    packet.write_int(alliance_id)?;
    packet.write_str_with_length(notice)?;
    Ok(packet)
}

/// The alliance is gone; the client empties the tab without a word.
pub fn build_alliance_disbanded(alliance_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_DISBANDED)?;
    packet.write_int(alliance_id)?;
    Ok(packet)
}

/// One of the alliance's guilds changed: its members, ranks, or emblem.
pub fn build_guild_changed(alliance_id: i32, guild: &Guild) -> Result<Packet, NetworkError> {
    let mut packet = alliance_operation(OP_GUILD_CHANGED)?;
    packet.write_int(alliance_id)?;
    packet.write_int(guild.id)?;
    write_guild(&mut packet, guild)?;
    Ok(packet)
}

pub fn build_alliance_chat(name: &str, text: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MultiChat as i16)?;
    packet.write_byte(MULTICHAT_ALLIANCE)?;
    packet.write_str_with_length(name)?;
    packet.write_str_with_length(text)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::guild::{Emblem, GuildMember};

    fn guild(id: i32, alliance_rank: u8) -> Guild {
        Guild {
            id,
            name: format!("Guild{id}"),
            titles: game::guild::DEFAULT_TITLES.map(String::from),
            capacity: 10,
            emblem: Emblem::default(),
            notice: String::new(),
            points: 0,
            alliance_id: 7,
            members: vec![GuildMember {
                id: id * 10,
                name: format!("Master{id}"),
                job: 100,
                level: 30,
                rank: 1,
                alliance_rank,
                online: true,
            }],
        }
    }

    fn alliance() -> Alliance {
        Alliance {
            id: 7,
            name: "Union".into(),
            titles: game::alliance::DEFAULT_TITLES.map(String::from),
            capacity: 3,
            notice: "Hi".into(),
            guilds: vec![guild(1, 1), guild(2, 2)],
        }
    }

    fn guild_data(guild: &Guild) -> Vec<u8> {
        let mut packet = Packet::new_empty();
        write_guild(&mut packet, guild).unwrap();
        packet.bytes
    }

    fn expect_alliance(expect: &mut Expect, ids: &[i32]) {
        expect.int(7).string("Union");
        for title in game::alliance::DEFAULT_TITLES {
            expect.string(title);
        }
        expect.byte(ids.len() as u8);
        for &id in ids {
            expect.int(id);
        }
        expect.int(3).string("Hi");
    }

    fn expect_op(packet: &Packet, op: u8) -> Expect<'_> {
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::AllianceOperation as i16).byte(op);
        expect
    }

    #[test]
    fn info_carries_every_guild_after_the_ids_and_capacity() {
        let union = alliance();
        let packet = build_alliance_info(&union).unwrap();
        let mut expect = expect_op(&packet, 0x0F);
        expect_alliance(&mut expect, &[1, 2]);
        expect
            .bytes(&guild_data(&union.guilds[0]))
            .bytes(&guild_data(&union.guilds[1]))
            .end();
    }

    #[test]
    fn joins_and_departures_carry_the_guild() {
        let union = alliance();
        let packet = build_guild_joined(&union, &union.guilds[1]).unwrap();
        let mut expect = expect_op(&packet, 0x12);
        expect_alliance(&mut expect, &[1, 2]);
        expect.int(2).bytes(&guild_data(&union.guilds[1])).end();

        let mut rest = alliance();
        let gone = rest.guilds.pop().unwrap();
        let packet = build_guild_left(&rest, &gone, true).unwrap();
        let mut expect = expect_op(&packet, 0x10);
        expect_alliance(&mut expect, &[1]);
        expect.int(2).bytes(&guild_data(&gone)).byte(1).end();

        let packet = build_guild_changed(7, &gone).unwrap();
        expect_op(&packet, 0x1E)
            .int(7)
            .int(2)
            .bytes(&guild_data(&gone))
            .end();
    }

    #[test]
    fn invite_and_small_changes() {
        let packet = build_alliance_invite(7, "Alice").unwrap();
        expect_op(&packet, 0x03)
            .int(7)
            .string("Alice")
            .string("")
            .end();

        let packet = build_alliance_changed(&alliance()).unwrap();
        let mut expect = expect_op(&packet, 0x17);
        expect_alliance(&mut expect, &[1, 2]);
        expect.end();

        let packet = build_member_online(7, 2, 20, true).unwrap();
        expect_op(&packet, 0x0E).int(7).int(2).int(20).byte(1).end();
        let packet = build_member_level_job(7, 2, 20, 31, 110).unwrap();
        expect_op(&packet, 0x18)
            .int(7)
            .int(2)
            .int(20)
            .int(31)
            .int(110)
            .end();
        let packet = build_master_changed(7, 10, 20).unwrap();
        expect_op(&packet, 0x19).int(7).int(10).int(20).end();
        let packet = build_rank_changed(20, 4).unwrap();
        expect_op(&packet, 0x1B).int(20).byte(4).end();
        let packet = build_notice_changed(7, "Yo").unwrap();
        expect_op(&packet, 0x1C).int(7).string("Yo").end();
        let packet = build_alliance_disbanded(7).unwrap();
        expect_op(&packet, 0x1D).int(7).end();

        let titles = ["A", "B", "C", "D", "E"].map(String::from);
        let packet = build_titles_changed(7, &titles).unwrap();
        let mut expect = expect_op(&packet, 0x1A);
        expect.int(7);
        for title in &titles {
            expect.string(title);
        }
        expect.end();
    }

    #[test]
    fn chat_is_multichat_mode_3() {
        let packet = build_alliance_chat("Alice", "hello").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::MultiChat as i16)
            .byte(3)
            .string("Alice")
            .string("hello")
            .end();
    }
}
