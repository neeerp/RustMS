//! The Chapel wedding (report b071, slice 7a): Wayne in Amoria starts an
//! engaged couple's booked wedding when it is due and both stand beside
//! him, dressed for it. The couple gather in their own copy of the hall,
//! then go on to the altar, where the priest takes each one's vows; the
//! second marries them: the tokens go, each gets the wedding ring and
//! MARRIAGE_RESULT 12, and the booking is gone. Jackie then sees them out.
//! A partner who leaves before the vows ends the wedding and its booking
//! (MARRIAGE_RESULT 31).
//!
//! Guests (slice 7b) holding their invitation come in through Bonnie while
//! the couple gather, go on to the altar with them, may leave through
//! Jackie or by logging out without ending anything, and are seen out with
//! the couple at the end.
//!
//! In the hall (report b085) the groom sends Travis a wishlist, and a guest
//! gives him a gift from it at Pila, only once; after the wedding he takes
//! it from Pila in Amoria.

use game_data::server_content::keys;
use game_data::GameData;
use integration_harness::database::{
    add_wedding_guest, book_wedding, couple_status, engage, grant_stack, replace_item, set_gender,
    set_job, wedding_due,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio::time::Duration;

const AMORIA: i32 = 680_000_000;
const HALL: i32 = 680_000_100;
const ALTAR: i32 = 680_000_110;
const EXIT: i32 = 680_000_500;
const WAYNE: i32 = 9201012;
const PRIEST: i32 = 9201011;
const JACKIE: i32 = 9201009;
const BONNIE: i32 = 9201008;
const NICOLE: i32 = 9201005;
const TRAVIS: i32 = 9201010;
const PILA: i32 = 9201014;
const RED_POTION: i32 = 2000000;
const USE: i16 = 2;
/// The invitation a Chapel guest receives.
const INVITATION: i32 = 4031406;
/// The assistants' menu choice for guests.
const AS_GUEST: i32 = 2;
const UNDERSHIRT: i32 = 1040002;
const MOONSTONE_BOX: i32 = 2240000;
/// The couple's halves of the engagement: the groom's empty box and the
/// bride's ring.
const EMPTY_BOX: i32 = 4031357;
const RING: i32 = 4031358;
const WEDDING_RING: i32 = 1112803;
const GROOM_CLOTHES: i32 = 1050113;
const BRIDE_CLOTHES: i32 = 1051114;
const ETC: i16 = 4;
const EQUIPPED: i16 = -1;
const TOP: i16 = -5;
const CHAPEL: i16 = 0;

const MARRIED: u8 = 12;
const CANCELLED: u8 = 31;
const STATUS_ENGAGED: i16 = 1;
const STATUS_MARRIED: i16 = 3;

/// One wedding at a time may use the Chapel on the harness's channel.
static CHAPEL_IN_USE: Mutex<()> = Mutex::const_new(());

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap_or_else(|| panic!("NPC {npc} on {map}"));
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

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

/// Read on from `t` until `session` has `count` priest packets.
async fn hear(session: &mut WorldSession, count: usize, mut t: Trace) -> Trace {
    until(&mut session.connection, &mut t, |t| {
        t.wedding.len() >= count
    })
    .await;
    read_for(&mut session.connection, &mut t, 300).await;
    t
}

/// The modes of the MARRIAGE_RESULT packets in `t`.
fn results(t: &Trace) -> Vec<u8> {
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
        .expect("wedding clothes");
    id
}

/// An engaged couple holding their tokens, with a Chapel wedding due now
/// on the harness's channel, both in Amoria; the couple's id.
async fn due_couple(configs: &[HarnessConfig; 2]) -> (i32, WorldSession, WorldSession) {
    let groom = new_character(&configs[0], 0, GROOM_CLOTHES).await;
    let bride = new_character(&configs[1], 1, BRIDE_CLOTHES).await;
    engage(groom, bride, MOONSTONE_BOX).await.expect("engage");
    grant_stack(groom, ETC, 1, EMPTY_BOX, 1).await.expect("box");
    grant_stack(bride, ETC, 1, RING, 1).await.expect("ring");
    let marriage_id = book_wedding(groom, CHAPEL, 0).await.expect("book");
    wedding_due(marriage_id, 0).await.expect("due");
    let a = enter(&configs[0], AMORIA).await;
    let b = enter(&configs[1], AMORIA).await;
    (marriage_id, a, b)
}

async fn enter(config: &HarnessConfig, map: i32) -> WorldSession {
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut session.connection, map, "", &mut t).await, map);
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

#[tokio::test]
async fn wayne_starts_a_due_wedding_and_the_priest_marries_the_couple() {
    let _chapel = CHAPEL_IN_USE.lock().await;
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let (marriage_id, mut a, mut b) = due_couple(&configs).await;

    // A starts it: both go to the hall, where the gathering's clock runs.
    let at = say_yes(&mut a, AMORIA, WAYNE).await;
    let at = arrive(&mut a, HALL, at).await;
    assert_eq!(at.countdowns.len(), 1, "{:?}", at.countdowns);
    arrive(&mut b, HALL, Trace::default()).await;

    // The gathering over, both go on to the altar with a new clock, where
    // the priest welcomes them.
    let (groom, bride) = (a.character_id, b.character_id);
    let at = arrive(&mut a, ALTAR, Trace::default()).await;
    assert_eq!(at.countdowns.len(), 1, "{:?}", at.countdowns);
    let at = hear(&mut a, 1, at).await;
    assert_eq!(at.wedding, [priest(0, groom, bride)]);
    let bt = arrive(&mut b, ALTAR, Trace::default()).await;
    let bt = hear(&mut b, 1, bt).await;
    assert_eq!(bt.wedding, [priest(0, groom, bride)]);

    // A's vows wait for B's; B's marry them.
    let at = say_yes(&mut a, ALTAR, PRIEST).await;
    assert_eq!(at.marriage, []);
    assert_eq!(
        couple_status(marriage_id).await.unwrap(),
        (STATUS_ENGAGED, true)
    );
    let mut bt = say_yes(&mut b, ALTAR, PRIEST).await;
    until(&mut b.connection, &mut bt, |t| !t.marriage.is_empty()).await;
    let bt = hear(&mut b, 1, bt).await;
    assert_eq!(results(&bt), [MARRIED]);
    // The priest pronounces them over the altar, not in a dialog of B's.
    assert_eq!(bt.wedding, [priest(1, groom, bride)]);
    assert!(bt.prompts.is_empty(), "{:?}", bt.prompt_texts);
    assert!(bt.items.contains(&(WEDDING_RING, 1)), "{:?}", bt.items);
    assert!(
        bt.removed.iter().any(|&(kind, _)| kind == ETC as u8),
        "{:?}",
        bt.removed
    );
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.marriage.is_empty()).await;
    let at = hear(&mut a, 1, at).await;
    assert_eq!(results(&at), [MARRIED]);
    assert_eq!(at.wedding, [priest(1, groom, bride)]);
    assert!(at.items.contains(&(WEDDING_RING, 1)), "{:?}", at.items);
    assert_eq!(
        couple_status(marriage_id).await.unwrap(),
        (STATUS_MARRIED, false)
    );

    // A closing the pronouncement brings the hearts and the priest's
    // farewell to both; a second close changes nothing.
    let m = a.character_name.clone();
    let at = step(&mut a.connection, &format!("{m} closes"), talked(1)).await;
    let at = hear(&mut a, 2, at).await;
    assert_eq!(at.wedding, [hearts(), priest(2, groom, bride)]);
    let bt = hear(&mut b, 2, Trace::default()).await;
    assert_eq!(bt.wedding, [hearts(), priest(2, groom, bride)]);
    let mut bt = step(&mut b.connection, "B closes", talked(1)).await;
    read_for(&mut b.connection, &mut bt, 500).await;
    assert_eq!(bt.wedding, []);

    // Jackie sees them out.
    let at = say_yes(&mut a, ALTAR, JACKIE).await;
    arrive(&mut a, EXIT, at).await;
    arrive(&mut b, EXIT, Trace::default()).await;
    // The Chapel is free again once its copy closes.
    tokio::time::sleep(Duration::from_secs(3)).await;
}

#[tokio::test]
async fn a_partner_leaving_before_the_vows_ends_the_wedding() {
    let _chapel = CHAPEL_IN_USE.lock().await;
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let (marriage_id, mut a, mut b) = due_couple(&configs).await;
    let at = say_yes(&mut a, AMORIA, WAYNE).await;
    arrive(&mut a, HALL, at).await;
    arrive(&mut b, HALL, Trace::default()).await;

    // B logs out: A goes to the exit, told the wedding is off, and the
    // booking is gone with it.
    drop(b);
    let mut at = arrive(&mut a, EXIT, Trace::default()).await;
    until(&mut a.connection, &mut at, |t| !t.marriage.is_empty()).await;
    assert_eq!(results(&at), [CANCELLED]);
    assert_eq!(
        couple_status(marriage_id).await.unwrap(),
        (STATUS_ENGAGED, false)
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
}

/// A character holding a Chapel invitation, on the guest list of the
/// couple `marriage_id` if any, in Amoria.
async fn guest(config: &HarnessConfig, marriage_id: Option<i32>) -> WorldSession {
    let id = new_character(config, 0, UNDERSHIRT).await;
    grant_stack(id, ETC, 1, INVITATION, 1)
        .await
        .expect("invitation");
    if let Some(marriage_id) = marriage_id {
        add_wedding_guest(marriage_id, id).await.expect("guest");
    }
    enter(config, AMORIA).await
}

/// Tell `npc` in Amoria the player is a guest; what they were asked or
/// told.
async fn as_guest(session: &mut WorldSession, npc: i32) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talks to {npc}"), talk(AMORIA, npc)).await;
    assert_eq!(t.prompts, [(npc, 4)], "{:?}", t.prompt_texts);
    send(c, answer(4, 1, Some(AS_GUEST))).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.prompts.is_empty()).await;
    read_for(c, &mut t, 300).await;
    t
}

/// What `npc` told the guest, who then closes the dialogue.
async fn told(session: &mut WorldSession, npc: i32, t: Trace) -> String {
    assert_eq!(t.prompts, [(npc, 0)], "{:?}", t.prompt_texts);
    let m = session.character_name.clone();
    step(
        &mut session.connection,
        &format!("{m} ok"),
        answer(0, 1, None),
    )
    .await;
    t.prompt_texts[0].clone()
}

/// Ask Bonnie to go in and say yes; what followed, once in the hall or
/// told why not.
async fn go_in(session: &mut WorldSession) -> Trace {
    let t = as_guest(session, BONNIE).await;
    assert_eq!(t.prompts, [(BONNIE, 1)], "{:?}", t.prompt_texts);
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let mut t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    until(c, &mut t, |t| {
        t.warps.contains(&HALL) || !t.prompts.is_empty()
    })
    .await;
    read_for(c, &mut t, 300).await;
    t
}

#[tokio::test]
async fn invited_guests_come_in_while_the_couple_gather() {
    let _chapel = CHAPEL_IN_USE.lock().await;
    let couple = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let mut configs = Vec::new();
    for _ in 0..5 {
        configs.push(load_harness_config_or_fail().await);
    }
    let (marriage_id, mut a, mut b) = due_couple(&couple).await;
    let mut stays = guest(&configs[0], Some(marriage_id)).await;
    let mut leaves = guest(&configs[1], Some(marriage_id)).await;
    let drops = guest(&configs[2], Some(marriage_id)).await;
    let mut late = guest(&configs[3], Some(marriage_id)).await;
    let mut uninvited = guest(&configs[4], None).await;
    let text = |key| net::content::text().get(key, &[]);

    // Off the list, or at the other venue, no one gets in.
    let t = as_guest(&mut uninvited, BONNIE).await;
    let said = told(&mut uninvited, BONNIE, t).await;
    assert_eq!(said, text(keys::WEDDING_GUEST_NOT_INVITED));
    let t = as_guest(&mut stays, NICOLE).await;
    let said = told(&mut stays, NICOLE, t).await;
    assert_eq!(said, text(keys::WEDDING_GUEST_NOT_INVITED));
    // Until the couple begin it, the guests wait.
    let t = go_in(&mut stays).await;
    let said = told(&mut stays, BONNIE, t).await;
    assert_eq!(said, text(keys::WEDDING_GUEST_WAIT));

    // While the couple gather, the guests come into their hall.
    let at = say_yes(&mut a, AMORIA, WAYNE).await;
    arrive(&mut a, HALL, at).await;
    arrive(&mut b, HALL, Trace::default()).await;
    let mut drops = drops;
    for guest in [&mut stays, &mut leaves, &mut drops] {
        let t = go_in(guest).await;
        assert!(t.warps.contains(&HALL), "{:?}", t.prompt_texts);
        assert_eq!(t.countdowns.len(), 1, "{:?}", t.countdowns);
    }
    // A guest logging out ends nothing.
    drop(drops);

    // The couple go on to the altar, and the guests with them, where all
    // hear the priest's welcome.
    let (groom, bride) = (a.character_id, b.character_id);
    for session in [&mut a, &mut b, &mut stays, &mut leaves] {
        let t = arrive(session, ALTAR, Trace::default()).await;
        let t = hear(session, 1, t).await;
        assert_eq!(t.wedding, [priest(0, groom, bride)]);
    }
    // Then the doors are closed.
    let t = go_in(&mut late).await;
    assert!(!t.warps.contains(&HALL), "{:?}", t.warps);
    let said = told(&mut late, BONNIE, t).await;
    assert_eq!(said, text(keys::WEDDING_GUEST_LATE));

    // The priest has no vows for a guest; Jackie sees one out, and the
    // wedding goes on.
    let m = leaves.character_name.clone();
    let c = &mut leaves.connection;
    let t = step(c, &format!("{m} talks to the priest"), talk(ALTAR, PRIEST)).await;
    assert_eq!(t.prompts, [(PRIEST, 0)], "{:?}", t.prompt_texts);
    assert_eq!(t.prompt_texts, [text(keys::WEDDING_PRIEST_INTRO)]);
    step(c, &format!("{m} ok"), answer(0, 1, None)).await;
    let t = say_yes(&mut leaves, ALTAR, JACKIE).await;
    let t = arrive(&mut leaves, EXIT, t).await;
    assert!(results(&t).is_empty());
    let mut at = Trace::default();
    read_for(&mut a.connection, &mut at, 500).await;
    assert!(at.warps.is_empty(), "{:?}", at.warps);

    // B logs out before the vows: the couple's booking is gone, and the
    // guest still in goes out with A, without the couple's notice.
    drop(b);
    let mut at = arrive(&mut a, EXIT, Trace::default()).await;
    until(&mut a.connection, &mut at, |t| !t.marriage.is_empty()).await;
    assert_eq!(results(&at), [CANCELLED]);
    let st = arrive(&mut stays, EXIT, Trace::default()).await;
    assert!(results(&st).is_empty());
    assert_eq!(
        couple_status(marriage_id).await.unwrap(),
        (STATUS_ENGAGED, false)
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
}

fn wishlist(wishes: &[&str]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(9).unwrap();
    p.write_short(wishes.len() as i16).unwrap();
    for wish in wishes {
        p.write_str_with_length(wish).unwrap();
    }
    p
}

/// WEDDING_ACTION `mode` with `body`.
fn wedding_action(mode: u8, body: &[u8]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::WeddingAction as i16).unwrap();
    p.write_byte(mode).unwrap();
    p.write_bytes(body).unwrap();
    p
}

/// The bodies of the WEDDING_GIFT_RESULT packets in `t`.
fn gift_results(t: &Trace) -> Vec<Vec<u8>> {
    t.marriage
        .iter()
        .filter(|(op, _)| *op == SendOpcode::WeddingGiftResult as i16)
        .map(|(_, body)| body.clone())
        .collect()
}

/// Read on from `t` until `session` has a WEDDING_GIFT_RESULT.
async fn gift_result(session: &mut WorldSession, mut t: Trace) -> Vec<u8> {
    until(&mut session.connection, &mut t, |t| {
        !gift_results(t).is_empty()
    })
    .await;
    let results = gift_results(&t);
    assert_eq!(results.len(), 1, "{results:?}");
    results[0].clone()
}

/// Pick `choice` from Pila's menu on `map`; what followed.
async fn pila(session: &mut WorldSession, map: i32, choice: i32) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talks to Pila"), talk(map, PILA)).await;
    assert_eq!(t.prompts, [(PILA, 4)], "{:?}", t.prompt_texts);
    step(
        c,
        &format!("{m} picks {choice}"),
        answer(4, 1, Some(choice)),
    )
    .await
}

#[tokio::test]
async fn a_guest_gives_from_the_wishlist_once_and_the_groom_takes_it() {
    let _chapel = CHAPEL_IN_USE.lock().await;
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let couple = [configs[0].clone(), configs[1].clone()];
    let (marriage_id, mut a, mut b) = due_couple(&couple).await;
    let id = new_character(&configs[2], 0, UNDERSHIRT).await;
    grant_stack(id, ETC, 1, INVITATION, 1)
        .await
        .expect("invitation");
    grant_stack(id, USE, 1, RED_POTION, 10)
        .await
        .expect("potions");
    add_wedding_guest(marriage_id, id).await.expect("guest");
    let mut g = enter(&configs[2], AMORIA).await;
    let text = |key| net::content::text().get(key, &[]);

    let at = say_yes(&mut a, AMORIA, WAYNE).await;
    arrive(&mut a, HALL, at).await;
    arrive(&mut b, HALL, Trace::default()).await;
    let t = go_in(&mut g).await;
    assert!(t.warps.contains(&HALL), "{:?}", t.prompt_texts);

    // Before the groom's wishlist, the guest's window opens on none.
    let t = pila(&mut g, HALL, 0).await;
    assert_eq!(gift_result(&mut g, t).await, [9, 0]);
    step(&mut g.connection, "G closes", wedding_action(8, &[])).await;

    // Travis opens the groom's wishlist window; what he sends is kept,
    // once.
    let at = say_yes(&mut a, HALL, TRAVIS).await;
    let mut at2 = at;
    until(&mut a.connection, &mut at2, |t| !t.marriage.is_empty()).await;
    assert_eq!(
        at2.marriage,
        [(SendOpcode::MarriageRequest as i16, vec![9])]
    );
    step(
        &mut a.connection,
        "A wishes",
        wishlist(&["Potions", "Cake"]),
    )
    .await;
    step(&mut a.connection, "A again", wishlist(&["Nothing"])).await;
    let m = a.character_name.clone();
    let t = step(
        &mut a.connection,
        &format!("{m} talks to Travis"),
        talk(HALL, TRAVIS),
    )
    .await;
    assert_eq!(t.prompt_texts, [text(keys::WEDDING_WISHLIST_SENT)]);
    step(&mut a.connection, "A ok", answer(0, 1, None)).await;

    // The guest gives four potions from the groom's wishlist.
    let t = pila(&mut g, HALL, 0).await;
    let mut wishes = vec![9, 2, 7, 0];
    wishes.extend(b"Potions");
    wishes.extend([4, 0]);
    wishes.extend(b"Cake");
    assert_eq!(gift_result(&mut g, t).await, wishes);
    let mut give = Vec::new();
    give.extend(1i16.to_le_bytes());
    give.extend(RED_POTION.to_le_bytes());
    give.extend(4i16.to_le_bytes());
    let t = step(&mut g.connection, "G gives", wedding_action(6, &give)).await;
    assert!(
        t.quantities.contains(&(USE as u8, 1, 6)),
        "{:?}",
        t.quantities
    );
    let sent = gift_result(&mut g, t).await;
    assert_eq!(sent[0], 11);
    assert_eq!(sent[1..wishes.len()], wishes[1..]);
    step(&mut g.connection, "G closes", wedding_action(8, &[])).await;

    // Once only; and not from one's own.
    let t = pila(&mut g, HALL, 0).await;
    assert_eq!(t.prompt_texts, [text(keys::WEDDING_PILA_GIVEN)]);
    step(&mut g.connection, "G ok", answer(0, 1, None)).await;
    let t = pila(&mut a, HALL, 0).await;
    assert_eq!(t.prompt_texts, [text(keys::WEDDING_PILA_OWN)]);
    step(&mut a.connection, "A ok", answer(0, 1, None)).await;

    // The bride leaves, which ends the wedding; back in Amoria the groom
    // takes his gift from Pila.
    drop(b);
    arrive(&mut a, EXIT, Trace::default()).await;
    arrive(&mut g, EXIT, Trace::default()).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut a.connection, AMORIA, "", &mut t).await, AMORIA);
    let t = say_yes(&mut a, AMORIA, PILA).await;
    let gifts = gift_result(&mut a, t).await;
    assert_eq!(gifts[0], 10);
    assert_eq!(gifts[1..9], 0x7Ci64.to_le_bytes());
    assert_eq!(gifts[9..11], [0, 1], "no equips, one potion");
    let mut take = wedding_action(7, &[9, 0]);
    let t = step(&mut a.connection, "A takes nothing", take).await;
    assert_eq!(gift_result(&mut a, t).await, [16]);
    take = wedding_action(7, &[USE as u8, 0]);
    let t = step(&mut a.connection, "A takes", take).await;
    let use_bag = |kind: &u8| *kind == USE as u8;
    assert!(
        t.added
            .iter()
            .any(|(kind, _, id)| use_bag(kind) && *id == RED_POTION)
            || t.quantities.iter().any(|(kind, _, _)| use_bag(kind)),
        "{t:?}"
    );
    let mut taken = vec![15];
    taken.extend(0x08i64.to_le_bytes());
    taken.push(0);
    assert_eq!(gift_result(&mut a, t).await, taken);
    step(&mut a.connection, "A closes", wedding_action(8, &[])).await;
    // Nothing waits any more.
    let m = a.character_name.clone();
    let t = step(
        &mut a.connection,
        &format!("{m} talks to Pila"),
        talk(AMORIA, PILA),
    )
    .await;
    assert_eq!(t.prompt_texts, [text(keys::WEDDING_PILA_HELLO)]);
    tokio::time::sleep(Duration::from_secs(3)).await;
}
