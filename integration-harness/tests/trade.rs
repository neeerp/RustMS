//! Trading (PLAYER_INTERACTION, trading room): an invitation can be turned
//! down; once both press Trade the items and mesos swap, less the fee on
//! mesos received. A cancel, or a side that can't take the other's offer,
//! gives everyone their own offer back.

use integration_harness::database::{grant_stack, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MAP: i32 = 40_000;
const OTHER_MAP: i32 = 50_000;
const RED_POTION: i32 = 2000000;
const SNAIL_SHELL: i32 = 4000000;
const BLUE_SNAIL_SHELL: i32 = 4000001;
const USE: u8 = 2;
const ETC: u8 = 4;

fn interaction(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PlayerInteraction as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn create() -> Packet {
    let mut p = interaction(0);
    p.write_bytes(&[3, 0]).unwrap();
    p
}

fn invite(target: i32) -> Packet {
    let mut p = interaction(2);
    p.write_int(target).unwrap();
    p
}

fn decline(room: i32) -> Packet {
    let mut p = interaction(3);
    p.write_int(room).unwrap();
    p.write_byte(3).unwrap();
    p
}

fn visit(room: i32) -> Packet {
    let mut p = interaction(4);
    p.write_int(room).unwrap();
    p.write_bytes(&[0, 0]).unwrap();
    p
}

fn chat(text: &str) -> Packet {
    let mut p = interaction(6);
    p.write_str_with_length(text).unwrap();
    p
}

fn put_item(kind: u8, slot: i16, count: i16, trade_slot: u8) -> Packet {
    let mut p = interaction(0xf);
    p.write_byte(kind).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(count).unwrap();
    p.write_byte(trade_slot).unwrap();
    p
}

fn put_meso(amount: i32) -> Packet {
    let mut p = interaction(0x10);
    p.write_int(amount).unwrap();
    p
}

fn confirm() -> Packet {
    let mut p = interaction(0x11);
    p.write_byte(0).unwrap();
    p
}

fn leave() -> Packet {
    interaction(0xa)
}

/// A room packet that ends in a string.
fn named(head: &[u8], name: &str) -> Vec<u8> {
    let mut body = head.to_vec();
    body.write_str_with_length(name).unwrap();
    body
}

/// Items shown on one side (0 the receiver's own): (trade slot, item id).
fn offered(t: &Trace, who: u8) -> Vec<(u8, i32)> {
    t.rooms
        .iter()
        .filter(|b| b[..2] == [0xf, who])
        .map(|b| (b[2], (&b[4..8]).read_int().unwrap()))
        .collect()
}

/// Mesos shown on one side, as each update said.
fn meso_offers(t: &Trace, who: u8) -> Vec<i32> {
    t.rooms
        .iter()
        .filter(|b| b[..2] == [0x10, who])
        .map(|b| (&b[2..]).read_int().unwrap())
        .collect()
}

/// Make the character behind `config` and log it out again; its id and bag
/// sizes.
async fn new_character(config: &HarnessConfig) -> (i32, [u8; 5]) {
    let session = login_to_world_session(config).await.expect("first login");
    let made = (session.character_id, session.slot_limits);
    drop(session);
    drop(relogin(config).await);
    made
}

/// Both on `map`, each seeing the other.
async fn meet(a: &mut WorldSession, b: &mut WorldSession, map: i32) {
    let (mut at, mut bt) = (Trace::default(), Trace::default());
    assert_eq!(warp(&mut a.connection, map, "", &mut at).await, map);
    assert_eq!(warp(&mut b.connection, map, "", &mut bt).await, map);
    read_for(&mut a.connection, &mut at, 300).await;
    read_for(&mut b.connection, &mut bt, 300).await;
}

/// A opens a room and B joins it.
async fn open_room(a: &mut WorldSession, b: &mut WorldSession) {
    let t = step(&mut a.connection, "create", create()).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    assert_eq!(t.rooms[0][..4], [5, 3, 2, 0]);
    send(&mut a.connection, invite(b.character_id)).await;
    let room = invitation(&mut b.connection, &a.character_name).await;
    let t = step(&mut b.connection, "visit", visit(room)).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    assert_eq!(t.rooms[0][..4], [5, 3, 2, 1]);
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.rooms.is_empty()).await;
    assert_eq!(at.rooms[0][..2], [4, 1]);
}

/// Wait for an invitation from `from` and return its room.
async fn invitation(
    c: &mut integration_harness::connection::MapleTestConnection,
    from: &str,
) -> i32 {
    let mut t = Trace::default();
    until(c, &mut t, |t| t.rooms.iter().any(|b| b[0] == 2)).await;
    let body = t.rooms.iter().find(|b| b[0] == 2).unwrap();
    let mut r = &body[1..];
    assert_eq!(r.read_byte().unwrap(), 3);
    assert_eq!(r.read_str_with_length().unwrap(), from);
    r.read_int().unwrap()
}

/// Wait for the room to close with `leave`, and return what came with it.
async fn closed(
    c: &mut integration_harness::connection::MapleTestConnection,
    leave: [u8; 3],
) -> Trace {
    let mut t = Trace::default();
    until(c, &mut t, |t| t.rooms.iter().any(|b| b[..] == leave)).await;
    t
}

#[tokio::test]
async fn two_players_swap_items_and_mesos_less_the_fee() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let (a_id, _) = new_character(&a_config).await;
    let (b_id, _) = new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(a_id, 1_000_000).await.expect("give A mesos");
    grant_stack(a_id, ETC.into(), 1, SNAIL_SHELL, 10)
        .await
        .expect("give A snail shells");
    set_meso(b_id, 500_000).await.expect("give B mesos");
    grant_stack(b_id, USE.into(), 1, RED_POTION, 5)
        .await
        .expect("give B red potions");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;
    meet(&mut a, &mut b, MAP).await;

    // Someone not on the map can't be invited.
    let t = step(&mut a.connection, "create", create()).await;
    assert_eq!(t.rooms[0][..4], [5, 3, 2, 0]);
    let t = step(&mut a.connection, "nobody", invite(i32::MAX)).await;
    assert_eq!(t.rooms, [vec![3, 1]]);

    // B turns the invitation down; A's room stays open for another.
    send(&mut a.connection, invite(b_id)).await;
    let room = invitation(&mut b.connection, &a.character_name).await;
    send(&mut b.connection, decline(room)).await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.rooms.is_empty()).await;
    assert_eq!(at.rooms, [named(&[3, 3], &b.character_name)]);

    send(&mut a.connection, invite(b_id)).await;
    let room = invitation(&mut b.connection, &a.character_name).await;
    let t = step(&mut b.connection, "visit", visit(room)).await;
    assert_eq!(t.rooms[0][..4], [5, 3, 2, 1]);
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.rooms.is_empty()).await;
    assert_eq!(at.rooms[0][..2], [4, 1]);

    // Chat reaches both, with the speaker's name.
    let line = named(&[6, 8, 1], &format!("{} : hello", b.character_name));
    let t = step(&mut b.connection, "chat", chat("hello")).await;
    assert_eq!(t.rooms, [line.clone()]);
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| t.rooms.contains(&line)).await;

    // Part of A's shells and all of B's potions go up, shown on both sides.
    let t = step(&mut a.connection, "shells", put_item(ETC, 1, 4, 1)).await;
    assert_eq!(t.quantities, [(ETC, 1, 6)]);
    assert_eq!(offered(&t, 0), [(1, SNAIL_SHELL)]);
    let t = step(&mut b.connection, "potions", put_item(USE, 1, 5, 3)).await;
    assert_eq!(t.removed, [(USE, 1)]);
    assert_eq!(offered(&t, 0), [(3, RED_POTION)]);
    assert_eq!(offered(&t, 1), [(1, SNAIL_SHELL)]);

    // A level 1 may put up at most a million mesos.
    let t = step(&mut a.connection, "meso", put_meso(200_000)).await;
    assert_eq!(t.stat(stat::MESO), Some(800_000));
    assert_eq!(meso_offers(&t, 0), [200_000]);
    let t = step(&mut a.connection, "too-much", put_meso(900_000)).await;
    assert_eq!(t.rooms, [vec![0x15]]);
    assert_eq!(t.stat(stat::MESO), None);
    let t = step(&mut b.connection, "b-meso", put_meso(100_000)).await;
    assert_eq!(t.stat(stat::MESO), Some(400_000));
    assert_eq!(meso_offers(&t, 1), [200_000]);
    assert_eq!(meso_offers(&t, 0), [100_000]);

    // A presses Trade, then B: the offers swap.
    send(&mut a.connection, confirm()).await;
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| {
        t.rooms.contains(&vec![0x11])
    })
    .await;
    send(&mut b.connection, confirm()).await;
    // The meso update comes before the room closes: the client words its
    // success message by it. 0.8% of 100,000 and of 200,000 go in fees.
    let at = closed(&mut a.connection, [0xa, 0, 7]).await;
    assert!(at.rooms.contains(&vec![0x11]));
    assert!(at
        .added
        .iter()
        .any(|&(kind, _, id)| (kind, id) == (USE, RED_POTION)));
    assert_eq!(at.stat(stat::MESO), Some(800_000 + 99_200));
    let bt = closed(&mut b.connection, [0xa, 1, 7]).await;
    assert!(bt
        .added
        .iter()
        .any(|&(kind, _, id)| (kind, id) == (ETC, SNAIL_SHELL)));
    assert_eq!(bt.stat(stat::MESO), Some(400_000 + 198_400));
    drop(a);
    drop(b);

    let a = relogin(&a_config).await;
    assert_eq!(a.meso, 899_200);
    assert_eq!(a.items.get(&SNAIL_SHELL), Some(&6));
    assert_eq!(a.items.get(&RED_POTION), Some(&5));
    let b = relogin(&b_config).await;
    assert_eq!(b.meso, 598_400);
    assert_eq!(b.items.get(&SNAIL_SHELL), Some(&4));
    assert_eq!(b.items.get(&RED_POTION), None);
}

#[tokio::test]
async fn a_cancel_or_a_full_bag_gives_everything_back() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let (a_id, _) = new_character(&a_config).await;
    let (b_id, b_bags) = new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(a_id, 50_000).await.expect("give A mesos");
    grant_stack(a_id, ETC.into(), 1, SNAIL_SHELL, 10)
        .await
        .expect("give A snail shells");
    set_meso(b_id, 30_000).await.expect("give B mesos");
    for slot in 1..=i16::from(b_bags[usize::from(ETC) - 1]) {
        grant_stack(b_id, ETC.into(), slot, BLUE_SNAIL_SHELL, 1)
            .await
            .expect("fill B's Etc bag");
    }
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;
    meet(&mut a, &mut b, OTHER_MAP).await;

    // B walks out of the room: A gets back what they put up.
    open_room(&mut a, &mut b).await;
    let t = step(&mut a.connection, "shells", put_item(ETC, 1, 4, 1)).await;
    assert_eq!(t.quantities, [(ETC, 1, 6)]);
    let t = step(&mut a.connection, "meso", put_meso(10_000)).await;
    assert_eq!(t.stat(stat::MESO), Some(40_000));
    let t = step(&mut b.connection, "leave", leave()).await;
    // The one leaving closed their window already.
    assert!(t.rooms.iter().all(|b| b[0] != 0xa), "{:?}", t.rooms);
    let at = closed(&mut a.connection, [0xa, 0, 2]).await;
    assert_eq!(at.quantities, [(ETC, 1, 10)]);
    assert_eq!(at.stat(stat::MESO), Some(50_000));

    // B's Etc bag has no room for shells: the trade fails for both.
    open_room(&mut a, &mut b).await;
    step(&mut a.connection, "shells", put_item(ETC, 1, 4, 1)).await;
    let t = step(&mut b.connection, "meso", put_meso(10_000)).await;
    assert_eq!(t.stat(stat::MESO), Some(20_000));
    step(&mut a.connection, "a-trade", confirm()).await;
    send(&mut b.connection, confirm()).await;
    let at = closed(&mut a.connection, [0xa, 0, 8]).await;
    assert_eq!(at.quantities, [(ETC, 1, 10)]);
    assert_eq!(at.stat(stat::MESO), None);
    let bt = closed(&mut b.connection, [0xa, 1, 8]).await;
    assert_eq!(bt.stat(stat::MESO), Some(30_000));
    assert!(bt.added.is_empty() && bt.quantities.is_empty());
    drop(a);
    drop(b);

    let a = relogin(&a_config).await;
    assert_eq!(a.meso, 50_000);
    assert_eq!(a.items.get(&SNAIL_SHELL), Some(&10));
    let b = relogin(&b_config).await;
    assert_eq!(b.meso, 30_000);
    assert_eq!(b.items.get(&SNAIL_SHELL), None);
}
