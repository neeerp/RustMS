//! Equipping, unequipping, and moving items, as the client asks with ITEM_MOVE.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{set_equip_str, set_job, set_str, stow_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
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
    let mut trace = Trace::default();
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

/// Wait for the observer to see `of`'s look updated, skipping the other
/// tests' players that share the map in a full suite.
async fn look_update(c: &mut MapleTestConnection, of: i32) {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("look update"))
            .await
            .expect("no look update")
            .unwrap()
            .packet;
        if p.opcode() == SendOpcode::UpdateCharLook as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            if r.read_int().unwrap() == of {
                return;
            }
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
    look_update(&mut observer.connection, session.character_id).await;
    // Move it within the equip bag, then wear it again.
    assert_eq!(request(c, 1, 1, 4).await, Reply::Moves(vec![(1, 1, 4)]));
    assert_eq!(
        request(c, 1, 4, WEAPON).await,
        Reply::Moves(vec![(1, 4, WEAPON)])
    );
    look_update(&mut observer.connection, session.character_id).await;

    // A weapon is not a hat, and empty slots hold nothing.
    assert_eq!(
        request(c, -1, WEAPON, 2).await,
        Reply::Moves(vec![(1, WEAPON, 2)])
    );
    assert_eq!(request(c, 1, 2, -1).await, Reply::Refused);
    assert_eq!(request(c, 1, 9, 10).await, Reply::Refused);
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

const CUTLASS: i32 = 1302004;
const WORK_GLOVES: i32 = 1082002;
const GLOVES: i16 = -8;

#[tokio::test]
async fn other_equipment_s_stats_count_toward_an_equip_s_requirements() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // The Cutlass asks a level 35 warrior for 110 STR; the gloves give 2.
    set_job(id, 100, 35, 50).await.unwrap();
    set_str(id, 108).await.unwrap();
    stow_equip(id, 5, CUTLASS, 7).await.unwrap();
    stow_equip(id, 6, WORK_GLOVES, 5).await.unwrap();
    set_equip_str(id, 6, 2).await.unwrap();

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(request(c, 1, 5, WEAPON).await, Reply::Refused);
    assert_eq!(
        request(c, 1, 6, GLOVES).await,
        Reply::Moves(vec![(1, 6, GLOVES)])
    );
    assert_eq!(
        request(c, 1, 5, WEAPON).await,
        Reply::Moves(vec![(1, 5, WEAPON)])
    );
}

const SHOES: i16 = -7;

/// Send GATHER_ITEMS or SORT_ITEMS for the equip bag; returns the moves and
/// the result packet's opcode.
async fn rearrange(c: &mut MapleTestConnection, opcode: RecvOpcode) -> (Vec<(i16, i16)>, i16) {
    let mut p = Packet::new_empty();
    p.write_short(opcode as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(1).unwrap();
    send(c, p).await;
    let mut trace = Trace::default();
    let mut moves = Vec::new();
    loop {
        let p = read(c, &mut trace).await;
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        if p.opcode() == SendOpcode::InventoryOperation as i16 {
            r.read_byte().unwrap();
            for _ in 0..r.read_byte().unwrap() {
                assert_eq!(r.read_byte().unwrap(), 2, "only moves expected");
                assert_eq!(r.read_byte().unwrap(), 1, "equip bag");
                moves.push((r.read_short().unwrap(), r.read_short().unwrap()));
            }
        }
        if p.opcode() == SendOpcode::GatherItemResult as i16
            || p.opcode() == SendOpcode::SortItemResult as i16
        {
            assert_eq!(&p.bytes[2..], &[0, 1]);
            return (moves, p.opcode());
        }
    }
}

#[tokio::test]
async fn gather_and_sort_rearrange_the_bag_and_persist() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;

    // The weapon (130xxxx) ahead of the shoes (107xxxx), with gaps.
    assert_eq!(
        request(c, -1, WEAPON, 3).await,
        Reply::Moves(vec![(1, WEAPON, 3)])
    );
    assert_eq!(
        request(c, -1, SHOES, 6).await,
        Reply::Moves(vec![(1, SHOES, 6)])
    );

    assert_eq!(
        rearrange(c, RecvOpcode::GatherItems).await,
        (vec![(3, 1), (6, 2)], SendOpcode::GatherItemResult as i16)
    );
    assert_eq!(
        rearrange(c, RecvOpcode::SortItems).await,
        (vec![(2, 1)], SendOpcode::SortItemResult as i16)
    );
    drop(session);

    // The shoes are now first and the weapon second.
    let mut restored = relogin(&config).await;
    let c = &mut restored.connection;
    assert_eq!(
        request(c, 1, 2, WEAPON).await,
        Reply::Moves(vec![(1, 2, WEAPON)])
    );
    assert_eq!(
        request(c, 1, 1, SHOES).await,
        Reply::Moves(vec![(1, 1, SHOES)])
    );
}
