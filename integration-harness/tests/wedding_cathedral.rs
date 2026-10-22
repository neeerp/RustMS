//! The Cathedral wedding (report b075, slice 7d): the High Priest in
//! Amoria starts an engaged couple's booked wedding, and at the altar he
//! welcomes everyone, calls for blessings once the couple close his
//! welcome, and then asks the guests to bless the couple. Each guest's
//! blessing counts once, and only until the couple marry; the couple and
//! the guests' own closes don't move him on. The vows bring his
//! pronouncement, its close the hearts and his farewell, and Nancy sees
//! the couple out.

use game_data::server_content::keys;
use integration_harness::database::{
    add_wedding_guest, book_wedding, engage, grant_stack, replace_item, set_gender, set_job,
    wedding_due,
};
use integration_harness::events::talk;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const AMORIA: i32 = 680_000_000;
const HALL: i32 = 680_000_200;
const ALTAR: i32 = 680_000_210;
const EXIT: i32 = 680_000_500;
const HIGH_PRIEST: i32 = 9201002;
const NANCY: i32 = 9201007;
const NICOLE: i32 = 9201005;
/// The invitation a Cathedral guest receives.
const INVITATION: i32 = 4031407;
/// The assistants' menu choice for guests.
const AS_GUEST: i32 = 2;
const UNDERSHIRT: i32 = 1040002;
const MOONSTONE_BOX: i32 = 2240000;
const EMPTY_BOX: i32 = 4031357;
const RING: i32 = 4031358;
const GROOM_CLOTHES: i32 = 1050113;
const BRIDE_CLOTHES: i32 = 1051114;
const ETC: i16 = 4;
const EQUIPPED: i16 = -1;
const TOP: i16 = -5;
const CATHEDRAL: i16 = 1;
const MARRIED: u8 = 12;
/// BROADCAST_MSG in pink.
const PINK: u8 = 5;

/// WEDDING_PROGRESS: the priest's `step` for the couple.
fn priest(step: u8, groom: i32, bride: i32) -> (i16, Vec<u8>) {
    let mut body = vec![step];
    body.extend(groom.to_le_bytes());
    body.extend(bride.to_le_bytes());
    (SendOpcode::WeddingProgress as i16, body)
}

fn hearts() -> (i16, Vec<u8>) {
    (SendOpcode::WeddingCeremonyEnd as i16, vec![])
}

/// Tell the priest the dialog of `step` is closed.
fn talked(step: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::WeddingTalk as i16).unwrap();
    p.write_byte(step).unwrap();
    p
}

/// Yes to "Do you want to bless this couple?".
fn bless() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::WeddingBless as i16).unwrap();
    p
}

/// Read on from `t` until `session` has `count` priest packets.
async fn hear(session: &mut WorldSession, count: usize, mut t: Trace) -> Trace {
    until(&mut session.connection, &mut t, |t| {
        t.wedding.len() >= count
    })
    .await;
    read_for(&mut session.connection, &mut t, 300).await;
    t
}

/// Send `packet` from `session`; what followed within half a second.
async fn quietly(session: &mut WorldSession, what: &str, packet: Packet) -> Trace {
    let m = session.character_name.clone();
    let mut t = step(&mut session.connection, &format!("{m} {what}"), packet).await;
    read_for(&mut session.connection, &mut t, 500).await;
    t
}

fn marriage_results(t: &Trace) -> Vec<u8> {
    t.marriage
        .iter()
        .filter(|(op, _)| *op == SendOpcode::MarriageResult as i16)
        .filter_map(|(_, body)| body.first().copied())
        .collect()
}

/// Make the character behind `config` of `gender` at level 50, wearing
/// `clothes`, and log it out; its id.
async fn new_character(config: &HarnessConfig, gender: i16, clothes: i32) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.expect("level");
    set_gender(id, gender).await.expect("gender");
    replace_item(id, EQUIPPED, TOP, clothes)
        .await
        .expect("clothes");
    id
}

async fn enter(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut session.connection, AMORIA, "", &mut t).await,
        AMORIA
    );
    session
}

/// Say yes to `npc`'s question on `map`; what followed.
async fn say_yes(session: &mut WorldSession, map: i32, npc: i32) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talks to {npc}"), talk(map, npc)).await;
    assert_eq!(t.prompts, [(npc, 1)], "{:?}", t.prompt_texts);
    step(c, &format!("{m} yes"), answer(1, 1, None)).await
}

/// Read on from `t` until `session` arrives on `map`.
async fn arrive(session: &mut WorldSession, map: i32, mut t: Trace) -> Trace {
    until(&mut session.connection, &mut t, |t| t.warps.contains(&map)).await;
    read_for(&mut session.connection, &mut t, 300).await;
    t
}

/// Go into the wedding as a guest through Nicole.
async fn go_in(session: &mut WorldSession) {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talks to Nicole"), talk(AMORIA, NICOLE)).await;
    assert_eq!(t.prompts, [(NICOLE, 4)], "{:?}", t.prompt_texts);
    send(c, answer(4, 1, Some(AS_GUEST))).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.prompts.is_empty()).await;
    assert_eq!(t.prompts, [(NICOLE, 1)], "{:?}", t.prompt_texts);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    arrive(session, HALL, t).await;
}

#[tokio::test]
async fn the_high_priest_marries_a_couple_their_guests_bless() {
    let mut configs = Vec::new();
    for _ in 0..4 {
        configs.push(load_harness_config_or_fail().await);
    }
    let groom = new_character(&configs[0], 0, GROOM_CLOTHES).await;
    let bride = new_character(&configs[1], 1, BRIDE_CLOTHES).await;
    engage(groom, bride, MOONSTONE_BOX).await.expect("engage");
    grant_stack(groom, ETC, 1, EMPTY_BOX, 1).await.expect("box");
    grant_stack(bride, ETC, 1, RING, 1).await.expect("ring");
    let marriage_id = book_wedding(groom, CATHEDRAL, 0).await.expect("book");
    wedding_due(marriage_id, 0).await.expect("due");
    let mut guests = Vec::new();
    for config in &configs[2..] {
        let id = new_character(config, 0, UNDERSHIRT).await;
        grant_stack(id, ETC, 1, INVITATION, 1)
            .await
            .expect("invitation");
        add_wedding_guest(marriage_id, id).await.expect("guest");
        guests.push(enter(config).await);
    }
    let mut late = guests.pop().unwrap();
    let mut blesser = guests.pop().unwrap();
    let mut a = enter(&configs[0]).await;
    let mut b = enter(&configs[1]).await;

    // The High Priest in Amoria starts it; the guests come in through
    // Nicole while the couple gather.
    let at = say_yes(&mut a, AMORIA, HIGH_PRIEST).await;
    arrive(&mut a, HALL, at).await;
    arrive(&mut b, HALL, Trace::default()).await;
    go_in(&mut blesser).await;
    go_in(&mut late).await;

    // At the altar everyone hears his welcome.
    let mut everyone = [&mut a, &mut b, &mut blesser, &mut late];
    for session in everyone.iter_mut() {
        let t = arrive(session, ALTAR, Trace::default()).await;
        let t = hear(session, 1, t).await;
        assert_eq!(t.wedding, [priest(0, groom, bride)]);
    }

    // A's close brings his call for blessings (well before he would go on
    // by himself); B's late close of the welcome changes nothing.
    let at = quietly(&mut a, "closes the welcome", talked(0)).await;
    assert_eq!(at.wedding, [priest(1, groom, bride)]);
    let bt = quietly(&mut b, "closes the welcome", talked(0)).await;
    assert_eq!(bt.wedding, [priest(1, groom, bride)]);

    // A guest can't bless yet, and their close moves nothing on.
    let t = quietly(&mut blesser, "blesses early", bless()).await;
    assert_eq!(t.broadcasts, []);
    assert_eq!(t.wedding, [priest(1, groom, bride)]);
    let t = quietly(&mut blesser, "closes", talked(1)).await;
    assert_eq!(t.wedding, []);

    // A's close of the call brings the blessing.
    let at = quietly(&mut a, "closes the call", talked(1)).await;
    assert_eq!(at.wedding, [priest(2, groom, bride)]);
    let bt = hear(&mut b, 1, Trace::default()).await;
    assert_eq!(bt.wedding, [priest(2, groom, bride)]);
    let t = hear(&mut blesser, 1, Trace::default()).await;
    assert_eq!(t.wedding, [priest(2, groom, bride)]);
    let t = hear(&mut late, 2, Trace::default()).await;
    assert_eq!(
        t.wedding,
        [priest(1, groom, bride), priest(2, groom, bride)]
    );

    // The guest's blessing counts once; the couple have none to give.
    let blessed = net::content::text().get(keys::WEDDING_BLESSED, &[]);
    let t = quietly(&mut blesser, "blesses", bless()).await;
    assert_eq!(t.broadcasts, [(PINK, blessed)]);
    let t = quietly(&mut blesser, "blesses again", bless()).await;
    assert_eq!(t.broadcasts, []);
    let t = quietly(&mut a, "blesses", bless()).await;
    assert_eq!(t.broadcasts, []);

    // The vows marry them and bring the pronouncement; after that a
    // blessing is too late.
    let at = say_yes(&mut a, ALTAR, HIGH_PRIEST).await;
    assert_eq!(at.marriage, []);
    let mut bt = say_yes(&mut b, ALTAR, HIGH_PRIEST).await;
    until(&mut b.connection, &mut bt, |t| !t.marriage.is_empty()).await;
    let bt = hear(&mut b, 1, bt).await;
    assert_eq!(marriage_results(&bt), [MARRIED]);
    assert_eq!(bt.wedding, [priest(3, groom, bride)]);
    let lt = hear(&mut late, 1, Trace::default()).await;
    assert_eq!(lt.wedding, [priest(3, groom, bride)]);
    let t = quietly(&mut late, "blesses late", bless()).await;
    assert_eq!(t.broadcasts, []);

    // A's close of it brings the hearts and the farewell.
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.marriage.is_empty()).await;
    let at = hear(&mut a, 1, at).await;
    assert_eq!(at.wedding, [priest(3, groom, bride)]);
    let at = quietly(&mut a, "closes the pronouncement", talked(3)).await;
    let at = hear(&mut a, 2, at).await;
    assert_eq!(at.wedding, [hearts(), priest(4, groom, bride)]);
    let t = hear(&mut blesser, 3, Trace::default()).await;
    assert_eq!(
        t.wedding,
        [priest(3, groom, bride), hearts(), priest(4, groom, bride)]
    );

    // Nancy sees them out.
    let at = say_yes(&mut a, ALTAR, NANCY).await;
    arrive(&mut a, EXIT, at).await;
    arrive(&mut b, EXIT, Trace::default()).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
}
