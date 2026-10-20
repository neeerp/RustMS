//! The Game Option: a v83 client turns requests away for its player, and the
//! server tells whoever asked. A whisper turned away says the receiver
//! blocks whispers, a party invitation that they block invitations, and a
//! messenger invitation that they aren't accepting chat.

use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::packets::build_whisper;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;

/// Read until a packet with `op` and `mode`, and return what follows them.
async fn next(c: &mut MapleTestConnection, t: &mut Trace, op: SendOpcode, mode: u8) -> Vec<u8> {
    let op = op as i16;
    loop {
        let p = read(c, t).await;
        if p.opcode() == op && p.bytes[2] == mode {
            return p.bytes[3..].to_vec();
        }
    }
}

/// A name and the byte after it.
fn name_and_flag(body: &[u8]) -> (String, u8) {
    let mut r = Cursor::new(body);
    let name = r.read_str_with_length().unwrap();
    (name, r.read_byte().unwrap())
}

fn request(op: RecvOpcode, mode: u8, names: &[&str]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(op as i16).unwrap();
    p.write_byte(mode).unwrap();
    for name in names {
        p.write_str_with_length(name).unwrap();
    }
    p
}

#[tokio::test]
async fn requests_the_options_block_tell_whoever_asked() {
    let config = load_harness_config_or_fail().await;
    let mut asker = login_to_world_session(&config).await.unwrap();
    let blocker_config = load_harness_config_or_fail().await;
    let mut blocker = login_to_world_session(&blocker_config).await.unwrap();
    let (a, b) = (asker.character_name.clone(), blocker.character_name.clone());
    let (c, mut ct) = (&mut asker.connection, Trace::default());
    let (d, mut dt) = (&mut blocker.connection, Trace::default());

    // A whisper the blocker's client turns away (FUN_0053228e).
    send(c, build_whisper(&b, "hello").unwrap()).await;
    next(d, &mut dt, SendOpcode::Whisper, 0x12).await;
    send(d, request(RecvOpcode::Whisper, 0x22, &[&a])).await;
    let body = next(c, &mut ct, SendOpcode::Whisper, 0x22).await;
    assert_eq!(name_and_flag(&body), (b.clone(), 0));

    // A party invitation, turned down with reason 0x15 (FUN_00a3e31c).
    send(c, request(RecvOpcode::PartyOperation, 4, &[&b])).await;
    next(d, &mut dt, SendOpcode::PartyOperation, 4).await;
    let mut deny = Packet::new_empty();
    deny.write_short(RecvOpcode::DenyPartyRequest as i16)
        .unwrap();
    deny.write_byte(0x15).unwrap();
    deny.write_str_with_length(&a).unwrap();
    deny.write_str_with_length(&b).unwrap();
    send(d, deny).await;
    let body = next(c, &mut ct, SendOpcode::PartyOperation, 0x15).await;
    assert_eq!(Cursor::new(&body).read_str_with_length().unwrap(), b);

    // A messenger invitation, declined with the blocking flag (FUN_00851478).
    let mut open = request(RecvOpcode::Messenger, 0, &[]);
    open.write_int(0).unwrap();
    send(c, open).await;
    send(c, request(RecvOpcode::Messenger, 3, &[&b])).await;
    next(d, &mut dt, SendOpcode::Messenger, 3).await;
    let mut decline = request(RecvOpcode::Messenger, 5, &[&a, &b]);
    decline.write_byte(1).unwrap();
    send(d, decline).await;
    let body = next(c, &mut ct, SendOpcode::Messenger, 5).await;
    assert_eq!(name_and_flag(&body), (b, 1));
}
