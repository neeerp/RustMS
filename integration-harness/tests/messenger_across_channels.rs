use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

/// A messenger packet the server sent, decoded by mode.
#[derive(Debug, PartialEq, Eq)]
enum Messenger {
    Add { slot: u8, name: String, channel: u8 },
    Join { slot: u8 },
    Remove { slot: u8 },
    Invite { from: String, messenger_id: i32 },
    InviteResult { target: String, sent: bool },
    Chat { line: String },
}

fn decode(packet: &Packet) -> Messenger {
    let mut r = Cursor::new(&packet.bytes);
    r.read_short().unwrap();
    match r.read_byte().unwrap() {
        0x00 => {
            let slot = r.read_byte().unwrap();
            // The avatar look: gender, skin, face, megaphone flag, hair, two
            // 0xFF-terminated equipment lists, cash weapon, three pets.
            r.read_byte().unwrap();
            r.read_byte().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            r.read_int().unwrap();
            for _ in 0..2 {
                while r.read_byte().unwrap() != 0xFF {
                    r.read_int().unwrap();
                }
            }
            for _ in 0..4 {
                r.read_int().unwrap();
            }
            let name = r.read_str_with_length().unwrap();
            let channel = r.read_byte().unwrap();
            Messenger::Add {
                slot,
                name,
                channel,
            }
        }
        0x01 => Messenger::Join {
            slot: r.read_byte().unwrap(),
        },
        0x02 => Messenger::Remove {
            slot: r.read_byte().unwrap(),
        },
        0x03 => {
            let from = r.read_str_with_length().unwrap();
            r.read_byte().unwrap();
            Messenger::Invite {
                from,
                messenger_id: r.read_int().unwrap(),
            }
        }
        0x04 => Messenger::InviteResult {
            target: r.read_str_with_length().unwrap(),
            sent: r.read_byte().unwrap() == 1,
        },
        0x06 => Messenger::Chat {
            line: r.read_str_with_length().unwrap(),
        },
        mode => panic!("unexpected messenger mode {mode}"),
    }
}

/// The next messenger packet, skipping field traffic.
async fn next_messenger(connection: &mut MapleTestConnection, phase: &'static str) -> Messenger {
    for _ in 0..20 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .unwrap_or_else(|_| panic!("timed out: {phase}"))
            .unwrap_or_else(|e| panic!("{phase}: {e}"));
        if envelope.opcode() == SendOpcode::Messenger as i16 {
            return decode(&envelope.packet);
        }
    }
    panic!("no messenger packet: {phase}");
}

fn request(mode: u8, body: impl FnOnce(&mut Packet)) -> Packet {
    let mut packet = Packet::new_empty();
    packet.write_short(RecvOpcode::Messenger as i16).unwrap();
    packet.write_byte(mode).unwrap();
    body(&mut packet);
    packet
}

fn open(messenger_id: i32) -> Packet {
    request(0x00, |p| {
        p.write_int(messenger_id).unwrap();
    })
}

fn with_text(mode: u8, text: &str) -> Packet {
    request(mode, |p| {
        p.write_str_with_length(text).unwrap();
    })
}

#[tokio::test]
async fn messenger_across_channels() {
    let host = load_harness_config_or_fail().await.with_channel(0, 0);
    let guest = load_harness_config_or_fail().await.with_channel(0, 1);
    let mut host = login_to_world_session(&host)
        .await
        .expect("host failed to login");
    let mut guest = login_to_world_session(&guest)
        .await
        .expect("guest failed to login");

    host.connection
        .send_packet(open(0), "open messenger")
        .await
        .unwrap();
    assert_eq!(
        next_messenger(&mut host.connection, "host seated").await,
        Messenger::Join { slot: 0 }
    );

    // Joining without an invitation does nothing; the next packet the
    // guest sees is the invitation itself.
    guest
        .connection
        .send_packet(open(999_999), "uninvited join")
        .await
        .unwrap();
    host.connection
        .send_packet(with_text(0x03, &guest.character_name), "invite")
        .await
        .unwrap();
    let Messenger::Invite { from, messenger_id } =
        next_messenger(&mut guest.connection, "invitation").await
    else {
        panic!("expected an invitation");
    };
    assert_eq!(from, host.character_name);
    assert_eq!(
        next_messenger(&mut host.connection, "invite result").await,
        Messenger::InviteResult {
            target: guest.character_name.clone(),
            sent: true
        }
    );

    guest
        .connection
        .send_packet(open(messenger_id), "accept invitation")
        .await
        .unwrap();
    assert_eq!(
        next_messenger(&mut guest.connection, "guest seated").await,
        Messenger::Join { slot: 1 }
    );
    assert_eq!(
        next_messenger(&mut guest.connection, "guest sees host").await,
        Messenger::Add {
            slot: 0,
            name: host.character_name.clone(),
            channel: 0
        }
    );
    assert_eq!(
        next_messenger(&mut host.connection, "host sees guest").await,
        Messenger::Add {
            slot: 1,
            name: guest.character_name.clone(),
            channel: 1
        }
    );

    // Lines must speak as their sender; a forged one is dropped.
    let forged = format!("{} : forged", guest.character_name);
    let line = format!("{} : hello from channel 1", host.character_name);
    host.connection
        .send_packet(with_text(0x06, &forged), "forged chat")
        .await
        .unwrap();
    host.connection
        .send_packet(with_text(0x06, &line), "chat")
        .await
        .unwrap();
    assert_eq!(
        next_messenger(&mut guest.connection, "chat").await,
        Messenger::Chat { line }
    );

    guest
        .connection
        .send_packet(request(0x02, |_| {}), "leave")
        .await
        .unwrap();
    assert_eq!(
        next_messenger(&mut host.connection, "guest left").await,
        Messenger::Remove { slot: 1 }
    );

    host.connection
        .send_packet(with_text(0x03, "NobodyHere"), "invite offline")
        .await
        .unwrap();
    assert_eq!(
        next_messenger(&mut host.connection, "offline invite result").await,
        Messenger::InviteResult {
            target: "NobodyHere".into(),
            sent: false
        }
    );
}
