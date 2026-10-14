//! Monster Book: card drops go into the book (five copies at most), show
//! the card effect to the map, and persist with the cover.
//!
//! The harness world runs with `RUSTMS_CARD_DROP_RATE=1`, so every kill of
//! a mob with a card drops it.

use integration_harness::hunting::{attack, choose_cover, pickup};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const SNAIL_CARD: i32 = 2380000;
const BLUE_SNAIL_CARD: i32 = 2380001;
/// B opens the Monster Book (HeavenClient's menu action 59).
const MONSTER_BOOK_KEY: usize = 48;

#[tokio::test]
async fn snail_cards_fill_the_book_and_persist_with_the_cover() {
    let hunter_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut hunter = login_to_world_session(&hunter_config).await.unwrap();
    assert_eq!(hunter.keymap[MONSTER_BOOK_KEY], (4, 59));
    assert!(hunter.monster_book.is_empty());
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (mut h, mut w) = (Trace::default(), Trace::default());
    assert_eq!(
        warp(&mut hunter.connection, FOREST, "", &mut h).await,
        FOREST
    );
    until(&mut hunter.connection, &mut h, |t| {
        t.field.alive(SNAIL).len() >= 6
    })
    .await;
    assert_eq!(
        warp(&mut watcher.connection, FOREST, "", &mut w).await,
        FOREST
    );
    read_for(&mut watcher.connection, &mut w, 300).await;
    read_for(&mut hunter.connection, &mut h, 300).await;

    // Six snails, six cards: five fill the page, the sixth is wasted.
    for (kill, oid) in h.field.alive(SNAIL).into_iter().take(6).enumerate() {
        let seen = h.field.drops.len();
        send(&mut hunter.connection, attack(oid, 8)).await;
        until(&mut hunter.connection, &mut h, |t| {
            t.field.drops[seen..]
                .iter()
                .any(|d| !d.meso && d.value == SNAIL_CARD)
        })
        .await;
        let card = *h.field.drops[seen..]
            .iter()
            .find(|d| !d.meso && d.value == SNAIL_CARD)
            .unwrap();
        send(
            &mut hunter.connection,
            pickup(card.object_id, card.x, card.y),
        )
        .await;
        until(&mut hunter.connection, &mut h, |t| t.cards.len() > kill).await;
        assert_eq!(
            h.field.removals(card.object_id),
            [Some(hunter.character_id)]
        );
    }
    read_for(&mut hunter.connection, &mut h, 300).await;
    assert_eq!(
        h.cards,
        [
            (true, SNAIL_CARD, 1),
            (true, SNAIL_CARD, 2),
            (true, SNAIL_CARD, 3),
            (true, SNAIL_CARD, 4),
            (true, SNAIL_CARD, 5),
            (false, SNAIL_CARD, 5),
        ]
    );
    // The card effect plays for each card that went in, and others see it.
    assert_eq!(h.card_effects, 5);
    until(&mut watcher.connection, &mut w, |t| {
        t.field
            .effects
            .iter()
            .filter(|&&e| e == (hunter.character_id, 13))
            .count()
            == 5
    })
    .await;
    // Cards never enter the bags.
    assert!(h.added.iter().all(|&(_, _, item)| item != SNAIL_CARD));

    // Only a collected card can go on the cover.
    send(&mut hunter.connection, choose_cover(SNAIL_CARD)).await;
    send(&mut hunter.connection, choose_cover(BLUE_SNAIL_CARD)).await;
    until(&mut hunter.connection, &mut h, |t| t.covers.len() == 2).await;
    assert_eq!(h.covers, [SNAIL_CARD, SNAIL_CARD]);
    drop(watcher);
    drop(hunter);

    let restored = relogin(&hunter_config).await;
    assert_eq!(restored.monster_book_cover, SNAIL_CARD);
    assert_eq!(
        restored.monster_book.into_iter().collect::<Vec<_>>(),
        [(SNAIL_CARD, 5)]
    );
}
