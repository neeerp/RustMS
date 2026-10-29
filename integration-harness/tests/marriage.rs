//! Engagement (report b052): a player proposes with an engagement ring box
//! (RING_ACTION 0) to a player of the other gender on their map, who is
//! asked (MARRIAGE_REQUEST 0) and answers (RING_ACTION 2). The server
//! refuses a wrong name, another map, the same gender, too low a level, a
//! second proposal, and a partner already asked or engaged; a proposer may
//! withdraw (RING_ACTION 1). On a yes both get MARRIAGE_RESULT 11 with the
//! couple's record, the proposer the empty box and the partner the ring.
//! Dropping either token (RING_ACTION 3, report b054) breaks it off: both
//! get MARRIAGE_RESULT 13 and lose their tokens. While both are online each
//! is told the other's map (NOTIFY_MARRIED_PARTNER_MAP_TRANSFER, report
//! b057), and (0, 0) once the other logs off or the couple ends. A wedding
//! ring worn by either shows the couple's ring block to the map (report
//! b059).

use integration_harness::database::{
    grant_stack, item_owner_and_hours, marry, set_gender, set_job, stow_equip, wear_equip,
};
use integration_harness::packets::RingLooks;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const HENESYS: i32 = 100_000_000;

/// The marriage ring block of each UPDATE_CHAR_LOOK.
fn marriages(looks: &[(i32, RingLooks)]) -> Vec<(i32, Option<(i32, i32, i32)>)> {
    looks
        .iter()
        .map(|&(id, rings)| (id, rings.marriage))
        .collect()
}
const PERION: i32 = 102_000_000;
const MOONSTONE_BOX: i32 = 2240000;
const EMPTY_BOX: i32 = 4031357;
const RING: i32 = 4031358;
const WEDDING_RING: i32 = 1112803;
const RING_SLOT: i16 = -12;
const USE: i16 = 2;
const ETC: u8 = 4;

const ENGAGED: u8 = 11;
const BROKEN: u8 = 13;
const DIVORCED: u8 = 14;
const WRONG_NAME: u8 = 18;
const NOT_SAME_MAP: u8 = 19;
const SAME_GENDER: u8 = 22;
const ALREADY_ENGAGED: u8 = 23;
const PARTNER_ENGAGED: u8 = 24;
const PROPOSING: u8 = 27;
const BEING_ASKED: u8 = 28;
const WITHDREW: u8 = 29;
const DECLINED: u8 = 30;
const NOTICE: u8 = 36;

fn propose(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_int(MOONSTONE_BOX).unwrap();
    p
}

fn cancel() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(1).unwrap();
    p
}

fn answer(accept: bool, name: &str, proposer: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(2).unwrap();
    p.write_byte(u8::from(accept)).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_int(proposer).unwrap();
    p
}

fn break_off(token: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(3).unwrap();
    p.write_int(token).unwrap();
    p
}

fn item_move(from: i16, to: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(1).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(1).unwrap();
    p
}

fn result(mode: u8) -> (i16, Vec<u8>) {
    (SendOpcode::MarriageResult as i16, vec![mode])
}

fn asked(name: &str, proposer: i32) -> (i16, Vec<u8>) {
    let mut body = vec![0];
    body.extend((name.len() as u16).to_le_bytes());
    body.extend(name.as_bytes());
    body.extend(proposer.to_le_bytes());
    (SendOpcode::MarriageRequest as i16, body)
}

fn padded(name: &str) -> Vec<u8> {
    let mut bytes = name.as_bytes().to_vec();
    bytes.resize(13, 0);
    bytes
}

/// Make the character behind `config` of `gender` at `level`, with `boxes`
/// ring boxes, and log it out; its id.
async fn new_character(config: &HarnessConfig, gender: i16, level: i16, boxes: i16) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.expect("level");
    set_gender(id, gender).await.expect("gender");
    if boxes > 0 {
        grant_stack(id, USE, 20, MOONSTONE_BOX, boxes)
            .await
            .expect("ring box");
    }
    id
}

async fn enter(config: &HarnessConfig, map: i32) -> WorldSession {
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut session.connection, map, "", &mut t).await, map);
    session
}

/// The next `count` marriage packets `session` gets, and the items it
/// gained meanwhile.
async fn marriage(session: &mut WorldSession, count: usize) -> Trace {
    let mut t = Trace::default();
    until(&mut session.connection, &mut t, |t| {
        t.marriage.len() >= count
    })
    .await;
    read_for(&mut session.connection, &mut t, 300).await;
    t
}

/// As `marriage`, then the partner map notice that follows a couple's
/// change, which waits for the actor's save.
async fn marriage_and_map(session: &mut WorldSession, count: usize) -> Trace {
    let mut t = marriage(session, count).await;
    until(&mut session.connection, &mut t, |t| {
        !t.partner_maps.is_empty()
    })
    .await;
    read_for(&mut session.connection, &mut t, 300).await;
    t
}

async fn refused(session: &mut WorldSession, name: &str, mode: u8) {
    send(&mut session.connection, propose(name)).await;
    assert_eq!(marriage(session, 1).await.marriage, [result(mode)]);
}

#[tokio::test]
async fn a_proposal_said_yes_to_engages_the_two() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let [a_config, b_config, c_config, d_config] = &configs;
    let a_id = new_character(a_config, 0, 50, 2).await;
    let b_id = new_character(b_config, 1, 50, 0).await;
    new_character(c_config, 0, 50, 1).await;
    new_character(d_config, 1, 10, 0).await;
    let [an, bn, cn, dn] = configs.each_ref().map(|c| c.character_name.clone());

    let mut a = enter(a_config, HENESYS).await;
    let mut b = enter(b_config, PERION).await;
    let mut c = enter(c_config, HENESYS).await;
    let d = enter(d_config, HENESYS).await;

    refused(&mut a, "nobody here", WRONG_NAME).await;
    refused(&mut a, &an, WRONG_NAME).await;
    refused(&mut a, &bn, NOT_SAME_MAP).await;
    refused(&mut a, &cn, SAME_GENDER).await;
    send(&mut a.connection, propose(&dn)).await;
    let notice = marriage(&mut a, 1).await.marriage;
    assert_eq!(notice[0].1[..2], [NOTICE, 1]);
    drop(d);

    // B is asked; A may not ask again, nor C ask B meanwhile.
    let mut bt = Trace::default();
    assert_eq!(warp(&mut b.connection, HENESYS, "", &mut bt).await, HENESYS);
    send(&mut a.connection, propose(&bn)).await;
    assert_eq!(marriage(&mut b, 1).await.marriage, [asked(&an, a_id)]);
    refused(&mut a, &bn, PROPOSING).await;
    refused(&mut c, &bn, BEING_ASKED).await;

    // A withdraws; asked again, B says no.
    send(&mut a.connection, cancel()).await;
    assert_eq!(marriage(&mut b, 1).await.marriage, [result(WITHDREW)]);
    send(&mut a.connection, propose(&bn)).await;
    assert_eq!(marriage(&mut b, 1).await.marriage, [asked(&an, a_id)]);
    send(&mut b.connection, answer(false, &an, a_id)).await;
    assert_eq!(marriage(&mut a, 1).await.marriage, [result(DECLINED)]);

    // Then yes: both get the record, A the empty box and B the ring.
    send(&mut a.connection, propose(&bn)).await;
    assert_eq!(marriage(&mut b, 1).await.marriage, [asked(&an, a_id)]);
    send(&mut b.connection, answer(true, &an, a_id)).await;
    let at = marriage_and_map(&mut a, 1).await;
    let bt = marriage_and_map(&mut b, 1).await;
    let (op, record) = &at.marriage[0];
    assert_eq!(*op, SendOpcode::MarriageResult as i16);
    assert_eq!(record.len(), 1 + 0x30);
    assert_eq!(record[0], ENGAGED);
    assert_eq!(record[5..9], a_id.to_le_bytes());
    assert_eq!(record[9..13], b_id.to_le_bytes());
    assert_eq!(record[13..15], 1i16.to_le_bytes());
    assert_eq!(record[15..19], WEDDING_RING.to_le_bytes());
    assert_eq!(record[19..23], WEDDING_RING.to_le_bytes());
    assert_eq!(record[23..36], padded(&an));
    assert_eq!(record[36..49], padded(&bn));
    assert_eq!(bt.marriage, at.marriage);
    assert_eq!(at.items, [(EMPTY_BOX, 1)]);
    assert_eq!(bt.items, [(RING, 1)]);
    assert_eq!(at.partner_maps, [(HENESYS, b_id)]);
    assert_eq!(bt.partner_maps, [(HENESYS, a_id)]);

    // Engaged now, on this login and the next.
    refused(&mut c, &bn, PARTNER_ENGAGED).await;
    refused(&mut a, &cn, ALREADY_ENGAGED).await;
    // A wedding ring worn while engaged shows the couple (a player with
    // no couple has every one taken on login).
    drop(a);
    tokio::time::sleep(Duration::from_millis(500)).await;
    wear_equip(a_id, RING_SLOT, WEDDING_RING)
        .await
        .expect("a wedding ring worn while engaged");
    let mut a = enter(a_config, HENESYS).await;
    refused(&mut a, &cn, ALREADY_ENGAGED).await;

    // B saw A go and come back; A sees B move.
    let mut bt = Trace::default();
    assert_eq!(warp(&mut b.connection, PERION, "", &mut bt).await, PERION);
    assert_eq!(warp(&mut b.connection, HENESYS, "", &mut bt).await, HENESYS);
    until(&mut b.connection, &mut bt, |t| t.partner_maps.len() >= 2).await;
    read_for(&mut b.connection, &mut bt, 300).await;
    assert_eq!(bt.partner_maps, [(0, 0), (HENESYS, a_id)]);
    let seen = bt.players.iter().find(|p| p.character_id == a_id).unwrap();
    assert_eq!(seen.rings.marriage, Some((a_id, b_id, WEDDING_RING)));
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| t.partner_maps.len() >= 2).await;
    assert_eq!(at.partner_maps, [(PERION, b_id), (HENESYS, b_id)]);

    // B drops the ring: both are told, and both tokens go, with A's
    // wedding ring.
    send(&mut b.connection, break_off(RING)).await;
    let mut bt = marriage_and_map(&mut b, 1).await;
    until(&mut b.connection, &mut bt, |t| !t.looks.is_empty()).await;
    assert_eq!(marriages(&bt.looks), [(a_id, None)]);
    let at = marriage_and_map(&mut a, 1).await;
    assert_eq!(bt.marriage, [result(BROKEN)]);
    assert_eq!(at.marriage, [result(BROKEN)]);
    assert!(matches!(bt.removed[..], [(ETC, _)]), "{:?}", bt.removed);
    assert!(
        matches!(at.removed[..], [(1, RING_SLOT), (ETC, _)]),
        "{:?}",
        at.removed
    );
    assert_eq!(bt.partner_maps, [(0, 0)]);
    assert_eq!(at.partner_maps, [(0, 0)]);

    // Free again: A may ask B once more.
    send(&mut a.connection, propose(&bn)).await;
    assert_eq!(marriage(&mut b, 1).await.marriage, [asked(&an, a_id)]);
}

#[tokio::test]
async fn a_wedding_ring_worn_shows_the_couple_to_the_map() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let [a_config, b_config] = &configs;
    let a_id = new_character(a_config, 0, 50, 0).await;
    let b_id = new_character(b_config, 1, 50, 0).await;
    marry(a_id, b_id, MOONSTONE_BOX).await.expect("marry");
    wear_equip(a_id, RING_SLOT, WEDDING_RING)
        .await
        .expect("A's ring");
    stow_equip(b_id, 1, WEDDING_RING, 0)
        .await
        .expect("B's ring");
    grant_stack(a_id, i16::from(ETC), 1, EMPTY_BOX, 1)
        .await
        .expect("A's token");

    // B arrives and sees A wear the ring; A sees B without one.
    let mut a = enter(a_config, HENESYS).await;
    let mut b = enter(b_config, PERION).await;
    let mut bt = Trace::default();
    assert_eq!(warp(&mut b.connection, HENESYS, "", &mut bt).await, HENESYS);
    until(&mut b.connection, &mut bt, |t| {
        t.players.iter().any(|p| p.character_id == a_id)
    })
    .await;
    let seen = bt.players.iter().find(|p| p.character_id == a_id).unwrap();
    assert_eq!(seen.rings.marriage, Some((a_id, b_id, WEDDING_RING)));
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.players.iter().any(|p| p.character_id == b_id)
    })
    .await;
    let seen = at.players.iter().find(|p| p.character_id == b_id).unwrap();
    assert_eq!(seen.rings.marriage, None);

    // B puts the ring on, then takes it off.
    send(&mut b.connection, item_move(1, RING_SLOT)).await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.looks.is_empty()).await;
    assert_eq!(
        marriages(&at.looks),
        [(b_id, Some((b_id, a_id, WEDDING_RING)))]
    );
    send(&mut b.connection, item_move(RING_SLOT, 1)).await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.looks.is_empty()).await;
    assert_eq!(marriages(&at.looks), [(b_id, None)]);

    // With B away, A breaks it off with a token kept: A loses the worn
    // ring with the token, and B the stowed one on the next login.
    drop(b);
    tokio::time::sleep(Duration::from_millis(500)).await;
    send(&mut a.connection, break_off(EMPTY_BOX)).await;
    let at = marriage(&mut a, 1).await;
    assert_eq!(at.marriage, [result(DIVORCED)]);
    assert!(at.removed.contains(&(1, RING_SLOT)), "{:?}", at.removed);
    assert!(at.removed.iter().any(|&(kind, _)| kind == ETC));
    drop(enter(b_config, HENESYS).await);
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(item_owner_and_hours(b_id, WEDDING_RING).await.is_err());
    assert!(item_owner_and_hours(a_id, WEDDING_RING).await.is_err());
}

fn spouse_chat(text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpouseChat as i16).unwrap();
    // The client's name for the partner; the server goes by its record.
    p.write_str_with_length("Anyone").unwrap();
    p.write_str_with_length(text).unwrap();
    p
}

/// The next SPOUSE_CHAT `session` gets, past its opcode.
async fn spouse_line(session: &mut WorldSession) -> Vec<u8> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let envelope = tokio::time::timeout_at(deadline, session.connection.read_packet("spouse"))
            .await
            .expect("a spouse chat packet")
            .expect("read");
        if envelope.opcode() == SendOpcode::SpouseChat as i16 {
            return envelope.packet.bytes[2..].to_vec();
        }
    }
}

fn string_bytes(s: &str) -> Vec<u8> {
    let mut bytes = (s.len() as u16).to_le_bytes().to_vec();
    bytes.extend_from_slice(s.as_bytes());
    bytes
}

/// Spouse chat (FUN_008defff, FUN_00532087): a line goes to the partner on
/// the server's record wherever they are, mode 5 with the sender's name, and
/// the sender gets mode 4 with the text; with the partner away, or without
/// a partner, mode 4 says no one heard.
#[tokio::test]
async fn spouse_chat_reaches_the_partner() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let [a_config, b_config] = &configs;
    let a_id = new_character(a_config, 0, 50, 0).await;
    let b_id = new_character(b_config, 1, 50, 0).await;

    // Not married yet.
    let mut a = enter(a_config, HENESYS).await;
    send(&mut a.connection, spouse_chat("anyone?")).await;
    assert_eq!(spouse_line(&mut a).await, [4, 0]);
    drop(a);
    tokio::time::sleep(Duration::from_millis(500)).await;

    marry(a_id, b_id, MOONSTONE_BOX).await.expect("marry");
    let mut a = enter(a_config, HENESYS).await;
    let mut b = enter(b_config, PERION).await;

    send(&mut a.connection, spouse_chat("hello dear")).await;
    let mut heard = vec![5];
    heard.extend(string_bytes(&a.character_name));
    heard.push(5);
    heard.extend(string_bytes("hello dear"));
    assert_eq!(spouse_line(&mut b).await, heard);
    let mut echo = vec![4, 1];
    echo.extend(string_bytes("hello dear"));
    assert_eq!(spouse_line(&mut a).await, echo);

    // The partner gone, no one hears.
    // The partner's logout reaches the directory a little later.
    drop(b);
    let mut heard = Vec::new();
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        send(&mut a.connection, spouse_chat("still there?")).await;
        heard = spouse_line(&mut a).await;
        if heard == [4, 0] {
            break;
        }
    }
    assert_eq!(heard, [4, 0]);
}
