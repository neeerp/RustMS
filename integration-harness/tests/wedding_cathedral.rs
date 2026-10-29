//! The Cathedral wedding (report b075, slice 7d): the High Priest in
//! Amoria starts an engaged couple's booked wedding, and at the altar he
//! welcomes everyone, calls for blessings once the couple close his
//! welcome, and then asks the guests to bless the couple. Each guest's
//! blessing counts once, and only until the couple marry; the couple and
//! the guests' own closes don't move him on. The vows bring his
//! pronouncement, its close the hearts and his farewell, and Nancy sees
//! the couple out.
//!
//! The vows bring the couple 30000 EXP for each guest who blessed them and
//! stayed (slice 7e), and their guild reads that they married. A premium
//! wedding's couple go on with everyone to the party room, where Travis
//! and Robin show the way out, and Robin the way to the hunting ground and,
//! for seven Heart Keys, the box room (slice 7f). Leaving takes the keys.

use game_data::server_content::keys;
use integration_harness::database::{
    add_wedding_guest, book_wedding, engage, found_guild, grant_stack, join_guild, replace_item,
    set_gender, set_job, wedding_due, wedding_premium,
};
use integration_harness::events::talk;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::sync::Mutex;
use tokio::time::Duration;

const AMORIA: i32 = 680_000_000;
const HALL: i32 = 680_000_200;
const ALTAR: i32 = 680_000_210;
const EXIT: i32 = 680_000_500;
const PARTY_ROOM: i32 = 680_000_300;
const HUNTING_GROUND: i32 = 680_000_400;
const BOX_ROOM: i32 = 680_000_401;
const HIGH_PRIEST: i32 = 9201002;
const NANCY: i32 = 9201007;
const TRAVIS: i32 = 9201010;
const ROBIN: i32 = 9201021;
const NICOLE: i32 = 9201005;

/// One wedding at a time may use the Cathedral on the harness's channel.
static CATHEDRAL_IN_USE: Mutex<()> = Mutex::const_new(());

/// The invitation a Cathedral guest receives.
const INVITATION: i32 = 4031407;
/// The Amoria Heart Key, seven of which Robin takes for the box room.
const HEART_KEY: i32 = 4031409;
/// The hunting ground's Cake Monster.
const CAKE: i32 = 9400507;
/// Its Giant Cake, one to each wedding.
const GIANT_CAKE: i32 = 9400606;
/// A box of the box room.
const BOX: i32 = 6802000;
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
/// The EXP for each guest's blessing.
const BLESSING_EXP: i32 = 30000;

/// NOTIFY_MARRIAGE to `name`'s guild.
fn guild_married(name: &str) -> (i16, u8, i32, String) {
    (SendOpcode::NotifyMarriage as i16, 0, 0, format!("> {name}"))
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

/// Pick `choice` from Robin's menu on `map`; what followed.
async fn robin(session: &mut WorldSession, map: i32, choice: i32) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talks to Robin"), talk(map, ROBIN)).await;
    assert_eq!(t.prompts, [(ROBIN, 4)], "{:?}", t.prompt_texts);
    step(
        c,
        &format!("{m} picks {choice}"),
        answer(4, 1, Some(choice)),
    )
    .await
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

/// An engaged couple behind `configs[0..2]` with their Cathedral wedding
/// due, and the invited guests behind the rest of `configs`: the groom's
/// and the bride's ids, the couple's, and the guests' ids.
async fn book(configs: &[HarnessConfig], premium: bool) -> (i32, i32, i32, Vec<i32>) {
    let groom = new_character(&configs[0], 0, GROOM_CLOTHES).await;
    let bride = new_character(&configs[1], 1, BRIDE_CLOTHES).await;
    engage(groom, bride, MOONSTONE_BOX).await.expect("engage");
    grant_stack(groom, ETC, 1, EMPTY_BOX, 1).await.expect("box");
    grant_stack(bride, ETC, 1, RING, 1).await.expect("ring");
    let marriage_id = book_wedding(groom, CATHEDRAL, 1).await.expect("book");
    if premium {
        wedding_premium(marriage_id).await.expect("premium");
    }
    wedding_due(marriage_id, 0).await.expect("due");
    let mut guests = Vec::new();
    for config in &configs[2..] {
        let id = new_character(config, 0, UNDERSHIRT).await;
        grant_stack(id, ETC, 1, INVITATION, 1)
            .await
            .expect("invitation");
        add_wedding_guest(marriage_id, id).await.expect("guest");
        guests.push(id);
    }
    (groom, bride, marriage_id, guests)
}

/// Both of the couple take their vows; what each then heard, the
/// marriage result included.
async fn vow(a: &mut WorldSession, b: &mut WorldSession) -> (Trace, Trace) {
    let at = say_yes(a, ALTAR, HIGH_PRIEST).await;
    assert_eq!(at.marriage, []);
    let mut bt = say_yes(b, ALTAR, HIGH_PRIEST).await;
    until(&mut b.connection, &mut bt, |t| !t.marriage.is_empty()).await;
    let bt = hear(b, 1, bt).await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.marriage.is_empty()).await;
    let at = hear(a, 1, at).await;
    (at, bt)
}

#[tokio::test]
async fn the_high_priest_marries_a_couple_their_guests_bless() {
    let _cathedral = CATHEDRAL_IN_USE.lock().await;
    let mut configs = Vec::new();
    for _ in 0..5 {
        configs.push(load_harness_config_or_fail().await);
    }
    let (groom, bride, _, guest_ids) = book(&configs, false).await;
    // The groom and the late guest are in a guild; the bride isn't.
    let guild = format!("Wed{groom}");
    found_guild(groom, &guild).await.expect("guild");
    join_guild(guest_ids[1], &guild).await.expect("join");
    let mut guests = Vec::new();
    for config in &configs[2..] {
        guests.push(enter(config).await);
    }
    let mut leaver = guests.pop().unwrap();
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
    go_in(&mut leaver).await;

    // At the altar everyone hears his welcome.
    let mut everyone = [&mut a, &mut b, &mut blesser, &mut late, &mut leaver];
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
    let t = hear(&mut leaver, 2, Trace::default()).await;
    assert_eq!(
        t.wedding,
        [priest(1, groom, bride), priest(2, groom, bride)]
    );

    // The guest's blessing counts once; the couple have none to give.
    let blessed = net::content::text().get(keys::WEDDING_BLESSED, &[]);
    let t = quietly(&mut blesser, "blesses", bless()).await;
    assert_eq!(t.broadcasts, [(PINK, blessed.clone())]);
    let t = quietly(&mut blesser, "blesses again", bless()).await;
    assert_eq!(t.broadcasts, []);
    let t = quietly(&mut a, "blesses", bless()).await;
    assert_eq!(t.broadcasts, []);

    // Another guest blesses and leaves: their blessing goes with them.
    let t = quietly(&mut leaver, "blesses", bless()).await;
    assert_eq!(t.broadcasts, [(PINK, blessed)]);
    let t = say_yes(&mut leaver, ALTAR, NANCY).await;
    arrive(&mut leaver, EXIT, t).await;

    // The vows marry them and bring the pronouncement, and each of the
    // couple the EXP of the one blessing left; after that a blessing is
    // too late.
    let (at, bt) = vow(&mut a, &mut b).await;
    assert_eq!(marriage_results(&bt), [MARRIED]);
    assert_eq!(bt.wedding, [priest(3, groom, bride)]);
    assert_eq!(bt.gains, [BLESSING_EXP]);
    assert_eq!(at.gains, [BLESSING_EXP]);
    let mut lt = hear(&mut late, 1, Trace::default()).await;
    assert_eq!(lt.wedding, [priest(3, groom, bride)]);
    let t = quietly(&mut late, "blesses late", bless()).await;
    assert_eq!(t.broadcasts, []);

    // The groom's guild reads that he married, and so does he; the bride,
    // in no guild or family, hears nothing.
    let notice = guild_married(&a.character_name);
    until(&mut late.connection, &mut lt, |t| !t.notices.is_empty()).await;
    assert_eq!(lt.notices, [notice.clone()]);
    let mut at = at;
    until(&mut a.connection, &mut at, |t| !t.notices.is_empty()).await;
    assert_eq!(at.notices, [notice]);
    assert_eq!(bt.notices, []);

    // A's close of it brings the hearts and the farewell.
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

#[tokio::test]
async fn a_premium_wedding_goes_on_to_the_party_room() {
    let _cathedral = CATHEDRAL_IN_USE.lock().await;
    let mut configs = Vec::new();
    for _ in 0..3 {
        configs.push(load_harness_config_or_fail().await);
    }
    let (.., guests) = book(&configs, true).await;
    grant_stack(guests[0], ETC, 2, HEART_KEY, 9)
        .await
        .expect("keys");
    let mut guest = enter(&configs[2]).await;
    let mut a = enter(&configs[0]).await;
    let mut b = enter(&configs[1]).await;
    let at = say_yes(&mut a, AMORIA, HIGH_PRIEST).await;
    arrive(&mut a, HALL, at).await;
    arrive(&mut b, HALL, Trace::default()).await;
    go_in(&mut guest).await;
    for session in [&mut a, &mut b, &mut guest] {
        arrive(session, ALTAR, Trace::default()).await;
    }

    // Married without a blessing, they gain no EXP; Nancy takes everyone
    // on to the party room.
    let (at, bt) = vow(&mut a, &mut b).await;
    assert_eq!(marriage_results(&bt), [MARRIED]);
    assert_eq!((at.gains.len(), bt.gains.len()), (0, 0));
    let at = say_yes(&mut a, ALTAR, NANCY).await;
    arrive(&mut a, PARTY_ROOM, at).await;
    arrive(&mut b, PARTY_ROOM, Trace::default()).await;
    arrive(&mut guest, PARTY_ROOM, Trace::default()).await;

    // Robin takes the guest hunting among the cakes, and for seven keys on
    // to the boxes.
    let t = robin(&mut guest, PARTY_ROOM, 0).await;
    let t = arrive(&mut guest, HUNTING_GROUND, t).await;
    let cakes = t.field.mobs.iter().filter(|&&(_, mob)| mob == CAKE);
    assert!(cakes.count() > 0, "{:?}", t.field.mobs);
    // The wedding's copy opened with its one Giant Cake.
    let giant = t.field.mobs.iter().filter(|&&(_, mob)| mob == GIANT_CAKE);
    assert_eq!(giant.count(), 1, "{:?}", t.field.mobs);
    let t = robin(&mut guest, HUNTING_GROUND, 0).await;
    let t = arrive(&mut guest, BOX_ROOM, t).await;
    assert_eq!(t.items, [(HEART_KEY, -7)]);
    let boxes = t.field.reactors.iter().filter(|r| r.reactor_id == BOX);
    assert!(boxes.count() > 0, "{:?}", t.field.reactors);
    let t = say_yes(&mut guest, BOX_ROOM, ROBIN).await;
    let t = arrive(&mut guest, HUNTING_GROUND, t).await;
    // It is still the one: coming back brings no second.
    let giant = t.field.mobs.iter().filter(|&&(_, mob)| mob == GIANT_CAKE);
    assert_eq!(giant.count(), 1, "{:?}", t.field.mobs);
    // Two keys open nothing; leaving takes them.
    let t = robin(&mut guest, HUNTING_GROUND, 0).await;
    assert_eq!(t.prompts, [(ROBIN, 0)], "{:?}", t.prompt_texts);
    assert!(t.warps.is_empty() && t.items.is_empty());
    let t = robin(&mut guest, HUNTING_GROUND, 1).await;
    assert_eq!(t.prompts, [(ROBIN, 1)], "{:?}", t.prompt_texts);
    let m = guest.character_name.clone();
    let t = step(
        &mut guest.connection,
        &format!("{m} yes"),
        answer(1, 1, None),
    )
    .await;
    let t = arrive(&mut guest, EXIT, t).await;
    assert_eq!(t.removed, [(ETC as u8, 2)]);

    // Travis shows the groom back to Amoria; the bride stays until she too
    // goes.

    let at = say_yes(&mut a, PARTY_ROOM, TRAVIS).await;
    arrive(&mut a, AMORIA, at).await;
    let mut t = Trace::default();
    read_for(&mut b.connection, &mut t, 500).await;
    assert!(t.warps.is_empty(), "{:?}", t.warps);
    let bt = robin(&mut b, PARTY_ROOM, 1).await;
    assert_eq!(bt.prompts, [(ROBIN, 1)], "{:?}", bt.prompt_texts);
    let m = b.character_name.clone();
    let bt = step(&mut b.connection, &format!("{m} yes"), answer(1, 1, None)).await;
    arrive(&mut b, EXIT, bt).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
}
