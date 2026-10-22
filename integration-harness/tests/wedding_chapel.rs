//! The Chapel wedding (report b071, slice 7a): Wayne in Amoria starts an
//! engaged couple's booked wedding when it is due and both stand beside
//! him, dressed for it. The couple gather in their own copy of the hall,
//! then go on to the altar, where the priest takes each one's vows; the
//! second marries them: the tokens go, each gets the wedding ring and
//! MARRIAGE_RESULT 12, and the booking is gone. Jackie then sees them out.
//! A partner who leaves before the vows ends the wedding and its booking
//! (MARRIAGE_RESULT 31).

use game_data::GameData;
use integration_harness::database::{
    book_wedding, couple_status, engage, grant_stack, replace_item, set_gender, set_job,
    wedding_due,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, step, until, warp, Trace};
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

    // The gathering over, both go on to the altar with a new clock.
    let at = arrive(&mut a, ALTAR, Trace::default()).await;
    assert_eq!(at.countdowns.len(), 1, "{:?}", at.countdowns);
    arrive(&mut b, ALTAR, Trace::default()).await;

    // A's vows wait for B's; B's marry them.
    let at = say_yes(&mut a, ALTAR, PRIEST).await;
    assert_eq!(at.marriage, []);
    assert_eq!(
        couple_status(marriage_id).await.unwrap(),
        (STATUS_ENGAGED, true)
    );
    let mut bt = say_yes(&mut b, ALTAR, PRIEST).await;
    until(&mut b.connection, &mut bt, |t| !t.marriage.is_empty()).await;
    read_for(&mut b.connection, &mut bt, 300).await;
    assert_eq!(results(&bt), [MARRIED]);
    assert!(bt.items.contains(&(WEDDING_RING, 1)), "{:?}", bt.items);
    assert!(
        bt.removed.iter().any(|&(kind, _)| kind == ETC as u8),
        "{:?}",
        bt.removed
    );
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.marriage.is_empty()).await;
    read_for(&mut a.connection, &mut at, 300).await;
    assert_eq!(results(&at), [MARRIED]);
    assert!(at.items.contains(&(WEDDING_RING, 1)), "{:?}", at.items);
    assert_eq!(
        couple_status(marriage_id).await.unwrap(),
        (STATUS_MARRIED, false)
    );

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
