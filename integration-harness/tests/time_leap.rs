//! The Buccaneers' Time Leap (6361-6363): with Papulatus's Key the way to
//! Papulatus leads into the past instead, where the past Papulatus takes the
//! key, then sends the Buccaneer back with the record "A New Revelation"
//! completes on with Rolonay.

use game_data::server_content::keys;
use integration_harness::database::{grant_stack, set_job, start_quest};
use integration_harness::events::{maps, said, talk, told};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, build_quest_completion, START};
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::CLOCKTOWER_DEPTHS;
use net::packet::op::RecvOpcode;
use net::scripts::{NEW_REVELATION, PAST_CLOCKTOWER, SENT_BACK, SENT_BACK_RECORD};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ETC: i16 = 4;
const PAPULATUS_KEY: i32 = 4031870;
const PAPULATUS_KEY_QUEST: i16 = 6361;
const IN_THE_PAST: i16 = 6362;
const PAST_PAPULATUS: i32 = 2043000;
const ROLONAY: i32 = 1092012;
const ROLONAYS_MAP: i32 = 120000200;

/// CHANGE_MAP_SPECIAL: press Up on the way to Papulatus, `in00`.
fn enter() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("in00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// Walk `player`, on `map`, up to `npc`, near enough for its quests.
async fn beside(player: &mut WorldSession, map: i32, npc: i32) {
    let field = maps().field(map).unwrap();
    let at = field.map_npcs.iter().find(|n| n.npc_id == npc).unwrap();
    told(player, walk_to(at.x, at.y)).await;
}

/// `player` takes the quest step `packet` and is left with `quest` at `state`.
async fn step(player: &mut WorldSession, packet: Packet, quest: i16, state: u8) {
    let mut seen = told(player, packet).await;
    until(&mut player.connection, &mut seen, |t| {
        t.states.get(&quest) == Some(&state)
    })
    .await;
}

#[tokio::test]
async fn the_key_leads_a_buccaneer_into_the_past_and_back_with_the_revelation() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 512, 120, 50).await.unwrap();
    start_quest(id, PAPULATUS_KEY_QUEST).await.unwrap();
    grant_stack(id, ETC, 1, PAPULATUS_KEY, 1).await.unwrap();
    let mut a = relogin(&config).await;
    let mut moved = Trace::default();
    assert_eq!(
        warp(&mut a.connection, CLOCKTOWER_DEPTHS, "", &mut moved).await,
        CLOCKTOWER_DEPTHS
    );

    // With the key, alone and in no party, the way leads into the past.
    let mut seen = told(&mut a, enter()).await;
    until(&mut a.connection, &mut seen, |t| {
        t.warps.contains(&PAST_CLOCKTOWER)
    })
    .await;

    // The past Papulatus takes the key and tells the Buccaneer the rest.
    beside(&mut a, PAST_CLOCKTOWER, PAST_PAPULATUS).await;
    let key_quest = build_quest_completion(PAPULATUS_KEY_QUEST, PAST_PAPULATUS, -1);
    step(&mut a, key_quest, PAPULATUS_KEY_QUEST, 2).await;
    let past = build_quest_action(START, IN_THE_PAST, PAST_PAPULATUS);
    step(&mut a, past, IN_THE_PAST, 1).await;
    let past = build_quest_completion(IN_THE_PAST, PAST_PAPULATUS, -1);
    step(&mut a, past, IN_THE_PAST, 2).await;
    let revelation = build_quest_action(START, NEW_REVELATION, PAST_PAPULATUS);
    step(&mut a, revelation, NEW_REVELATION, 1).await;

    // He sends them back to their own time with the record.
    let trace = told(&mut a, talk(PAST_CLOCKTOWER, PAST_PAPULATUS)).await;
    assert!(said(&trace, keys::PAST_PAPULATUS_SEND_BACK), "{trace:?}");
    let mut seen = told(&mut a, answer(0, 1, None)).await;
    until(&mut a.connection, &mut seen, |t| {
        t.kills.get(&SENT_BACK_RECORD).map(String::as_str) == Some(SENT_BACK)
            && t.warps.contains(&CLOCKTOWER_DEPTHS)
    })
    .await;

    // Rolonay completes "A New Revelation" on it.
    warp(&mut a.connection, ROLONAYS_MAP, "", &mut moved).await;
    beside(&mut a, ROLONAYS_MAP, ROLONAY).await;
    let done = build_quest_completion(NEW_REVELATION, ROLONAY, -1);
    step(&mut a, done, NEW_REVELATION, 2).await;
}
