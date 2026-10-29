//! Wedding reservations (report b066): Nicole and Bonnie in Amoria book an
//! engaged couple's wedding for a ticket from the Cash inventory while both
//! partners stand beside them. The booker loses the ticket; each partner
//! gets 15 invitations and MARRIAGE_RESULT 16. A booked couple hears when
//! the wedding starts and may not break off (MARRIAGE_RESULT 32). The
//! assistant refuses without a ticket, with the partner away or offline,
//! and with a partner who has no room or a wedding ring already.

use game_data::GameData;
use integration_harness::database::{engage, grant_stack, set_gender, set_job, stow_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const AMORIA: i32 = 680_000_000;
const HENESYS: i32 = 100_000_000;
const NICOLE: i32 = 9201005;
const BONNIE: i32 = 9201008;
const MOONSTONE_BOX: i32 = 2240000;
/// The bride's engagement ring, a token she may drop to break it off.
const RING: i32 = 4031358;
const WEDDING_RING: i32 = 1112803;
const CATHEDRAL_TICKET: i32 = 5251000;
const CATHEDRAL_PREMIUM: i32 = 5251003;
const CHAPEL_TICKET: i32 = 5251001;
const CATHEDRAL_INVITATION: i32 = 4031395;
const ETC: i16 = 4;
const CASH: i16 = 5;
const ETC_SLOTS: i16 = 24;
const SNAIL_SHELL: i32 = 4000000;

const RESERVED: u8 = 16;
const NO_BREAKING_OFF: u8 = 32;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(npc: i32) -> Packet {
    let index = maps()
        .field(AMORIA)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap_or_else(|| panic!("NPC {npc} in Amoria"));
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

fn break_off(token: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(3).unwrap();
    p.write_int(token).unwrap();
    p
}

fn result(mode: u8) -> (i16, Vec<u8>) {
    (SendOpcode::MarriageResult as i16, vec![mode])
}

/// Make the character behind `config` of `gender` at level 50 and log it
/// out; its id.
async fn new_character(config: &HarnessConfig, gender: i16) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.expect("level");
    set_gender(id, gender).await.expect("gender");
    id
}

async fn enter(config: &HarnessConfig, map: i32) -> WorldSession {
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut session.connection, map, "", &mut t).await, map);
    session
}

/// Ask `npc` to book the wedding; what the player was told, once the
/// assistant has answered.
async fn reserve(session: &mut WorldSession, npc: i32) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} assistant"), talk(npc)).await;
    assert_eq!(t.prompts, [(npc, 4)]);
    send(c, answer(4, 1, Some(1))).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.prompts.is_empty()).await;
    read_for(c, &mut t, 300).await;
    assert_eq!(t.prompts, [(npc, 0)]);
    step(c, &format!("{m} ok"), answer(0, 1, None)).await;
    t
}

/// As `reserve`, for a refusal, which takes and books nothing; its text.
async fn refused(session: &mut WorldSession, npc: i32) -> String {
    let t = reserve(session, npc).await;
    assert_eq!(t.items, []);
    assert_eq!(t.marriage, []);
    t.prompt_texts[0].clone()
}

#[tokio::test]
async fn nicole_books_an_engaged_couple_for_a_ticket() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let [a_config, b_config] = &configs;
    let a_id = new_character(a_config, 0).await;
    let b_id = new_character(b_config, 1).await;
    engage(a_id, b_id, MOONSTONE_BOX).await.expect("engage");
    grant_stack(a_id, CASH, 1, CATHEDRAL_TICKET, 1)
        .await
        .expect("ticket");
    grant_stack(b_id, ETC, 1, RING, 1).await.expect("ring");
    let mut a = enter(a_config, AMORIA).await;
    let mut b = enter(b_config, AMORIA).await;

    // A books: the ticket goes, and both get invitations and the notice.
    let at = reserve(&mut a, NICOLE).await;
    assert_eq!(
        at.items,
        [(CATHEDRAL_TICKET, -1), (CATHEDRAL_INVITATION, 15)]
    );
    assert_eq!(at.marriage, [result(RESERVED)]);
    // When, and a reminder of the wedding clothes.
    assert_eq!(at.broadcasts.len(), 2, "{:?}", at.broadcasts);
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| !t.marriage.is_empty()).await;
    read_for(&mut b.connection, &mut bt, 300).await;
    assert_eq!(bt.items, [(CATHEDRAL_INVITATION, 15)]);
    assert_eq!(bt.marriage, [result(RESERVED)]);
    assert_eq!(bt.broadcasts.len(), 2, "{:?}", bt.broadcasts);

    // Booked, the couple hears when, on this login and the next, and may
    // not break off.
    let booked = format!("#t{CATHEDRAL_TICKET}#");
    let t = reserve(&mut b, NICOLE).await;
    assert_eq!(t.items, []);
    assert!(t.prompt_texts[0].contains(&booked), "{:?}", t.prompt_texts);
    assert!(!t.prompt_texts[0].contains(&format!("#t{CATHEDRAL_PREMIUM}#")));
    drop(a);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut a = enter(a_config, AMORIA).await;
    let t = reserve(&mut a, NICOLE).await;
    assert_eq!(t.items, []);
    assert!(t.prompt_texts[0].contains(&booked), "{:?}", t.prompt_texts);

    send(&mut b.connection, break_off(RING)).await;
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| !t.marriage.is_empty()).await;
    read_for(&mut b.connection, &mut bt, 300).await;
    assert_eq!(bt.marriage, [result(NO_BREAKING_OFF)]);
    assert_eq!(bt.removed, []);
}

#[tokio::test]
async fn the_assistants_refuse_a_couple_not_ready() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let [a_config, b_config] = &configs;
    let a_id = new_character(a_config, 0).await;
    let b_id = new_character(b_config, 1).await;
    engage(a_id, b_id, MOONSTONE_BOX).await.expect("engage");
    grant_stack(a_id, CASH, 1, CHAPEL_TICKET, 1)
        .await
        .expect("ticket");
    for slot in 1..=ETC_SLOTS {
        grant_stack(b_id, ETC, slot, SNAIL_SHELL, 1)
            .await
            .expect("a full Etc tab");
    }
    let mut a = enter(a_config, AMORIA).await;

    // A Chapel ticket books no Cathedral wedding.
    let no_ticket = refused(&mut a, NICOLE).await;
    assert!(no_ticket.contains(&format!("#t{CATHEDRAL_PREMIUM}#")));
    // B is offline, then elsewhere, then has no room for the invitations.
    let offline = refused(&mut a, BONNIE).await;
    let b = enter(b_config, HENESYS).await;
    let elsewhere = refused(&mut a, BONNIE).await;
    drop(b);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let b = enter(b_config, AMORIA).await;
    let no_room = refused(&mut a, BONNIE).await;
    drop(b);
    // Then B has a wedding ring already.
    tokio::time::sleep(Duration::from_millis(500)).await;
    stow_equip(b_id, 1, WEDDING_RING, 0)
        .await
        .expect("B's ring");
    let b = enter(b_config, AMORIA).await;
    let has_ring = refused(&mut a, BONNIE).await;
    drop(b);

    let texts = [no_ticket, offline, elsewhere, no_room, has_ring];
    for (i, text) in texts.iter().enumerate() {
        for other in &texts[i + 1..] {
            assert_ne!(text, other, "each refusal says why");
        }
    }
}
