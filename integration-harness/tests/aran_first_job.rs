//! An Aran's first job (row 191): at level 10 Lilin's The Five Heroes
//! (21100) points a Legend at the Polearm in Rien's ice, whose The
//! Polearm-Wielding Hero (21101) starts at a touch and, held again, makes
//! them an Aran (2100) with its medal.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_job;
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const RIEN: i32 = 140000000;
const LILIN: i32 = 1201000;
const POLEARM: i32 = 1201001;
const FIVE_HEROES: i16 = 21100;
const POLEARM_WIELDING_HERO: i16 = 21101;
const ARAN_MEDAL: i32 = 1142129;
const JOB: i32 = 0x20;

/// Walk to NPC `npc` in Rien and send a quest action to it.
async fn quest(c: &mut MapleTestConnection, m: &str, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(RIEN).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

/// Click through `n` say pages.
async fn next(c: &mut MapleTestConnection, m: &str, n: usize) -> Trace {
    let mut t = Trace::default();
    for _ in 0..n {
        t = step(c, m, answer(0, 1, None)).await;
    }
    t
}

#[tokio::test]
async fn lilin_and_the_polearm_make_a_legend_of_10_an_aran() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2000, 10, 50).await.expect("a Legend of 10");

    let mut before = relogin(&config).await;
    let c = &mut before.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut t).await, RIEN);

    // Lilin tells of the Polearm and starts The Five Heroes, and the client
    // is pointed at it; she ends it from the data.
    let t = quest(
        c,
        &m,
        LILIN,
        build_quest_action(SCRIPT_START, FIVE_HEROES, LILIN),
    )
    .await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    let t = next(c, &m, 6).await;
    assert_eq!(t.prompts, [(LILIN, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&FIVE_HEROES), Some(&1));
    assert_eq!(
        t.scene_parts,
        ["Effect/Direction1.img/aranTutorial/ClickPoleArm"]
    );
    let t = next(c, &m, 1).await;
    assert!(t.prompts.is_empty());
    let t = step(c, &m, build_quest_completion(FIVE_HEROES, LILIN, -1)).await;
    assert_eq!(t.states.get(&FIVE_HEROES), Some(&2));

    // A touch starts the Polearm's quest; held again, it makes an Aran.
    let start = build_quest_action(SCRIPT_START, POLEARM_WIELDING_HERO, POLEARM);
    let t = quest(c, &m, POLEARM, start).await;
    assert_eq!(t.prompts, [(POLEARM, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&POLEARM_WIELDING_HERO), Some(&1));
    next(c, &m, 1).await;
    let end = build_quest_action(SCRIPT_END, POLEARM_WIELDING_HERO, POLEARM);
    let t = step(c, &m, end).await;
    assert_eq!(t.prompts, [(POLEARM, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [2100]);
    assert_eq!(t.states.get(&POLEARM_WIELDING_HERO), Some(&2));
    let grown = [1, 4].map(|kind| (kind, before.slot_limits[usize::from(kind) - 1] + 4));
    assert_eq!(t.bag_slots, grown);
    assert_eq!(
        t.added.iter().map(|a| (a.0, a.2)).collect::<Vec<_>>(),
        [(1, ARAN_MEDAL)]
    );
    let t = next(c, &m, 1).await;
    assert!(t.prompts.is_empty());
    drop(before);

    let after = relogin(&config).await;
    assert_eq!(after.job, 2100);
    assert_eq!(after.items.get(&ARAN_MEDAL), Some(&1));
}
