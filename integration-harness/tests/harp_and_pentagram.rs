//! Two info quests their NPCs count up: Eliza's Eternal Sleep (3114), whose
//! harp strings play Twinkle Twinkle Little Star note by note, a wrong one
//! starting it over, and The Black Magician's Magic Pentagram (3345), whose
//! circles are touched in turn, the last taking the three Magic Stones.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const ELIZAS_GARDEN: i32 = 920020000;
const ELIZA: i32 = 2012026;
const ELIZAS_ETERNAL_SLEEP: i16 = 3114;
const SONG_BOOK: i32 = 4161036;
/// The harp's strings, do to si.
const HARP: [i32; 7] = [
    2012027, 2012028, 2012029, 2012030, 2012031, 2012032, 2012033,
];
const TUNE: &[u8; 42] = b"CCGGAAGFFEEDDCGGFFEEDGGFFEEDCCGGAAGFFEEDDC";

const EL_NATH_MARKET: i32 = 211000100;
const ALCASTER: i32 = 2020005;
const LAB: i32 = 261040000;
const MAGIC_PENTAGRAM: i16 = 3345;
const CIRCLES: [i32; 4] = [2111020, 2111021, 2111022, 2111023];
const STONES: [i32; 3] = [4031739, 4031740, 4031741];

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// Walk to NPC `npc` on `map` and talk to it.
async fn touch(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32) -> Trace {
    send(c, to(map, npc)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, talk(map, npc)).await
}

/// The string for note `note` of the tune.
fn string(note: u8) -> i32 {
    HARP[b"CDEFGAB".iter().position(|&n| n == note).unwrap()]
}

#[tokio::test]
async fn elizas_harp_plays_the_lullaby_through() {
    let (config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 200, 85, 50).await.unwrap();
    set_quest(id, 3113, 2, "").await.unwrap();
    let mut listener = login_to_world_session(&other_config).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    let (o, other) = (&mut listener.connection, &other_config.character_name);
    let mut heard = Trace::default();
    assert_eq!(warp(o, ELIZAS_GARDEN, "", &mut heard).await, ELIZAS_GARDEN);

    assert_eq!(warp(c, ELIZAS_GARDEN, "", &mut trace).await, ELIZAS_GARDEN);
    send(c, to(ELIZAS_GARDEN, ELIZA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, ELIZAS_ETERNAL_SLEEP, ELIZA),
    )
    .await;
    assert_eq!(t.states.get(&ELIZAS_ETERNAL_SLEEP), Some(&1));
    assert_eq!(t.items, [(SONG_BOOK, 1)]);

    // Two notes right, then a wrong one: over again.
    let t = touch(c, &m, ELIZAS_GARDEN, string(b'C')).await;
    assert_eq!(t.field_effects, [(4, "orbis/do".to_owned())]);
    assert_eq!(t.kills[&ELIZAS_ETERNAL_SLEEP], "1");
    touch(c, &m, ELIZAS_GARDEN, string(b'C')).await;
    let t = touch(c, &m, ELIZAS_GARDEN, string(b'A')).await;
    assert_eq!(t.kills[&ELIZAS_ETERNAL_SLEEP], "0");
    assert_eq!(t.broadcasts.len(), 1, "{t:?}");
    assert!(t
        .field_effects
        .contains(&(3, "quest/party/wrong_kor".to_owned())));
    // The other player there hears the strings.
    barrier(o, other, &mut heard).await;
    assert!(heard.field_effects.contains(&(4, "orbis/la".to_owned())));

    let mut t = Trace::default();
    for (n, &note) in TUNE.iter().enumerate() {
        t = touch(c, &m, ELIZAS_GARDEN, string(note)).await;
        assert_eq!(t.kills[&ELIZAS_ETERNAL_SLEEP], (n + 1).to_string());
    }
    assert!(t.field_effects.contains(&(4, "Party1/Clear".to_owned())));
    // Played through, a string only sounds.
    let t = touch(c, &m, ELIZAS_GARDEN, string(b'B')).await;
    assert!(t.kills.is_empty(), "{t:?}");

    send(c, to(ELIZAS_GARDEN, ELIZA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(ELIZAS_ETERNAL_SLEEP, ELIZA, -1),
    )
    .await;
    assert_eq!(t.states.get(&ELIZAS_ETERNAL_SLEEP), Some(&2));
    assert_eq!(t.fame_gains, [20]);
}

#[tokio::test]
async fn the_pentagram_takes_the_stones_from_its_last_circle() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 200, 75, 50).await.unwrap();
    set_quest(id, 3344, 2, "").await.unwrap();
    grant_stack(id, 4, 1, STONES[1], 1).await.unwrap();
    grant_stack(id, 4, 2, STONES[2], 1).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(
        warp(c, EL_NATH_MARKET, "", &mut trace).await,
        EL_NATH_MARKET
    );
    send(c, to(EL_NATH_MARKET, ALCASTER)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, MAGIC_PENTAGRAM, ALCASTER)).await;
    assert_eq!(t.states.get(&MAGIC_PENTAGRAM), Some(&1));
    assert_eq!(t.items, [(STONES[0], 1)]);

    assert_eq!(warp(c, LAB, "", &mut trace).await, LAB);
    let circle = |n: usize| CIRCLES[n];
    // Out of turn: over again.
    assert_eq!(
        touch(c, &m, LAB, circle(0)).await.kills[&MAGIC_PENTAGRAM],
        "1"
    );
    assert_eq!(
        touch(c, &m, LAB, circle(2)).await.kills[&MAGIC_PENTAGRAM],
        "0"
    );
    for n in 0..3 {
        let t = touch(c, &m, LAB, CIRCLES[n]).await;
        assert_eq!(t.kills[&MAGIC_PENTAGRAM], (n + 1).to_string());
        assert!(t.prompts.is_empty(), "{t:?}");
    }
    let t = touch(c, &m, LAB, circle(3)).await;
    assert_eq!(t.kills[&MAGIC_PENTAGRAM], "4");
    assert_eq!(t.prompts, [(CIRCLES[3], 0)]);
    let mut items = t.items.clone();
    items.sort();
    assert_eq!(items, STONES.map(|stone| (stone, -1)));

    assert_eq!(
        warp(c, EL_NATH_MARKET, "", &mut trace).await,
        EL_NATH_MARKET
    );
    send(c, to(EL_NATH_MARKET, ALCASTER)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(MAGIC_PENTAGRAM, ALCASTER, -1)).await;
    assert_eq!(t.states.get(&MAGIC_PENTAGRAM), Some(&2));
    assert_eq!(t.gains, [2200]);
}
