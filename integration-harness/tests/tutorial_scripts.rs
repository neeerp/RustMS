//! Map entry banners and touch portals.

use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, warp, Trace};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;

#[derive(Debug, PartialEq)]
enum Seen {
    Banner(String),
    Image(String),
    Hint(String, i16, i16),
}

fn touch(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn decode(p: &Packet) -> Option<Seen> {
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    if p.opcode() == SendOpcode::FieldEffect as i16 {
        assert_eq!(r.read_byte().unwrap(), 3);
        Some(Seen::Banner(r.read_str_with_length().unwrap()))
    } else if p.opcode() == SendOpcode::ShowItemGainInChat as i16 {
        assert_eq!(r.read_byte().unwrap(), 0x17);
        Some(Seen::Image(r.read_str_with_length().unwrap()))
    } else if p.opcode() == SendOpcode::PlayerHint as i16 {
        let text = r.read_str_with_length().unwrap();
        Some(Seen::Hint(
            text,
            r.read_short().unwrap(),
            r.read_short().unwrap(),
        ))
    } else {
        None
    }
}

/// Touch a portal and collect what it showed, up to the enable-actions
/// stat update that ends every script.
async fn touch_portal(c: &mut MapleTestConnection, portal: &str) -> Vec<Seen> {
    send(c, touch(portal)).await;
    let mut trace = Trace::default();
    let mut seen = Vec::new();
    loop {
        let p = read(c, &mut trace).await;
        if let Some(s) = decode(&p) {
            seen.push(s);
        }
        if p.opcode() == SendOpcode::StatChange as i16 && p.bytes.len() == 2 + 1 + 4 {
            return seen;
        }
    }
}

async fn banner(c: &mut MapleTestConnection) -> String {
    let mut trace = Trace::default();
    loop {
        if let Some(Seen::Banner(path)) = decode(&read(c, &mut trace).await) {
            return path;
        }
    }
}

#[tokio::test]
async fn banners_show_and_unscripted_portals_do_nothing() {
    let config = load_harness_config_or_fail().await;
    let mut trace = Trace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    assert_eq!(session.map_id, 0);

    // The original island's advice portals name scripts the server doesn't
    // have; touching them, or portals without scripts, does nothing.
    assert_eq!(touch_portal(c, "tutorial0").await, []);
    assert_eq!(touch_portal(c, "out00").await, []);
    assert_eq!(touch_portal(c, "no-such-portal").await, []);

    // Maps with a go<map id> entry script show their name banner.
    assert_eq!(warp(c, 1_010_100, "", &mut trace).await, 1_010_100);
    assert_eq!(banner(c).await, "maplemap/enter/1010100");
}
