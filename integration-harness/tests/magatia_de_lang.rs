//! Dr. De Lang, Magatia's missing alchemist: Parwen sends the player to
//! Dran's Lab (3320, 3353) and back there on a click once 3320 is taken;
//! De Lang starts 3321 and 3354 there. Parwen's password (3360) is the
//! quest's info, and the Secret Passage asks for it at the secret door;
//! given, the door opens and the quest can be completed.

use integration_harness::database::{set_job, set_quest};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PARWENS_MAP: i32 = 261020401;
const DRANS_LAB: i32 = 926120200;
const ZENUMIST_HALLWAY: i32 = 261010000;
const SECRET_BASEMENT_PATH: i32 = 261030000;
const ALCADNO_SOCIETY: i32 = 261000020;
const PARWEN: i32 = 2111006;
const DE_LANG: i32 = 2111002;
const MAED: i32 = 2111001;
const SECRET_PASSAGE: i32 = 2111024;
const VERIFYING_THE_PASSWORD: i16 = 3360;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
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

#[tokio::test]
async fn parwen_de_lang_and_the_secret_passage_password() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 75, 50).await.unwrap();
    for done in [3311, 3317, 3319, 3346, 3359] {
        set_quest(id, done, 2, "").await.unwrap();
    }
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // What Parwen Knows: asked, then started in Dran's Lab.
    assert_eq!(warp(c, PARWENS_MAP, "", &mut trace).await, PARWENS_MAP);
    send(c, to(PARWENS_MAP, PARWEN)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, 3320, PARWEN)).await;
    assert_eq!(t.prompts, [(PARWEN, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.progress, [(3320, 1)]);
    assert_eq!(t.warps, [DRANS_LAB]);
    send(c, to(DRANS_LAB, DE_LANG)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(3320, DE_LANG, -1)).await;
    assert_eq!(t.states.get(&3320), Some(&2));
    assert_eq!(t.gains, [5000]);

    // The Missing Alchemist: two pages, the ask, three more.
    let t = step(c, &m, build_quest_action(SCRIPT_START, 3321, DE_LANG)).await;
    assert_eq!(t.prompts, [(DE_LANG, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(DE_LANG, 12)]);
    step(c, &m, answer(12, 1, None)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(3321, 1)]);

    // Parwen sends the player back on a click; What De Lang Wants too.
    assert_eq!(warp(c, PARWENS_MAP, "", &mut trace).await, PARWENS_MAP);
    send(c, to(PARWENS_MAP, PARWEN)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(PARWENS_MAP, PARWEN)).await;
    assert_eq!(t.warps, [DRANS_LAB]);
    assert_eq!(warp(c, PARWENS_MAP, "", &mut trace).await, PARWENS_MAP);
    send(c, to(PARWENS_MAP, PARWEN)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_START, 3353, PARWEN)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(PARWEN, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.progress, [(3353, 1)]);
    assert_eq!(t.warps, [DRANS_LAB]);
    send(c, to(DRANS_LAB, DE_LANG)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(3353, DE_LANG, -1)).await;
    assert_eq!(t.states.get(&3353), Some(&2));
    let t = step(c, &m, build_quest_action(SCRIPT_START, 3354, DE_LANG)).await;
    assert_eq!(t.prompts, [(DE_LANG, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.progress, [(3354, 1)]);
    assert_eq!(
        warp(c, ALCADNO_SOCIETY, "", &mut trace).await,
        ALCADNO_SOCIETY
    );
    send(c, to(ALCADNO_SOCIETY, MAED)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(3354, MAED, -1)).await;
    assert_eq!(t.states.get(&3354), Some(&2));
    assert_eq!(t.gains, [100]);

    // Verifying the Password: the quest starts with Parwen's password.
    assert_eq!(warp(c, PARWENS_MAP, "", &mut trace).await, PARWENS_MAP);
    send(c, to(PARWENS_MAP, PARWEN)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_START, 3360, PARWEN)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(PARWEN, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.prompts, [(PARWEN, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(VERIFYING_THE_PASSWORD, 1)]);
    let password = t.kills[&VERIFYING_THE_PASSWORD].clone();
    assert_eq!(password.len(), 10, "{password:?}");
    // Not done before the password is given.
    let t = step(c, &m, build_quest_completion(3360, PARWEN, -1)).await;
    assert!(t.states.is_empty(), "{t:?}");

    // It is kept: the door asks for it after a relog, and a wrong one
    // keeps it shut.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(
        warp(c, ZENUMIST_HALLWAY, "", &mut trace).await,
        ZENUMIST_HALLWAY
    );
    let t = step(c, &m, enter("secret00")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.prompts, [(SECRET_PASSAGE, 2)]);
    let t = step(c, &m, type_text("0000000000")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.prompts, [(SECRET_PASSAGE, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, enter("secret00")).await;
    let t = step(c, &m, type_text(&password)).await;
    assert_eq!(t.kills[&VERIFYING_THE_PASSWORD], "1");
    assert_eq!(t.portal_sounds, 1);
    assert_eq!(t.warps, [SECRET_BASEMENT_PATH]);

    // Given, the door just opens, and Parwen completes the quest.
    assert_eq!(
        warp(c, ZENUMIST_HALLWAY, "", &mut trace).await,
        ZENUMIST_HALLWAY
    );
    let t = step(c, &m, enter("secret00")).await;
    assert_eq!(t.warps, [SECRET_BASEMENT_PATH]);
    assert_eq!(warp(c, PARWENS_MAP, "", &mut trace).await, PARWENS_MAP);
    send(c, to(PARWENS_MAP, PARWEN)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(3360, PARWEN, -1)).await;
    assert_eq!(t.states.get(&3360), Some(&2));
}
