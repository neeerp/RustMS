//! The Puppeteer's password: the Cave Wall of Evil Eye at the Sleepywood
//! cave's door asks for it, and lets a knight hunting the Puppeteer (20730)
//! into Francis's room on the right one. Francis sends the intruder into the
//! hideout, where the Puppeteer waits. An Aran on The Puppeteer's Cave
//! (21728) searches the wall instead, which sets the record its completion
//! reads.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const EREVE: i32 = 130000000;
const NEINHEART: i32 = 1101002;
const FIND_THE_PUPPETEER: i16 = 20730;
const PUPPETEER: i32 = 9300285;

const SLEEPYWOOD_CAVE: i32 = 105070300;
/// The cave's door, `in00`.
const CAVE_DOOR: (i16, i16) = (118, 622);
const CAVE_WALL: i32 = 1063011;
const FRANCIS_ROOM: i32 = 910510001;
const FRANCIS: i32 = 1104000;
const HIDEOUT: i32 = 910510000;

const ANOTHER_SIDE: i32 = 105040300;
const ARAN_QUEST_NPC: i32 = 1061019;
const THE_PUPPETEERS_CAVE: i16 = 21728;
const CAVE_RECORD: i16 = 21761;

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

/// Going through a scripted portal.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// The answer to a text prompt.
fn type_text(text: &str) -> Packet {
    let mut p = answer(2, 1, None);
    p.write_str_with_length(text).unwrap();
    p
}

/// At the cave's door, press Up on it.
async fn knock(c: &mut MapleTestConnection, m: &str) -> Trace {
    step(c, m, walk_to(CAVE_DOOR.0, CAVE_DOOR.1)).await;
    step(c, m, enter("in00")).await
}

#[tokio::test]
async fn the_password_lets_the_puppeteers_hunter_into_the_hideout() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1100, 30, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    set_quest(id, 20725, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Not yet hunting: the right password is right, but the way stays shut.
    assert_eq!(
        warp(c, SLEEPYWOOD_CAVE, "", &mut trace).await,
        SLEEPYWOOD_CAVE
    );
    let t = knock(c, &m).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.prompts, [(CAVE_WALL, 2)]);
    let t = step(c, &m, type_text("Francis is a genius Puppeteer!")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.broadcasts.len(), 1, "told in pink: {:?}", t.broadcasts);

    assert_eq!(warp(c, EREVE, "", &mut trace).await, EREVE);
    send(c, to(EREVE, NEINHEART)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, FIND_THE_PUPPETEER, NEINHEART),
    )
    .await;
    assert_eq!(t.states.get(&FIND_THE_PUPPETEER), Some(&1));

    // A wrong password keeps it shut; the right one, in any case, opens it.
    assert_eq!(
        warp(c, SLEEPYWOOD_CAVE, "", &mut trace).await,
        SLEEPYWOOD_CAVE
    );
    knock(c, &m).await;
    let t = step(c, &m, type_text("open sesame")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.prompts, [(CAVE_WALL, 0)]);
    send(c, answer(0, 1, None)).await;
    let t = knock(c, &m).await;
    assert_eq!(t.prompts, [(CAVE_WALL, 2)]);
    let t = step(c, &m, type_text("francis is a genius puppeteer")).await;
    assert_eq!(t.portal_sounds, 1);
    assert_eq!(t.warps, [FRANCIS_ROOM]);

    // Francis catches the intruder and sends them to the hideout, where
    // the Puppeteer waits.
    let t = touch(c, &m, FRANCIS_ROOM, FRANCIS).await;
    assert_eq!(t.prompts, [(FRANCIS, 0)]);
    send(c, answer(0, 1, None)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [HIDEOUT] && t.field.alive(PUPPETEER).len() == 1
    })
    .await;

    let puppeteer = trace.field.alive(PUPPETEER)[0];
    for _ in 0..20 {
        send(c, attack(puppeteer, 30_000)).await;
        read_for(c, &mut trace, 100).await;
        if trace.field.killed.contains(&puppeteer) {
            break;
        }
    }
    assert_eq!(trace.kills[&FIND_THE_PUPPETEER], "001");

    // Found, the door leads back the way it always did.
    assert_eq!(warp(c, EREVE, "", &mut trace).await, EREVE);
    send(c, to(EREVE, NEINHEART)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(FIND_THE_PUPPETEER, NEINHEART, -1),
    )
    .await;
    assert_eq!(t.states.get(&FIND_THE_PUPPETEER), Some(&2));
    assert_eq!(t.gains, [12000]);
    assert_eq!(
        warp(c, SLEEPYWOOD_CAVE, "", &mut trace).await,
        SLEEPYWOOD_CAVE
    );
    let t = knock(c, &m).await;
    assert!(t.prompts.is_empty(), "{t:?}");
    assert_eq!(t.warps.len(), 1, "{t:?}");
}

#[tokio::test]
async fn an_aran_searches_the_cave_wall() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2100, 30, 50).await.unwrap();
    set_quest(id, 21727, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, ANOTHER_SIDE, "", &mut trace).await, ANOTHER_SIDE);
    send(c, to(ANOTHER_SIDE, ARAN_QUEST_NPC)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, THE_PUPPETEERS_CAVE, ARAN_QUEST_NPC),
    )
    .await;
    assert_eq!(t.states.get(&THE_PUPPETEERS_CAVE), Some(&1));
    // Not searched yet: not done.
    let t = step(
        c,
        &m,
        build_quest_completion(THE_PUPPETEERS_CAVE, ARAN_QUEST_NPC, -1),
    )
    .await;
    assert!(t.states.is_empty(), "{t:?}");

    assert_eq!(
        warp(c, SLEEPYWOOD_CAVE, "", &mut trace).await,
        SLEEPYWOOD_CAVE
    );
    let t = knock(c, &m).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.prompts, [(CAVE_WALL, 0)]);
    assert_eq!(t.kills[&CAVE_RECORD], "0");
    send(c, answer(0, 1, None)).await;

    assert_eq!(warp(c, ANOTHER_SIDE, "", &mut trace).await, ANOTHER_SIDE);
    send(c, to(ANOTHER_SIDE, ARAN_QUEST_NPC)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(THE_PUPPETEERS_CAVE, ARAN_QUEST_NPC, -1),
    )
    .await;
    assert_eq!(t.states.get(&THE_PUPPETEERS_CAVE), Some(&2));
    assert_eq!(t.gains, [200]);
}
