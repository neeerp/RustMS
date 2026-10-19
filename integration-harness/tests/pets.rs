//! Pets: a Baby Tiger bought in the Cash Shop comes out of the Cash
//! Inventory named after its kind. Summoned, it follows its owner and the
//! map sees it come, move, obey, speak and go back; it eats food and cash
//! food and is renamed; it is out again after a relog, and a player
//! arriving later sees it beside its owner.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_nx_credit, set_pet_fullness};
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
const USE: u8 = 2;
const PET_FOOD: i32 = 2_120_000;
/// Cash food for the Baby Tiger alone, and food for monkeys.
const GOLDEN_DRUMSTICK: i32 = 5_240_027;
const MONKEY_BANANA: i32 = 5_240_000;
const NAME_TAG: i32 = 5_170_000;

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

/// PET_FOOD: feed the food in USE `slot`.
fn pet_food(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PetFood as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_int(PET_FOOD).unwrap();
    p
}

/// USE_CASH_ITEM: the cash food `item_id` in Cash `slot`.
fn cash_food(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_int(0).unwrap(); // time
    p
}

/// USE_CASH_ITEM: the name tag in Cash `slot` names the first pet out.
fn name_tag(slot: i16, name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(NAME_TAG).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_int(0).unwrap(); // time
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
    assert_eq!(bought[..2], [0x89, 3], "a package's three items");
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
    // Leaving the Cash Shop moves channels; a relog does as well. Offline,
    // the owner gets food and the tiger grows hungry.
    drop(shopper);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(owner_id, USE.into(), 1, PET_FOOD, 2)
        .await
        .expect("grant pet food");
    for (slot, item) in [(2, GOLDEN_DRUMSTICK), (3, MONKEY_BANANA), (4, NAME_TAG)] {
        grant_stack(owner_id, CASH.into(), slot, item, 2)
            .await
            .expect("grant cash pet food and a name tag");
    }
    set_pet_fullness(cash_id, 60).await.expect("a hungry tiger");
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
        assert_eq!(r.read_byte().unwrap(), 1, "called by name");
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

    // What its owner's client showed it say, the others see, in the pet's
    // own balloon since the command called it by name.
    send(o, pet_chat(cash_id, 12, "Ahh")).await;
    let said = next(w, &mut wt, SendOpcode::PetChat).await;
    let mut r = Cursor::new(&said[..]);
    assert_eq!(r.read_int().unwrap(), owner_id);
    assert_eq!(r.read_bytes(3).unwrap(), [0, 0, 12], "pet 0, kind, action");
    assert_eq!(r.read_str_with_length().unwrap(), "Ahh");
    assert_eq!(r.read_byte().unwrap(), 1, "its own balloon");

    // Fed, the hungry tiger eats and grows closer: one food goes, and the
    // tiger's item comes back fuller, unlocking the owner's client.
    send(o, pet_food(1)).await;
    let answer = next(o, &mut ot, SendOpcode::PetCommand).await;
    assert_eq!(answer[..4], owner_id.to_le_bytes());
    assert_eq!(answer[4..], [0, 1, 1, 0], "pet 0 enjoyed its food");
    let update = next(o, &mut ot, SendOpcode::InventoryOperation).await;
    let mut r = Cursor::new(&update[..]);
    assert_eq!(r.read_bytes(2).unwrap(), [1, 3], "a reply; three changes");
    assert_eq!(r.read_bytes(2).unwrap(), [1, USE], "one food fewer");
    assert_eq!((r.read_short().unwrap(), r.read_short().unwrap()), (1, 1));
    assert_eq!(r.read_bytes(2).unwrap(), [3, CASH]);
    assert_eq!(r.read_short().unwrap(), 1);
    assert_eq!(r.read_bytes(2).unwrap(), [0, CASH]);
    assert_eq!(r.read_short().unwrap(), 1);
    r.read_bytes(1 + 4 + 1 + 8 + 8 + 13).unwrap();
    assert_eq!(r.read_byte().unwrap(), 3, "still level 3");
    assert_eq!(r.read_short().unwrap(), 4, "closeness 4");
    assert_eq!(r.read_byte().unwrap(), 90, "fullness 90");

    // Its own cash food fills it up and brings it 100 closer: 76 names the
    // pet that ate, then the answer and the update as for PET_FOOD.
    send(o, cash_food(2, GOLDEN_DRUMSTICK)).await;
    let ate = next(o, &mut ot, SendOpcode::CashPetFoodResult).await;
    assert_eq!(ate, [0, 0], "pet 0 ate");
    let answer = next(o, &mut ot, SendOpcode::PetCommand).await;
    assert_eq!(answer[4..], [0, 1, 1, 0], "and enjoyed it");
    let update = next(o, &mut ot, SendOpcode::InventoryOperation).await;
    let mut r = Cursor::new(&update[..]);
    assert_eq!(r.read_bytes(2).unwrap(), [1, 3], "a reply; three changes");
    assert_eq!(r.read_bytes(2).unwrap(), [1, CASH], "one drumstick fewer");
    assert_eq!((r.read_short().unwrap(), r.read_short().unwrap()), (2, 1));
    r.read_bytes(2 + 2 + 2 + 2 + 1 + 4 + 1 + 8 + 8 + 13)
        .unwrap();
    assert_eq!(r.read_byte().unwrap(), 7, "level 7");
    assert_eq!(r.read_short().unwrap(), 104, "closeness 104");
    assert_eq!(r.read_byte().unwrap(), 100, "full");
    let effect = next(o, &mut ot, SendOpcode::ShowItemGainInChat).await;
    assert_eq!(effect, [4, 0, 0], "its level up");
    // A food for other pets is refused, and the client unlocked.
    send(o, cash_food(3, MONKEY_BANANA)).await;
    let ate = next(o, &mut ot, SendOpcode::CashPetFoodResult).await;
    assert_eq!(ate, [1], "no pet out eats it");
    next(o, &mut ot, SendOpcode::StatChange).await;

    // A name tag renames it: one update takes the tag and re-sends the pet,
    // then the owner and the watcher learn its name.
    send(o, name_tag(4, "Tigger")).await;
    let update = next(o, &mut ot, SendOpcode::InventoryOperation).await;
    assert_eq!(update[..2], [1, 3], "a reply; three changes");
    let mut renamed = owner_id.to_le_bytes().to_vec();
    renamed.extend([0, 6, 0]); // pet 0, then the name's length
    renamed.extend(b"Tigger");
    renamed.push(0); // no name tag
    assert_eq!(next(o, &mut ot, SendOpcode::PetNameChange).await, renamed);
    assert_eq!(next(w, &mut wt, SendOpcode::PetNameChange).await, renamed);
    let summoned = (owner_id, 0, BABY_TIGER, "Tigger".to_string(), cash_id);

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
    assert_eq!(seen.pets, [(BABY_TIGER, "Tigger".to_string(), cash_id)]);
}
