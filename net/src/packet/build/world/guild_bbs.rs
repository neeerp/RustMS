//! `GUILD_BBS` (0x3B): the guild bulletin board's list and threads
//! (0x816c32; scratch/251/bbs.md, section 3). Dates are UTC FILETIMEs,
//! which the client prints as they are.

use crate::codec::time::filetime;
use crate::{error::NetworkError, packet::op::SendOpcode};
use db::guild_bbs::{Page, Summary, Thread};
use packet::{io::write::PktWrite, Packet};

const LIST: u8 = 6;
const THREAD: u8 = 7;
const MISSING: u8 = 8;

fn bbs(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::GuildBbs as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

fn write_summary(packet: &mut Packet, thread: &Summary) -> Result<(), NetworkError> {
    packet.write_int(thread.id)?;
    packet.write_int(thread.poster)?;
    packet.write_str_with_length(&thread.title)?;
    packet.write_long(filetime(thread.posted_at))?;
    packet.write_int(i32::from(thread.icon))?;
    packet.write_int(thread.replies)?;
    Ok(())
}

/// One page of the list (mode 6): the notice, the number of other threads,
/// and the page's threads.
pub fn build_bbs_list(page: &Page) -> Result<Packet, NetworkError> {
    let mut packet = bbs(LIST)?;
    packet.write_byte(u8::from(page.notice.is_some()))?;
    if let Some(notice) = &page.notice {
        write_summary(&mut packet, notice)?;
    }
    packet.write_int(page.total)?;
    // The client reads the page only if there are threads.
    if page.total > 0 {
        packet.write_int(page.threads.len() as i32)?;
        for thread in &page.threads {
            write_summary(&mut packet, thread)?;
        }
    }
    Ok(packet)
}

/// A thread with its replies (mode 7).
pub fn build_bbs_thread(thread: &Thread) -> Result<Packet, NetworkError> {
    let mut packet = bbs(THREAD)?;
    packet.write_int(thread.id)?;
    packet.write_int(thread.poster)?;
    packet.write_long(filetime(thread.posted_at))?;
    packet.write_str_with_length(&thread.title)?;
    packet.write_str_with_length(&thread.body)?;
    packet.write_int(i32::from(thread.icon))?;
    packet.write_int(thread.replies.len() as i32)?;
    for reply in &thread.replies {
        packet.write_int(reply.id)?;
        packet.write_int(reply.poster)?;
        packet.write_long(filetime(reply.posted_at))?;
        packet.write_str_with_length(&reply.text)?;
    }
    Ok(packet)
}

/// The thread asked for was deleted (mode 8): "This message had been
/// deleted by another user."
pub fn build_bbs_missing() -> Result<Packet, NetworkError> {
    bbs(MISSING)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use db::guild_bbs::Reply;
    use std::time::{Duration, UNIX_EPOCH};

    fn at(secs: u64) -> std::time::SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn summary(id: i32) -> Summary {
        Summary {
            id,
            poster: 5,
            title: format!("t{id}"),
            posted_at: at(1),
            icon: 2,
            replies: 3,
        }
    }

    fn expect_summary(expect: &mut Expect, id: i32) {
        expect
            .int(id)
            .int(5)
            .string(&format!("t{id}"))
            .long(filetime(at(1)))
            .int(2)
            .int(3);
    }

    #[test]
    fn an_empty_board_has_no_page() {
        let packet = build_bbs_list(&Page::default()).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildBbs as i16)
            .byte(6)
            .byte(0)
            .int(0)
            .end();
    }

    #[test]
    fn the_list_puts_the_notice_first() {
        let page = Page {
            notice: Some(summary(0)),
            total: 12,
            threads: vec![summary(2), summary(1)],
        };
        let packet = build_bbs_list(&page).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GuildBbs as i16).byte(6).byte(1);
        expect_summary(&mut expect, 0);
        expect.int(12).int(2);
        expect_summary(&mut expect, 2);
        expect_summary(&mut expect, 1);
        expect.end();
    }

    #[test]
    fn a_thread_lists_its_replies() {
        let thread = Thread {
            id: 4,
            poster: 5,
            title: "Hi".into(),
            body: "Body".into(),
            posted_at: at(1),
            icon: 101,
            replies: vec![Reply {
                id: 70,
                poster: 6,
                text: "yo".into(),
                posted_at: at(2),
            }],
        };
        let packet = build_bbs_thread(&thread).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::GuildBbs as i16)
            .byte(7)
            .int(4)
            .int(5)
            .long(filetime(at(1)))
            .string("Hi")
            .string("Body")
            .int(101)
            .int(1)
            .int(70)
            .int(6)
            .long(filetime(at(2)))
            .string("yo")
            .end();

        let packet = build_bbs_missing().unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GuildBbs as i16).byte(8).end();
    }
}
