//! Who is the Owner of the Mysterious Note? (2238) and To Tristan's Tomb
//! (2245): Mu Young's start scripts start 2238, then send the player alone
//! into the unknown Jr. Balrog's room for ten minutes, completing 2245 so
//! it may be taken again. Its fall brings out Tristan's Spirit, where 2238
//! ends. One player at a time may be in the room on a channel. The harness
//! world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the ten
//! minutes are ten seconds.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, SCRIPT_START};
use integration_harness::trace::{read_for, send, step, until, until_within, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::events::{MU_YOUNG_MAP, TRISTANS_SPIRIT, TRISTAN_BALROG, TRISTAN_ROOM};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const MU_YOUNG: i32 = 1_061_014;

/// A level 55 fighter, armed to fell Jr. Balrog, with 2237 done, waiting
/// at Mu Young.
async fn at_mu_young(config: &HarnessConfig) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 55, 50).await.expect("a level 55 warrior");
    replace_item(id, -1, -11, 1302000).await.expect("a sword");
    set_weapon_attack(id, 500).await.expect("its attack");
    set_str(id, 999).await.expect("strength");
    set_quest(id, 2237, 2, "").await.expect("2237 done");
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, MU_YOUNG_MAP, "", &mut trace).await, MU_YOUNG_MAP);
    step(c, &config.character_name, walk_to(617, 246)).await;
    session
}

/// Ask Mu Young for 2245 and follow what comes of it for a moment.
async fn ask_to_go(c: &mut MapleTestConnection) -> Trace {
    send(c, build_quest_action(SCRIPT_START, 2245, MU_YOUNG)).await;
    let mut trace = Trace::default();
    read_for(c, &mut trace, 1_500).await;
    trace
}

#[tokio::test]
async fn jr_balrogs_fall_brings_out_tristans_spirit() {
    let (config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let m = config.character_name.clone();
    let mut session = at_mu_young(&config).await;
    let mut other = at_mu_young(&other_config).await;
    let c = &mut session.connection;

    // 2238 starts with no word, and 2245 takes the player in.
    let t = step(c, &m, build_quest_action(SCRIPT_START, 2238, MU_YOUNG)).await;
    assert!(t.prompts.is_empty(), "Mu Young says nothing");
    assert_eq!(t.progress, [(2238, 1)]);
    // What came on entering the room, Jr. Balrog among it.
    let mut trace = ask_to_go(c).await;
    assert!(trace.prompts.is_empty(), "nor here");
    assert_eq!(trace.progress, [(2245, 2)], "2245 is done as it starts");
    assert_eq!(trace.warps, [TRISTAN_ROOM]);
    assert_eq!(trace.countdowns.len(), 1, "the ten minutes run");

    // Someone else on 2238 must wait while the room is taken.
    let n = other_config.character_name.clone();
    let o = &mut other.connection;
    step(o, &n, build_quest_action(SCRIPT_START, 2238, MU_YOUNG)).await;
    let t = ask_to_go(o).await;
    assert_eq!(t.prompts, [(MU_YOUNG, 0)], "told the room is taken");
    assert!(t.progress.is_empty() && t.warps.is_empty());

    // Jr. Balrog falls, and Tristan's Spirit comes out, where 2238 ends.
    until(c, &mut trace, |t| t.field.alive(TRISTAN_BALROG).len() == 1).await;
    let balrog = trace.field.alive(TRISTAN_BALROG)[0];
    let spirit = |t: &Trace| {
        t.field
            .npcs
            .iter()
            .find(|&&(_, npc)| npc == TRISTANS_SPIRIT)
            .map(|&(oid, _)| oid)
    };
    for _ in 0..40 {
        send(c, attack(balrog, 30_000)).await;
        read_for(c, &mut trace, 100).await;
        if trace.field.killed.contains(&balrog) {
            break;
        }
    }
    until(c, &mut trace, |t| {
        t.field.killed.contains(&balrog) && spirit(t).is_some()
    })
    .await;
    let oid = spirit(&trace).unwrap();
    let (x, y) = trace.field.npc_positions[&oid];
    step(c, &m, walk_to(x, y)).await;
    let t = step(c, &m, build_quest_completion(2238, TRISTANS_SPIRIT, -1)).await;
    assert_eq!(t.progress, [(2238, 2)]);
    assert_eq!(t.gains, [50_000]);

    // Out by its portal, back to Mu Young, and the room is free again.
    send(c, walk_to(-343, 188)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("out00").unwrap();
    p.write_short(0).unwrap();
    send(c, p).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [MU_YOUNG_MAP]).await;
    drop(session);

    // The other goes in at last, and the time running out sends them back.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let o = &mut other.connection;
    let t = ask_to_go(o).await;
    assert_eq!(t.progress, [(2245, 2)]);
    assert_eq!(t.warps, [TRISTAN_ROOM]);
    let mut trace = Trace::default();
    until_within(o, &mut trace, Duration::from_secs(20), |t| {
        t.warps == [MU_YOUNG_MAP]
    })
    .await;
    drop(other);

    // 2238 is done for good, and 2245 stays done.
    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&2238), Some(&2));
    assert_eq!(restored.quests.get(&2245), Some(&2));
}
