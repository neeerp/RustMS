//! Pets: a Baby Tiger bought in the Cash Shop comes out of the Cash
//! Inventory named after its kind. Summoned, it follows its owner and the
//! map sees it come, move, obey, speak and go back; it is out again after a
//! relog, and a player arriving later sees it beside its owner.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_nx_credit;
use integration_harness::packets::{
    build_cash_shop_buy_package, build_cash_shop_take_out, build_enter_cash_shop,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;

/// The Baby Tiger Package (16,900 NX): the pet, its equipment and food.
const PACKAGE: i32 = 10002307;
const BABY_TIGER: i32 = 5_000_066;
/// Ellinia, which no other test visits.
const ELLINIA: i32 = 101_000_000;
const CASH: u8 = 5;

/// Read until a packet with `op`, and return its body.
async fn next(c: &mut MapleTestConnection, t: &mut Trace, op: SendOpcode) -> Vec<u8> {
    let op = op as i16;
    loop {
        let p = read(c, t).await;
        if p.opcode() == op {
            return p.bytes[2..].to_vec();
        }
    }
}

/// SPAWN_PET: the pet in Cash slot `slot` comes out or goes back.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// MOVE_PET: the pet `serial` walks from (x, y) to (x + 50, y).
fn move_pet(serial: i64, x: i16, y: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MovePet as i16).unwrap();
    p.write_long(serial).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_bytes(&[1, 0]).unwrap();
    p.write_short(x + 50).unwrap();
    p.write_short(y).unwrap();
    p.write_bytes(&[0; 6]).unwrap();
    p.write_byte(4).unwrap();
    p.write_short(100).unwrap();
    p
}

/// PET_COMMAND: the words the owner said matched `command` of pet `serial`.
fn pet_command(serial: i64, command: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PetCommand as i16).unwrap();
    p.write_long(serial).unwrap();
    p.write_byte(1).unwrap(); // began with its name
    p.write_byte(command).unwrap();
    p
}

/// PET_CHAT: the owner's client showed pet `serial` saying `text`.
fn pet_chat(serial: i64, action: u8, text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PetChat as i16).unwrap();
    p.write_long(serial).unwrap();
    p.write_byte(0).unwrap(); // a command's answer
    p.write_byte(action).unwrap();
    p.write_str_with_length(text).unwrap();
    p
}

/// "Cute" at levels 1 to 9 (`interact/28`): obeyed 85% of the time, for 3
/// closeness.
const CUTE: u8 = 28;

#[tokio::test]
async fn a_pet_comes_out_follows_its_owner_and_stays_out() {
    let config = load_harness_config_or_fail().await;
    let mut shopper = login_to_world_session(&config)
        .await
        .expect("owner login failed");
    let owner_id = shopper.character_id;
    set_nx_credit(&config.username, 20_000)
        .await
        .expect("grant NX");
    let (o, mut ot) = (&mut shopper.connection, Trace::default());

    // Buy the package; the pet is its first item.
    send(o, build_enter_cash_shop()).await;
    next(o, &mut ot, SendOpcode::QueryCashResult).await;
    send(o, build_cash_shop_buy_package(1, PACKAGE).unwrap()).await;
    let bought = next(o, &mut ot, SendOpcode::CashShopOperation).await;
    assert_eq!(bought[..2], [0xA4, 3], "a package's three items");
    let mut r = Cursor::new(&bought[2..]);
    let cash_id = r.read_long().unwrap();
    r.read_bytes(8).unwrap();
    assert_eq!(r.read_int().unwrap(), BABY_TIGER);

    // Taken out, it is a pet named after its kind.
    send(o, build_cash_shop_take_out(cash_id, CASH, 1).unwrap()).await;
    let taken = next(o, &mut ot, SendOpcode::CashShopOperation).await;
    let mut r = Cursor::new(&taken[..]);
    assert_eq!(r.read_byte().unwrap(), 0x68, "taken out");
    assert_eq!(r.read_short().unwrap(), 1, "into the first Cash slot");
    assert_eq!(r.read_byte().unwrap(), 3, "a pet item");
    assert_eq!(r.read_int().unwrap(), BABY_TIGER);
    assert_eq!(r.read_byte().unwrap(), 1, "a cash item");
    assert_eq!(r.read_long().unwrap(), cash_id);
    r.read_long().unwrap(); // expiration
    let name = r.read_bytes(13).unwrap();
    assert_eq!(&name[..11], b"Baby Tiger\0");
    assert_eq!(r.read_byte().unwrap(), 1, "level 1");
    assert_eq!(r.read_short().unwrap(), 0, "no closeness yet");
    assert_eq!(r.read_byte().unwrap(), 100, "full");
    // Leaving the Cash Shop moves channels; a relog does as well.
    drop(shopper);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut owner = relogin(&config).await;
    let (o, mut ot) = (&mut owner.connection, Trace::default());

    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (w, mut wt) = (&mut watcher.connection, Trace::default());
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(warp(o, ELLINIA, "", &mut ot).await, ELLINIA);
    until(w, &mut wt, |t| {
        t.players.iter().any(|p| p.character_id == owner_id)
    })
    .await;

    // Summoned, it comes out for both, and its owner's stats list it.
    send(o, spawn_pet(1)).await;
    until(o, &mut ot, |t| !t.pets.is_empty() && t.stat(0x8).is_some()).await;
    let summoned = (owner_id, 0, BABY_TIGER, "Baby Tiger".to_string(), cash_id);
    assert_eq!(ot.pets, [summoned.clone()]);
    assert_eq!(ot.stat(0x8), Some(cash_id));
    assert_eq!(ot.stat(stat::PET_2), Some(0));
    until(w, &mut wt, |t| !t.pets.is_empty()).await;
    assert_eq!(wt.pets, [summoned.clone()]);

    // Where it walks, the others see it walk.
    send(o, move_pet(cash_id, 100, 200)).await;
    until(w, &mut wt, |t| !t.pet_moves.is_empty()).await;
    assert_eq!(wt.pet_moves, [(owner_id, 0, 100, 200)]);
    // A pet that isn't out goes nowhere.
    send(o, move_pet(cash_id + 1, 0, 0)).await;
    read_for(w, &mut wt, 300).await;
    assert_eq!(wt.pet_moves.len(), 1);

    // Told the same thing until it obeys, it grows closer, which levels it
    // up: only the owner hears the answer, and everyone sees the level up.
    let mut tries = 0;
    loop {
        tries += 1;
        assert!(tries <= 20, "the tiger never obeyed");
        send(o, pet_command(cash_id, CUTE)).await;
        let answer = next(o, &mut ot, SendOpcode::PetCommand).await;
        let mut r = Cursor::new(&answer[..]);
        assert_eq!(r.read_int().unwrap(), owner_id);
        assert_eq!(r.read_bytes(3).unwrap(), [0, 0, CUTE], "pet 0, command");
        let obeyed = r.read_byte().unwrap() == 1;
        assert_eq!(r.read_byte().unwrap(), 0);
        if !obeyed {
            continue;
        }
        let update = next(o, &mut ot, SendOpcode::InventoryOperation).await;
        let mut r = Cursor::new(&update[..]);
        assert_eq!(r.read_bytes(2).unwrap(), [0, 2], "not a reply; two changes");
        assert_eq!(r.read_bytes(2).unwrap(), [3, CASH], "the pet's item goes");
        assert_eq!(r.read_short().unwrap(), 1);
        assert_eq!(r.read_bytes(2).unwrap(), [0, CASH], "and comes back");
        assert_eq!(r.read_short().unwrap(), 1);
        assert_eq!(r.read_byte().unwrap(), 3);
        assert_eq!(r.read_int().unwrap(), BABY_TIGER);
        r.read_bytes(1 + 8 + 8 + 13).unwrap();
        assert_eq!(r.read_byte().unwrap(), 3, "level 3");
        assert_eq!(r.read_short().unwrap(), 3, "closeness 3");
        let effect = next(o, &mut ot, SendOpcode::ShowItemGainInChat).await;
        assert_eq!(effect, [4, 0, 0], "its level up, on pet 0");
        break;
    }
    let mut seen = next(w, &mut wt, SendOpcode::ShowForeignEffect).await;
    while seen[4] != 4 {
        seen = next(w, &mut wt, SendOpcode::ShowForeignEffect).await;
    }
    assert_eq!(seen[..4], owner_id.to_le_bytes());
    assert_eq!(seen[4..], [4, 0, 0]);

    // What its owner's client showed it say, the others see.
    send(o, pet_chat(cash_id, 12, "Ahh")).await;
    let said = next(w, &mut wt, SendOpcode::PetChat).await;
    let mut r = Cursor::new(&said[..]);
    assert_eq!(r.read_int().unwrap(), owner_id);
    assert_eq!(r.read_bytes(3).unwrap(), [0, 0, 12], "pet 0, kind, action");
    assert_eq!(r.read_str_with_length().unwrap(), "Ahh");
    assert_eq!(r.read_byte().unwrap(), 0);

    // Summoned again, it goes back.
    send(o, spawn_pet(1)).await;
    until(o, &mut ot, |t| !t.pets_gone.is_empty()).await;
    assert_eq!(ot.pets_gone, [(owner_id, 0, 0)]);
    until(o, &mut ot, |t| t.stat(0x8) == Some(0)).await;
    until(w, &mut wt, |t| !t.pets_gone.is_empty()).await;
    assert_eq!(wt.pets_gone, [(owner_id, 0, 0)]);

    // Out when its owner leaves, it is out when they return, and the
    // watcher sees it come with them.
    send(o, spawn_pet(1)).await;
    until(o, &mut ot, |t| t.pets.len() == 2).await;
    drop(owner);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut owner = relogin(&config).await;
    let (o, mut ot) = (&mut owner.connection, Trace::default());
    until(o, &mut ot, |t| !t.pets.is_empty()).await;
    assert_eq!(ot.pets, [summoned.clone()]);
    wt.players.clear();
    until(w, &mut wt, |t| {
        t.players.iter().any(|p| p.character_id == owner_id)
    })
    .await;
    let seen = wt
        .players
        .iter()
        .find(|p| p.character_id == owner_id)
        .unwrap();
    assert_eq!(seen.pets, [(BABY_TIGER, "Baby Tiger".to_string(), cash_id)]);
}
