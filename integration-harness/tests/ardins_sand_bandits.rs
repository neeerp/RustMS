//! Ardin's Sand Bandits (3933): Ardin's start script sends one player at a
//! time to face Ardin's other self, which counts toward the quest; the way
//! out leads back, and the room is fresh for the next.

use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::events::maps;
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START,
};
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::scripts::ARDINS_ROOM;
use packet::Packet;
use tokio::time::Duration;

const ARDINS_MAP: i32 = 260000200;
const ARDIN: i32 = 2101003;
const OTHER_SELF: i32 = 9100013;
const ARDINS_SAND_BANDITS: i16 = 3933;

/// A walk to Ardin.
fn to_ardin() -> Packet {
    let npcs = &maps().field(ARDINS_MAP).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == ARDIN).unwrap();
    walk_to(at.x, at.y)
}

/// A new level-30 Warrior past 3932 and hitting very hard, beside Ardin.
async fn bandit() -> (WorldSession, String) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    set_quest(id, 3932, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let m = config.character_name.clone();
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, ARDINS_MAP, "", &mut seen).await, ARDINS_MAP);
    send(c, to_ardin()).await;
    barrier(c, &m, &mut seen).await;
    (session, m)
}

/// Ardin's script read through, saying yes: the trace of its last page.
async fn ask_ardin(player: &mut WorldSession, m: &str) -> Trace {
    let c = &mut player.connection;
    let t = step(
        c,
        m,
        build_quest_action(SCRIPT_START, ARDINS_SAND_BANDITS, ARDIN),
    )
    .await;
    assert_eq!(t.prompts, [(ARDIN, 0)]);
    let t = step(c, m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(ARDIN, 12)]);
    let t = step(c, m, answer(12, 1, None)).await;
    assert_eq!(t.prompts, [(ARDIN, 0)]);
    step(c, m, answer(0, 1, None)).await
}

#[tokio::test]
async fn ardin_sends_one_player_at_a_time_to_face_the_other_self() {
    let (mut a, am) = bandit().await;
    let (mut b, bm) = bandit().await;

    let mut seen = ask_ardin(&mut a, &am).await;
    assert_eq!(seen.progress, [(ARDINS_SAND_BANDITS, 1)]);
    assert_eq!(seen.warps, [ARDINS_ROOM]);

    // The next is turned away while the room is taken.
    let t = ask_ardin(&mut b, &bm).await;
    assert!(t.warps.is_empty() && t.progress.is_empty(), "{t:?}");
    assert_eq!(t.prompts, [(ARDIN, 0)]);
    step(&mut b.connection, &bm, answer(0, 1, None)).await;

    // The other self counts toward the quest.
    let c = &mut a.connection;
    until(c, &mut seen, |t| !t.field.alive(OTHER_SELF).is_empty()).await;
    let other_self = seen.field.alive(OTHER_SELF)[0];
    send(c, attack(other_self, 5_000_000)).await;
    until(c, &mut seen, |t| {
        t.kills.get(&ARDINS_SAND_BANDITS).map(String::as_str) == Some("001")
    })
    .await;

    // The way out leads back to Ardin, who completes the quest.
    let mut out = Trace::default();
    assert_eq!(warp(c, -1, "st00", &mut out).await, ARDINS_MAP);
    send(c, to_ardin()).await;
    barrier(c, &am, &mut out).await;
    let t = step(
        c,
        &am,
        build_quest_completion(ARDINS_SAND_BANDITS, ARDIN, -1),
    )
    .await;
    assert_eq!(t.states.get(&ARDINS_SAND_BANDITS), Some(&2));
    assert_eq!(t.gains, [7250]);

    // The room is fresh for the next.
    let mut seen = ask_ardin(&mut b, &bm).await;
    assert_eq!(seen.warps, [ARDINS_ROOM]);
    until(&mut b.connection, &mut seen, |t| {
        !t.field.alive(OTHER_SELF).is_empty()
    })
    .await;
}
