//! Portal scripts of a032's first slice: ways out of pockets players were
//! stuck in (the Dangerous Forest, the Bottom of the Temple, Fox Ridge),
//! ways that just let the player through (Ludibrium's Time Gate into Ellin
//! Forest, Leafre's and Aqua Road's boss areas), doors that keep players
//! back with a pink line until a quest opens them, and Ereve's forests,
//! where the Master of Disguise's hideouts take one searcher at a time.
//! Then a032's second: the ways an item opens (a Magic Seed between
//! Ellinia and Leafre, a Warp Card between the Nautilus and Omega Sector),
//! Ariant's palace, which turns out anyone not in Tigun's disguise, and
//! the original Maple Island's instruction balloons.

use integration_harness::database::{grant_stack, set_quest};
use integration_harness::hunting::use_item;
use integration_harness::preconditions::{
    load_harness_config_or_fail, load_multi_harness_config_or_fail,
};
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: walk into, or press Up on, the scripted portal
/// `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Go to `map` and try its portal `portal`; returns what came back.
async fn try_portal(player: &mut WorldSession, map: i32, portal: &str) -> Trace {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    step(&mut player.connection, &name, enter(portal)).await
}

/// Where the portal led, after its sound.
fn arrived(trace: &Trace) -> (i32, u8) {
    assert_eq!(trace.portal_sounds, 1, "{trace:?}");
    let [(map, portal, _)] = trace.arrivals[..] else {
        panic!("{:?}", trace.arrivals);
    };
    (map, portal)
}

fn pink(trace: &Trace) -> bool {
    matches!(trace.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty())
}

#[tokio::test]
async fn ways_through_open_and_quest_doors_stay_shut_to_a_new_character() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();

    for (map, portal, to) in [
        // Out of the Dangerous Forest, both ways.
        (140010000, "east00", 140000000),
        (140010000, "west00", 140010100),
        // Up from the Bottom of the Temple.
        (105100100, "top001", 105100000),
        (105100100, "top002", 105100000),
        // Fox Ridge back to Tiger Ridge.
        (222010300, "west00", 222010200),
        (240010500, "in00", 240010501),
        (240020100, "boss00", 240020101),
        (240020400, "boss00", 240020401),
        (230040410, "boss00", 230040420),
        (240040700, "out00", 240040600),
        (200010300, "eliza", 920020000),
        (250010504, "in00", 925000000),
        (104000000, "in03", 104000004),
        // Ereve's forests lead on to the next.
        (130010000, "in00", 130010010),
        (130010200, "west00", 130020000),
    ] {
        let trace = try_portal(&mut player, map, portal).await;
        assert_eq!(arrived(&trace).0, to, "{map} {portal}");
        assert!(trace.broadcasts.is_empty(), "{map}: {:?}", trace.broadcasts);
    }

    // Two ways say something on the way through.
    for (map, portal, to) in [
        (222020400, "in01", 300000100),
        (105100100, "OutPerrion", 105100000),
    ] {
        let trace = try_portal(&mut player, map, portal).await;
        assert_eq!(arrived(&trace).0, to, "{map} {portal}");
        assert!(pink(&trace), "{map}: {:?}", trace.broadcasts);
    }

    for (map, portal) in [
        (140010100, "in00"),
        (140010200, "in00"),
        (240040610, "in00"),
        (106021000, "in00"),
        (105100100, "in00"),
        (100030000, "quest00"),
        (105040201, "in00"),
        (260000200, "hiden00"),
        (220070400, "pt00"),
        (105090700, "in00"),
    ] {
        let trace = try_portal(&mut player, map, portal).await;
        assert!(trace.warps.is_empty(), "{map}: {:?}", trace.warps);
        assert!(pink(&trace), "{map}: {:?}", trace.broadcasts);
    }

    // Into Nett's Pyramid's lobby and back out to where the player was.
    let trace = try_portal(&mut player, 260020500, "piramid00").await;
    assert_eq!(arrived(&trace), (926010000, 4));
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, enter("out00")).await;
    assert_eq!(arrived(&trace), (260020500, 3));

    // The queen's treasure room says nothing to a player not after it.
    let trace = try_portal(&mut player, 260000302, "hiden0").await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(trace.broadcasts.is_empty(), "{:?}", trace.broadcasts);
}

#[tokio::test]
async fn the_master_of_disguises_forest_takes_one_searcher_at_a_time() {
    const TRAINING_FOREST: i32 = 130010000;
    const TINOS_FOREST: i32 = 108010600;
    let config = load_multi_harness_config_or_fail().await;
    let (searcher_config, other_config) = (
        config.player("sender").unwrap(),
        config.player("recipient").unwrap(),
    );
    let searcher = login_to_world_session(&searcher_config).await.unwrap();
    let id = searcher.character_id;
    drop(searcher);
    drop(relogin(&searcher_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Looking for the Master of Disguise.
    set_quest(id, 20301, 1, "").await.unwrap();
    let mut searcher = relogin(&searcher_config).await;
    let mut other = login_to_world_session(&other_config).await.unwrap();

    // Someone is in Tino's forest already.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut other.connection, TINOS_FOREST, "", &mut trace).await,
        TINOS_FOREST
    );
    let trace = try_portal(&mut searcher, TRAINING_FOREST, "in00").await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(pink(&trace), "{:?}", trace.broadcasts);

    // Once they leave, it's the searcher's turn.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut other.connection, TRAINING_FOREST, "", &mut trace).await,
        TRAINING_FOREST
    );
    let trace = try_portal(&mut searcher, TRAINING_FOREST, "in00").await;
    assert_eq!(arrived(&trace).0, TINOS_FOREST);
    assert!(trace.broadcasts.is_empty(), "{:?}", trace.broadcasts);
}

#[tokio::test]
async fn seeds_cards_and_tiguns_disguise_open_their_ways() {
    const USE: i16 = 2;
    const ETC: i16 = 4;
    const MAGIC_SEED: i32 = 4031346;
    const WARP_CARD: i32 = 4031890;
    const TIGUNS_DISGUISE: i32 = 2210005;
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;

    // Without the item, each way says what it needs.
    for (map, portal) in [
        (101010000, "minar00"),
        (240010100, "elli00"),
        (120000101, "earth01"),
        (221000300, "earth00"),
    ] {
        let trace = try_portal(&mut player, map, portal).await;
        assert!(trace.warps.is_empty(), "{map}: {:?}", trace.warps);
        assert!(pink(&trace), "{map}: {:?}", trace.broadcasts);
    }
    // The palace turns a stranger out to Ariant Castle's `R001`.
    let trace = try_portal(&mut player, 260000301, "ring00").await;
    assert_eq!(arrived(&trace), (260000300, 7));
    assert!(pink(&trace), "{:?}", trace.broadcasts);
    // Maple Island's balloons.
    let trace = try_portal(&mut player, 0, "tutorial0").await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(
        matches!(trace.hints.as_slice(), [(text, 250, 5)] if !text.is_empty()),
        "{:?}",
        trace.hints
    );

    drop(player);
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 1, MAGIC_SEED, 2).await.unwrap();
    grant_stack(id, ETC, 2, WARP_CARD, 1).await.unwrap();
    grant_stack(id, USE, 1, TIGUNS_DISGUISE, 1).await.unwrap();
    let mut player = relogin(&config).await;

    // A seed each way.
    let trace = try_portal(&mut player, 101010000, "minar00").await;
    assert_eq!(arrived(&trace).0, 240010100);
    assert!(trace.quantities.contains(&(ETC as u8, 1, 1)), "{trace:?}");
    assert!(pink(&trace), "{:?}", trace.broadcasts);
    let trace = try_portal(&mut player, 240010100, "elli00").await;
    assert_eq!(arrived(&trace).0, 101010000);
    assert_eq!(trace.removed, [(ETC as u8, 1)]);
    // The card is kept.
    for (map, portal, to) in [
        (120000101, "earth01", 221000300),
        (221000300, "earth00", 120000101),
    ] {
        let trace = try_portal(&mut player, map, portal).await;
        assert_eq!(arrived(&trace).0, to, "{map} {portal}");
        assert!(trace.removed.is_empty(), "{trace:?}");
    }

    // In Tigun's disguise, the palace lets the player be.
    let name = player.character_name.clone();
    let mut trace = step(&mut player.connection, &name, use_item(1, TIGUNS_DISGUISE)).await;
    until(&mut player.connection, &mut trace, |t| !t.buffs.is_empty()).await;
    let trace = try_portal(&mut player, 260000301, "ring00").await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(trace.broadcasts.is_empty(), "{:?}", trace.broadcasts);
}
