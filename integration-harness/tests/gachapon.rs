//! The Henesys Gachapon (9100100): a Gachapon Ticket buys the prize the
//! machine rolled, and the world hears of an uncommon one. The harness pins
//! every roll to the same uncommon prize, a Scroll for Bow for ATT 10%
//! (RUSTMS_GACHAPON_SEED in docker-compose.test.yml). A full bag keeps the
//! ticket before any roll, and without one the machine only explains itself.

use integration_harness::database::grant_item;
use integration_harness::events::talk;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use std::time::Duration;

const HENESYS: i32 = 100_000_100;
const MACHINE: i32 = 9_100_100;
const TICKET: i32 = 5_220_000;
/// What seed 10 wins.
const PRIZE: i32 = 2_044_502;
const USE: i16 = 2;
const CASH: i16 = 5;
const SAY: u8 = 0;
const YES_NO: u8 = 1;
const MENU: u8 = 4;

/// A new character, holding a ticket if `ticket`, logged in again in
/// Henesys after `prepare` has changed it in the database.
async fn player<F>(
    ticket: bool,
    prepare: impl FnOnce(i32) -> F,
) -> (WorldSession, Trace, HarnessConfig)
where
    F: std::future::Future<Output = ()>,
{
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    if ticket {
        grant_item(id, CASH, 1, TICKET).await.expect("ticket");
    }
    prepare(id).await;
    let mut session = relogin(&config).await;
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut session.connection, HENESYS, "", &mut t).await,
        HENESYS
    );
    (session, t, config)
}

/// The type of the next NPC prompt, and its text.
async fn prompt(session: &mut WorldSession, t: &mut Trace) -> (u8, String) {
    let seen = t.prompts.len();
    until(&mut session.connection, t, |t| t.prompts.len() > seen).await;
    assert_eq!(t.prompts[seen].0, MACHINE);
    (t.prompts[seen].1, t.prompt_texts[seen].clone())
}

#[tokio::test]
async fn a_ticket_buys_the_prize_and_the_world_hears_of_it() {
    let (mut session, mut t, config) = player(true, |_| async {}).await;
    let mut listener = login_to_world_session(&load_harness_config_or_fail().await)
        .await
        .unwrap();
    let mut heard = Trace::default();
    read_for(&mut listener.connection, &mut heard, 300).await;

    send(&mut session.connection, talk(HENESYS, MACHINE)).await;
    assert_eq!(prompt(&mut session, &mut t).await.0, YES_NO);
    send(&mut session.connection, answer(YES_NO, 1, None)).await;
    let (kind, text) = prompt(&mut session, &mut t).await;
    assert_eq!(kind, SAY);
    assert!(text.contains(&format!("#t{PRIZE}#")), "{text}");
    assert_eq!(t.removed, [(CASH as u8, 1)]);
    assert!(
        t.added
            .iter()
            .any(|&(kind, _, item)| (kind, item) == (USE as u8, PRIZE)),
        "{:?}",
        t.added
    );

    until(&mut listener.connection, &mut heard, |t| {
        !t.gachapon_wins.is_empty()
    })
    .await;
    let (message, _, town, item) = &heard.gachapon_wins[0];
    assert_eq!(*message, format!("{} : got a(n)", session.character_name));
    assert_eq!((town.as_str(), *item), ("Henesys", PRIZE));
    drop(listener);
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    let back = relogin(&config).await;
    assert_eq!(back.items.get(&PRIZE), Some(&1));
    assert_eq!(back.items.get(&TICKET), None);
}

#[tokio::test]
async fn a_full_bag_keeps_the_ticket_unrolled() {
    let (mut session, mut t, config) = player(true, |id| async move {
        for slot in 1..=24 {
            // Red Potions, which the prize can't stack with.
            grant_item(id, USE, slot, 2_000_000).await.expect("potion");
        }
    })
    .await;
    // Before any roll, so a full bag never turns a prize away.
    send(&mut session.connection, talk(HENESYS, MACHINE)).await;
    assert_eq!(prompt(&mut session, &mut t).await.0, SAY);
    read_for(&mut session.connection, &mut t, 300).await;
    assert!(t.removed.is_empty(), "{:?}", t.removed);
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    let back = relogin(&config).await;
    assert_eq!(back.items.get(&TICKET), Some(&1));
    assert_eq!(back.items.get(&PRIZE), None);
}

#[tokio::test]
async fn without_a_ticket_the_machine_explains_itself() {
    let (mut session, mut t, _) = player(false, |_| async {}).await;
    send(&mut session.connection, talk(HENESYS, MACHINE)).await;
    assert_eq!(prompt(&mut session, &mut t).await.0, MENU);
    send(&mut session.connection, answer(MENU, 1, Some(0))).await;
    assert_eq!(prompt(&mut session, &mut t).await.0, SAY);
}
