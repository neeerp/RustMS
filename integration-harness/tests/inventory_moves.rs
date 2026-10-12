//! Equipping, unequipping, and moving items, as the client asks with ITEM_MOVE.

use integration_harness::connection::MapleTestConnection;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{read, send, QuestTrace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const WEAPON: i16 = -11;

fn item_move(kind: i8, from: i16, to: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(kind as u8).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(1).unwrap();
    p
}

#[derive(Debug, PartialEq)]
enum Reply {
    /// INVENTORY_OPERATION moves: (type, from, to).
    Moves(Vec<(u8, i16, i16)>),
    Refused,
}

/// Send a move and read the server's answer to it.
async fn request(c: &mut MapleTestConnection, kind: i8, from: i16, to: i16) -> Reply {
    send(c, item_move(kind, from, to)).await;
    let mut trace = QuestTrace::default();
    loop {
        let p = read(c, &mut trace).await;
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        if p.opcode() == SendOpcode::InventoryOperation as i16 {
            r.read_byte().unwrap();
            let count = r.read_byte().unwrap();
            let mut moves = Vec::new();
            for _ in 0..count {
                assert_eq!(r.read_byte().unwrap(), 2, "only moves expected");
                moves.push((
                    r.read_byte().unwrap(),
                    r.read_short().unwrap(),
                    r.read_short().unwrap(),
                ));
            }
            return Reply::Moves(moves);
        }
        if p.opcode() == SendOpcode::StatChange as i16 && p.bytes.len() == 7 {
            return Reply::Refused;
        }
    }
}

/// The next UPDATE_CHAR_LOOK an observer sees: whose, and their weapon.
async fn look_update(c: &mut MapleTestConnection) -> i32 {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("look update"))
            .await
            .expect("no look update")
            .unwrap()
            .packet;
        if p.opcode() == SendOpcode::UpdateCharLook as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            return r.read_int().unwrap();
        }
    }
}

#[tokio::test]
async fn equipment_moves_persist_and_update_the_look_for_others() {
    let config = load_harness_config_or_fail().await;
    let observer_config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut observer = login_to_world_session(&observer_config).await.unwrap();
    assert_eq!(session.map_id, observer.map_id);
    let weapon = session.equipped[&WEAPON.abs()];
    let c = &mut session.connection;

    // Take the weapon off (HeavenClient sends the EQUIPPED type, -1).
    assert_eq!(
        request(c, -1, WEAPON, 1).await,
        Reply::Moves(vec![(1, WEAPON, 1)])
    );
    assert_eq!(
        look_update(&mut observer.connection).await,
        session.character_id
    );
    // Move it within the equip bag, then wear it again.
    assert_eq!(request(c, 1, 1, 4).await, Reply::Moves(vec![(1, 1, 4)]));
    assert_eq!(
        request(c, 1, 4, WEAPON).await,
        Reply::Moves(vec![(1, 4, WEAPON)])
    );
    assert_eq!(
        look_update(&mut observer.connection).await,
        session.character_id
    );

    // A weapon is not a hat, empty slots hold nothing, and drops need a floor.
    assert_eq!(
        request(c, -1, WEAPON, 2).await,
        Reply::Moves(vec![(1, WEAPON, 2)])
    );
    assert_eq!(request(c, 1, 2, -1).await, Reply::Refused);
    assert_eq!(request(c, 1, 9, 10).await, Reply::Refused);
    assert_eq!(request(c, 1, 2, 0).await, Reply::Refused);
    drop(session);
    drop(observer);

    let restored = relogin(&config).await;
    assert!(
        !restored.equipped.contains_key(&WEAPON.abs()),
        "the weapon stays in the bag"
    );
    let mut restored = restored;
    assert_eq!(
        request(&mut restored.connection, 1, 2, WEAPON).await,
        Reply::Moves(vec![(1, 2, WEAPON)])
    );
    drop(restored);
    let worn = relogin(&config).await;
    assert_eq!(worn.equipped[&WEAPON.abs()], weapon);
}
