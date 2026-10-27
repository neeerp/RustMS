//! How to Shoo Away the Evil (2236): Chrishrama hands over six charms and
//! the info string "000000"; each of the six Shaman Rocks around Sleepywood
//! takes one charm and turns its own character to '1', the two in the same
//! swamp each their own; and only with "111111" and no charm left does
//! Chrishrama complete the quest, for 60,000 EXP.

use game_data::server_content::keys;
use integration_harness::events::{maps, said, talk_to};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{database::set_job, login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const SLEEPYWOOD: i32 = 105040300;
const CHRISHRAMA: i32 = 1061000;
const SHOO_AWAY_THE_EVIL: i16 = 2236;
const CHARM: i32 = 4032263;
/// The rocks in the order visited: map, NPC, which of that NPC on the map,
/// and its character in the string. The swamp's second rock goes first.
const ROCKS: [(i32, i32, usize, usize); 6] = [
    (105090000, 1063013, 1, 4),
    (105090000, 1063013, 0, 3),
    (105050200, 1063012, 0, 0),
    (105060000, 1063012, 0, 1),
    (105070000, 1063012, 0, 2),
    (105090100, 1063013, 0, 5),
];

/// The `nth` NPC `npc` on `map`: its object id, and a walk to it.
fn npc(map: i32, npc: i32, nth: usize) -> (i32, Packet) {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let (index, at) = (npcs.iter().enumerate())
        .filter(|(_, n)| n.npc_id == npc)
        .nth(nth)
        .unwrap();
    (1_000_000_000 + index as i32, walk_to(at.x, at.y))
}

#[tokio::test]
async fn each_shaman_rock_takes_a_charm_for_chrishrama() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, SLEEPYWOOD, "", &mut trace).await, SLEEPYWOOD);
    send(c, npc(SLEEPYWOOD, CHRISHRAMA, 0).1).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, SHOO_AWAY_THE_EVIL, CHRISHRAMA),
    )
    .await;
    assert_eq!(t.states.get(&SHOO_AWAY_THE_EVIL), Some(&1));
    assert_eq!(t.kills[&SHOO_AWAY_THE_EVIL], "000000");
    assert_eq!(t.items, [(CHARM, 6)]);

    let mut info = *b"000000";
    for (n, (map, rock, nth, slot)) in ROCKS.into_iter().enumerate() {
        assert_eq!(warp(c, map, "", &mut trace).await, map);
        let (rock, walk) = npc(map, rock, nth);
        send(c, walk).await;
        barrier(c, &m, &mut trace).await;
        let t = step(c, &m, talk_to(rock)).await;
        info[slot] = b'1';
        assert_eq!(t.kills[&SHOO_AWAY_THE_EVIL].as_bytes(), info, "rock {n}");
        assert_eq!(t.items, [(CHARM, -1)]);
        assert!(said(&t, keys::QUEST_INFO_SHAMAN_ROCK));
        step(c, &m, answer(0, 1, None)).await;
        // A charmed rock takes no second charm.
        let t = step(c, &m, talk_to(rock)).await;
        assert!(t.kills.is_empty() && t.items.is_empty(), "{t:?}");
        assert!(!said(&t, keys::QUEST_INFO_SHAMAN_ROCK));
        step(c, &m, answer(0, 1, None)).await;
    }

    assert_eq!(warp(c, SLEEPYWOOD, "", &mut trace).await, SLEEPYWOOD);
    send(c, npc(SLEEPYWOOD, CHRISHRAMA, 0).1).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(SHOO_AWAY_THE_EVIL, CHRISHRAMA, -1),
    )
    .await;
    assert_eq!(t.states.get(&SHOO_AWAY_THE_EVIL), Some(&2));
    assert_eq!(t.gains, [60_000]);
}
