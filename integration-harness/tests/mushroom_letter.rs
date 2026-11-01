//! Endangered Mushking Empire (2300-2310): a job instructor's start script
//! gives a level-30 second-job character a recommendation letter, which starts the
//! quest, and can send them straight to the kingdom's opening scene; the
//! Head Security Officer's end script, where the player answers him, takes
//! the letter for the quest and 1,200 EXP.

use integration_harness::database::set_job;
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ENDANGERED_MUSHKING_EMPIRE: i16 = 2300;
const DANCES_WITH_BALROG: i32 = 1022000;
const WARRIORS_SANCTUARY: i32 = 102000003;
const LETTER: i32 = 4032375;
const OPENING: i32 = 106020001;
const MUSHROOM_FOREST_FIELD: i32 = 106020000;
const OFFICER: i32 = 1300005;
const PLAYER_SPEAKS: u8 = 2;
const DAWN_WARRIORS_LETTER: i16 = 2305;
const MIHILE: i32 = 1101003;
const EREVE: i32 = 130000000;
const DAWN_WARRIOR: i16 = 1110;

/// CHANGE_MAP by id, as the client asks for a scene's field when it ends.
fn scene_end(field: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMap as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(field).unwrap();
    p.write_str_with_length("").unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

#[tokio::test]
async fn the_instructors_letter_takes_the_player_to_the_mushking_empire() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 30, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(
        warp(c, WARRIORS_SANCTUARY, "", &mut trace).await,
        WARRIORS_SANCTUARY
    );
    let npcs = &maps().field(WARRIORS_SANCTUARY).unwrap().map_npcs;
    let instructor = (npcs.iter())
        .find(|n| n.npc_id == DANCES_WITH_BALROG)
        .unwrap();
    send(c, walk_to(instructor.x, instructor.y)).await;
    barrier(c, &m, &mut trace).await;
    let start = build_quest_action(SCRIPT_START, ENDANGERED_MUSHKING_EMPIRE, DANCES_WITH_BALROG);
    let t = step(c, &m, start).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 1)], "asked");
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)], "the kingdom");
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(LETTER, 1)]);
    assert_eq!(t.progress, [(ENDANGERED_MUSHKING_EMPIRE, 1)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 1)], "shall I take you?");
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [OPENING]);
    let t = step(c, &m, scene_end(MUSHROOM_FOREST_FIELD)).await;
    assert_eq!(t.warps, [MUSHROOM_FOREST_FIELD]);

    let npcs = &maps().field(MUSHROOM_FOREST_FIELD).unwrap().map_npcs;
    let officer = npcs.iter().find(|n| n.npc_id == OFFICER).unwrap();
    send(c, walk_to(officer.x, officer.y)).await;
    barrier(c, &m, &mut trace).await;
    let end = build_quest_action(SCRIPT_END, ENDANGERED_MUSHKING_EMPIRE, OFFICER);
    let t = step(c, &m, end).await;
    assert_eq!(t.prompts, [(OFFICER, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompt_speakers, [PLAYER_SPEAKS], "the player answers");
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&ENDANGERED_MUSHKING_EMPIRE), Some(&2));
    assert_eq!(t.gains, [1200]);
    assert_eq!(t.items, [(LETTER, -1)]);
}

#[tokio::test]
async fn mihile_writes_a_dawn_warrior_the_same_letter() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, DAWN_WARRIOR, 30, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, EREVE, "", &mut trace).await, EREVE);
    let npcs = &maps().field(EREVE).unwrap().map_npcs;
    let mihile = npcs.iter().find(|n| n.npc_id == MIHILE).unwrap();
    send(c, walk_to(mihile.x, mihile.y)).await;
    barrier(c, &m, &mut trace).await;
    let start = build_quest_action(SCRIPT_START, DAWN_WARRIORS_LETTER, MIHILE);
    let t = step(c, &m, start).await;
    assert_eq!(t.prompts, [(MIHILE, 1)], "asked");
    step(c, &m, answer(1, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(LETTER, 1)]);
    assert_eq!(t.progress, [(DAWN_WARRIORS_LETTER, 1)]);
}
