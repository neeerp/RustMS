//! The Forest of Poison Haze, its first slice: Ellin in the Deep Fairy
//! Forest takes those of a party of four to six at levels 44 to 55 who are
//! there into its own copy of the forest; the entrance leads on to the first
//! stage, whose way on stays shut until all its Poisoned Lord Trees are
//! dead; the other Ellin there lets a member out to the prize map, whose
//! portal leads home; and when the time runs out everyone lands on the
//! prize map. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the forest lasts 30 seconds.

use game_data::server_content::keys;
use integration_harness::events::{armed_in, cleared, maps, party, talk, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{ELLIN, ELLIN_ENTRY, ELLIN_EXIT, ELLIN_LOBBY, ELLIN_STAGES};
use net::npc_scripts::ELLIN as ELLIN_NPC;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// Ellin on each of the forest's maps.
const ELLIN_GUIDE: i32 = 2133001;
/// The Poisoned Lord Tree, and how many the first stage holds.
const TREE: i32 = 9300172;
const TREES: usize = 50;
const PINK: u8 = 5;

/// `key`, with who may play.
fn terms(key: &str) -> String {
    let (team, levels) = (&ELLIN.team, &ELLIN.levels);
    net::content::text().get(
        key,
        &[
            ("least", &team.start().to_string()),
            ("most", &team.end().to_string()),
            ("low", &levels.start().to_string()),
            ("high", &levels.end().to_string()),
        ],
    )
}

/// CHANGE_MAP_SPECIAL: walk to `from`'s way on, `east00`, and press Up.
async fn way_on(player: &mut WorldSession, from: i32) {
    let field = maps().field(from).unwrap();
    let portal = field.portal_by_name("east00").unwrap();
    let (x, y) = (portal.x as i16, portal.y as i16);
    send(&mut player.connection, walk_to(x, y)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("east00").unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

/// `player` takes `from`'s way on and arrives on `to`.
async fn through(player: &mut WorldSession, from: i32, to: i32) {
    way_on(player, from).await;
    let mut trace = Trace::default();
    until(&mut player.connection, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [to], "{trace:?}");
}

#[tokio::test]
async fn ellin_takes_five_in_and_the_first_stage_clears_once_its_trees_are_dead() {
    let mut team = Vec::new();
    for _ in 0..5 {
        team.push(armed_in(ELLIN_LOBBY, 50).await);
    }
    party(&mut team).await;

    // Only the leader may take the party in, and only with four there.
    for player in team[3..].iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut player.connection, 300_030_000, "", &mut trace).await,
            300_030_000
        );
    }
    assert_eq!(
        told(&mut team[1], talk(ELLIN_LOBBY, ELLIN_NPC))
            .await
            .prompts,
        [(ELLIN_NPC, 4)]
    );
    let trace = told(&mut team[1], answer(4, 1, Some(0))).await;
    let not_leader = net::content::text().get(keys::ELLIN_NOT_LEADER, &[]);
    assert!(trace.prompt_texts[0].contains(&not_leader), "{trace:?}");
    told(&mut team[0], talk(ELLIN_LOBBY, ELLIN_NPC)).await;
    let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
    let not_eligible = terms(keys::ELLIN_NOT_ELIGIBLE);
    assert!(trace.prompt_texts[0].contains(&not_eligible), "{trace:?}");

    // With the two back, the five go in, once any party before them is
    // out, onto the entrance, on the clock.
    for player in team[3..].iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut player.connection, ELLIN_LOBBY, "", &mut trace).await,
            ELLIN_LOBBY
        );
    }
    let mut seen = None;
    for _ in 0..30 {
        told(&mut team[0], talk(ELLIN_LOBBY, ELLIN_NPC)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == ELLIN_NPC) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Ellin never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&ELLIN_ENTRY) && !t.countdowns.is_empty()
        })
        .await;
    }

    // The entrance leads on to the first stage, whose way on is shut while
    // a tree lives.
    for player in team.iter_mut() {
        through(player, ELLIN_ENTRY, ELLIN_STAGES[0]).await;
    }
    let shut = net::content::text().get(keys::ELLIN_STAGE1_SHUT, &[]);
    way_on(&mut team[0], ELLIN_STAGES[0]).await;
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        t.broadcasts.contains(&(PINK, shut.clone()))
    })
    .await;
    assert!(seen.warps.is_empty(), "{seen:?}");

    // The leader kills every tree; with the last the stage clears.
    until(c, &mut seen, |t| {
        t.field.mobs.iter().filter(|m| m.1 == TREE).count() >= TREES
    })
    .await;
    let trees: Vec<i32> = (seen.field.mobs.iter())
        .filter(|m| m.1 == TREE)
        .map(|m| m.0)
        .collect();
    for &oid in &trees {
        send(c, attack(oid, 30_000)).await;
        until(c, &mut seen, |t| t.field.killed.contains(&oid)).await;
    }
    until(c, &mut seen, cleared).await;
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, cleared).await;
    }

    // Now the way on leads to the second stage.
    through(&mut team[0], ELLIN_STAGES[0], ELLIN_STAGES[1]).await;

    // The other Ellin lets a member out to the prize map, whose portal
    // leads home; four go on without them.
    let trace = told(&mut team[4], talk(ELLIN_STAGES[0], ELLIN_GUIDE)).await;
    assert_eq!(trace.prompts, [(ELLIN_GUIDE, 1)], "{trace:?}");
    assert_eq!(
        told(&mut team[4], answer(1, 1, None)).await.warps,
        [ELLIN_EXIT]
    );
    through(&mut team[4], ELLIN_EXIT, ELLIN_LOBBY).await;

    // The time runs out: everyone still in lands on the prize map.
    for player in team[..4].iter_mut() {
        let mut trace = Trace::default();
        for _ in 0..30 {
            if trace.warps.contains(&ELLIN_EXIT) {
                break;
            }
            read_for(&mut player.connection, &mut trace, 1_000).await;
        }
        assert!(trace.warps.contains(&ELLIN_EXIT), "{trace:?}");
    }
}
