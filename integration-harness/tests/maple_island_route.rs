//! A new Explorer plays Maple Island from the Training Camp to Amherst and Mai the way
//! a player would: through portals only (no debug warps), doing every quest
//! on the way, hunting with melee attacks, and picking up quest drops.

use integration_harness::hunting::{attack, pickup, use_item, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, COMPLETE, SCRIPT_END, SCRIPT_START, START,
};
use integration_harness::trace::{self, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use packet::Packet;

const ROGERS_APPLE: i32 = 2010007;
const SHELLPIECE: i32 = 4031802;
const SENTINEL: i32 = 9300018;
const SNAIL: i32 = 100100;

/// One player's run and everything the server told them along the way.
struct Run {
    session: WorldSession,
    marker: String,
    trace: Trace,
}

impl Run {
    async fn send(&mut self, p: Packet) {
        trace::send(&mut self.session.connection, p).await;
    }

    async fn wait(&mut self, done: impl Fn(&Trace) -> bool) {
        trace::until(&mut self.session.connection, &mut self.trace, done).await;
    }

    /// Wait until the server has handled everything sent so far: our own
    /// chat line comes back only after that.
    async fn settle(&mut self) {
        trace::barrier(&mut self.session.connection, &self.marker, &mut self.trace).await;
    }

    async fn step(&mut self, p: Packet) {
        self.send(p).await;
        self.settle().await;
    }

    async fn quest(&mut self, action: u8, quest_id: i16, npc: i32) {
        self.step(build_quest_action(action, quest_id, npc)).await;
    }

    /// Take a portal by name, as the client does (map id -1).
    async fn portal(&mut self, name: &str, destination: i32) {
        let map = trace::warp(&mut self.session.connection, -1, name, &mut self.trace).await;
        assert_eq!(map, destination, "portal {name}");
        self.settle().await;
    }

    /// Kill `count` of `mob_id`, waiting for respawns if needed.
    async fn hunt(&mut self, mob_id: i32, count: usize) {
        for _ in 0..count {
            self.wait(|t| !t.field.alive(mob_id).is_empty()).await;
            let target = self.trace.field.alive(mob_id)[0];
            self.send(attack(target, 8)).await;
            self.wait(|t| t.field.killed.contains(&target)).await;
        }
        self.settle().await;
    }

    fn state(&self, quest_id: i16) -> Option<u8> {
        self.trace.states.get(&quest_id).copied()
    }
}

#[tokio::test]
async fn a_new_explorer_plays_maple_island_to_amherst() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    assert_eq!(session.map_id, 0);
    let mut run = Run {
        session,
        marker: config.character_name.clone(),
        trace: Trace::default(),
    };
    run.settle().await;

    // Outside the Training Camp: Heena sends us to Sera.
    run.quest(START, 1031, 2101).await;
    run.quest(COMPLETE, 1031, 2100).await;
    assert_eq!(run.state(1031), Some(2));

    // Mushroom Town: Roger's Apple. The script takes HP down to 25 and
    // hands over an apple; eating it lets Roger finish the quest.
    run.portal("out00", 10_000).await;
    run.step(build_quest_action(SCRIPT_START, 1021, 2000)).await;
    run.step(answer(0, 1, None)).await;
    run.step(answer(0, 1, None)).await;
    run.step(answer(12, 1, None)).await;
    assert_eq!(run.state(1021), Some(1));
    let &(_, slot, _) = run
        .trace
        .added
        .iter()
        .find(|(_, _, item)| *item == ROGERS_APPLE)
        .expect("Roger's apple");
    run.step(answer(0, 1, None)).await;
    run.step(answer(0, 1, None)).await;
    run.step(use_item(slot, ROGERS_APPLE)).await;
    run.step(build_quest_action(SCRIPT_END, 1021, 2000)).await;
    for _ in 0..3 {
        run.step(answer(0, 1, None)).await;
    }
    assert_eq!(run.state(1021), Some(2));

    // Further into town, and Sen's house: Nina and Sen's errands, and the
    // Mushroom Candy.
    run.portal("out00", 20_000).await;
    run.quest(START, 1032, 2102).await;
    run.portal("in01", 20_001).await;
    run.quest(COMPLETE, 1032, 2001).await;
    run.quest(START, 1033, 2001).await;
    run.portal("out00", 20_000).await;
    run.quest(COMPLETE, 1033, 2102).await;
    run.quest(START, 1034, 2102).await;
    run.portal("in01", 20_001).await;
    run.quest(COMPLETE, 1034, 2001).await;
    run.portal("out00", 20_000).await;
    for id in [1032, 1033, 1034] {
        assert_eq!(run.state(id), Some(2), "quest {id}");
    }

    // East Entrance: Todd's hunt, the shellpiece, and Peter up by the exit.
    run.portal("out00", 30_000).await;
    run.quest(START, 1035, 2004).await;
    run.hunt(SENTINEL, 1).await;
    run.wait(|t| t.field.item_drop(SHELLPIECE).is_some()).await;
    let shellpiece = run.trace.field.item_drop(SHELLPIECE).unwrap();
    run.send(walk_to(shellpiece.x, shellpiece.y)).await;
    run.send(pickup(shellpiece.object_id, shellpiece.x, shellpiece.y))
        .await;
    run.wait(|t| !t.field.removals(shellpiece.object_id).is_empty())
        .await;
    run.send(walk_to(972, -694)).await;
    run.quest(COMPLETE, 1035, 2002).await;
    assert_eq!(run.state(1035), Some(2));

    // Snail Hunting Ground I: Robin, then Sam's snails.
    run.portal("out00", 40_000).await;
    run.quest(START, 1036, 2003).await;
    run.quest(COMPLETE, 1036, 2003).await;
    assert_eq!(run.state(1036), Some(2));
    run.quest(START, 1037, 2005).await;
    run.hunt(SNAIL, 10).await;
    assert_eq!(run.trace.kills.get(&1037).map(String::as_str), Some("010"));

    // A Split Road: Maria, and her letter for Lucas across Amherst.
    run.portal("east00", 40_001).await;
    run.portal("east00", 40_002).await;
    run.portal("east00", 50_000).await;
    run.quest(COMPLETE, 1037, 2103).await;
    run.quest(START, 1038, 2103).await;
    run.portal("east00", 1_000_000).await;
    run.portal("east00", 1_010_000).await;
    run.send(walk_to(1500, 154)).await;
    run.quest(COMPLETE, 1038, 12000).await;
    assert_eq!(run.state(1038), Some(2));

    // Lucas's introduction sends us to Mai; it can't be finished until her
    // training (1041-1044) is done.
    run.quest(START, 1040, 12000).await;
    assert_eq!(run.state(1040), Some(1));
    run.quest(COMPLETE, 1040, 12000).await;
    assert_eq!(run.state(1040), Some(1));
    run.portal("east00", 1_020_000).await;
    run.send(walk_to(-21, 95)).await;
    run.quest(START, 1041, 12100).await;
    assert_eq!(run.state(1041), Some(1));
    let gained: i32 = run.trace.gains.iter().sum();
    drop(run);

    // Everything was saved: the quests, the map, and the EXP.
    let restored = relogin(&config).await;
    assert_eq!(restored.map_id, 1_020_000);
    for id in [1021, 1031, 1032, 1033, 1034, 1035, 1036, 1037, 1038] {
        assert_eq!(restored.quests.get(&id), Some(&2), "quest {id}");
    }
    for id in [1040, 1041] {
        assert_eq!(restored.quests.get(&id), Some(&1), "quest {id}");
    }
    assert!(
        restored.level >= 3,
        "level {} after {gained} EXP",
        restored.level
    );
}
