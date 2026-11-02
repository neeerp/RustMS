//! The Aran's trail from Mu Lung to Ellin Forest and back (21746-21757):
//! Mu Gong tests the Aran in the back street's arena and sends it after the
//! Shadow Knight, whose shrine opens to the pillar's password; Lilin
//! teaches Final Charge; in Ellin Forest the Gentleman is seen leaving the
//! warehouse with the seal stone, and Athena Pierce keeps a letter for the
//! Aran's own time, which Neinheart is told of.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_quest, set_str, set_weapon_attack,
};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const DOJO: i32 = 925020000;
const BACK_STREET: i32 = 925040000;
const ARENA: i32 = 925040001;
const MU_GONG: i32 = 2091007;
const ARENA_FOE: i32 = 9300350;
const TEMPLE: i32 = 250020300;
const PILLAR: i32 = 2091009;
const SHRINE: i32 = 925040100;
const SHADOW_KNIGHT: i32 = 9300351;
const INFO_SHOP: i32 = 104000004;
const TRU: i32 = 1002104;
const RIEN: i32 = 140000000;
const LILIN: i32 = 1201000;
const ELLINIA_OF_OLD: i32 = 300000000;
const EURYTH: i32 = 2131002;
const CONFERENCE_HALL: i32 = 300000010;
const ATHENA_OF_ELLIN: i32 = 2131000;
const KANDERUN_MAP: i32 = 300010400;
const KANDERUN: i32 = 2132000;
const WAREHOUSE: i32 = 300000011;
const GENTLEMAN: i32 = 9300353;
const BOWMAN_SCHOOL: i32 = 100000201;
const ATHENA_PIERCE: i32 = 1012100;
const EREVE: i32 = 130000000;
const NEINHEART: i32 = 1101002;
const MU_GONGS_TEST: i16 = 21746;
const CATCH_THE_SHADOW_KNIGHT: i16 = 21747;
const MU_LUNG_STONE: i16 = 21748;
const BACK_TO_THE_PAST: i16 = 21749;
const SHE_REMEMBERS_ME: i16 = 21750;
const ELLIN_STONE: i16 = 21751;
const FIND_THE_KEY: i16 = 21752;
const LOST_SEAL_STONE: i16 = 21753;
const ANOTHER_SEAL_STONE: i16 = 21754;
const NEINHEART_THE_STRATEGIST: i16 = 21757;
const WAREHOUSE_RECORD: i16 = 21764;
const ATHENA_FOUND_RECORD: i16 = 21765;
const FINAL_CHARGE: i32 = 21100002;
const KEY_PIECE: i32 = 4032326;
const ATHENAS_LETTER: i32 = 4032328;
const LETTER_TO_THE_EMPRESS: i32 = 4032330;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// Walk to NPC `npc` on `map` and send `action` to it.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    send(c, to(map, npc)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
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

/// Hit the one `mob` in `trace`'s field until it falls.
async fn beat(c: &mut MapleTestConnection, trace: &mut Trace, mob: i32) {
    let id = trace.field.alive(mob)[0];
    for _ in 0..20 {
        send(c, attack(id, 30_000)).await;
        read_for(c, trace, 100).await;
        if trace.field.killed.contains(&id) {
            return;
        }
    }
    panic!("{mob} still standing");
}

#[tokio::test]
async fn an_aran_chases_the_shadow_knight_and_the_gentleman() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2110, 68, 50).await.unwrap();
    replace_item(id, -1, -11, 1442000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    set_quest(id, 21744, 2, "").await.unwrap();
    set_quest(id, 21756, 2, "").await.unwrap();
    grant_stack(id, 4, 5, KEY_PIECE, 10).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // With Mu Gong's business ahead, the dojo's back door leads to Mu Gong.
    assert_eq!(warp(c, DOJO, "", &mut trace).await, DOJO);
    step(c, &m, walk_to(-812, 51)).await;
    let t = step(c, &m, enter("back00")).await;
    assert_eq!(t.warps, [BACK_STREET]);

    // Mu Gong's test: the arena's foe, then the data's end.
    let t = quest(
        c,
        &m,
        BACK_STREET,
        MU_GONG,
        build_quest_action(SCRIPT_START, MU_GONGS_TEST, MU_GONG),
    )
    .await;
    assert_eq!(t.prompts, [(MU_GONG, 0)]);
    send(c, answer(0, 1, None)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [ARENA] && t.field.alive(ARENA_FOE).len() == 1
    })
    .await;
    assert_eq!(trace.states.get(&MU_GONGS_TEST), Some(&1));
    beat(c, &mut trace, ARENA_FOE).await;
    assert_eq!(trace.kills[&MU_GONGS_TEST], "001");
    step(c, &m, walk_to(-393, 8)).await;
    let t = step(c, &m, enter("out00")).await;
    assert_eq!(t.warps, [BACK_STREET]);
    let t = quest(
        c,
        &m,
        BACK_STREET,
        MU_GONG,
        build_quest_completion(MU_GONGS_TEST, MU_GONG, -1),
    )
    .await;
    assert_eq!(t.states.get(&MU_GONGS_TEST), Some(&2));
    assert_eq!(t.gains, [5000]);

    // After the Shadow Knight, with the pillar's password.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, CATCH_THE_SHADOW_KNIGHT, MU_GONG),
    )
    .await;
    assert_eq!(t.prompts, [(MU_GONG, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&CATCH_THE_SHADOW_KNIGHT), Some(&1));
    step(c, &m, answer(0, 1, None)).await;

    // On it, the dojo's back door opens.
    let mut trace = Trace::default();
    assert_eq!(warp(c, DOJO, "", &mut trace).await, DOJO);
    step(c, &m, walk_to(-812, 51)).await;
    let t = step(c, &m, enter("back00")).await;
    assert_eq!(t.warps, [BACK_STREET]);

    assert_eq!(warp(c, TEMPLE, "", &mut trace).await, TEMPLE);
    let t = quest(c, &m, TEMPLE, PILLAR, talk(TEMPLE, PILLAR)).await;
    assert_eq!(t.prompts, [(PILLAR, 2)]);
    let t = step(c, &m, type_text("Words speak louder than actions")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    send(c, answer(0, 1, None)).await;
    let t = quest(c, &m, TEMPLE, PILLAR, talk(TEMPLE, PILLAR)).await;
    assert_eq!(t.prompts, [(PILLAR, 2)]);
    send(c, type_text("Actions speak louder than words.")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [SHRINE] && t.field.alive(SHADOW_KNIGHT).len() == 1
    })
    .await;
    beat(c, &mut trace, SHADOW_KNIGHT).await;
    assert_eq!(trace.kills[&CATCH_THE_SHADOW_KNIGHT], "001");

    assert_eq!(warp(c, BACK_STREET, "", &mut trace).await, BACK_STREET);
    let t = quest(
        c,
        &m,
        BACK_STREET,
        MU_GONG,
        build_quest_action(SCRIPT_END, CATCH_THE_SHADOW_KNIGHT, MU_GONG),
    )
    .await;
    assert_eq!(t.prompts, [(MU_GONG, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&CATCH_THE_SHADOW_KNIGHT), Some(&2));
    assert_eq!(t.gains, [16000]);

    // Mu Gong's business done, the dojo's back door stays shut.
    assert_eq!(warp(c, DOJO, "", &mut trace).await, DOJO);
    step(c, &m, walk_to(-812, 51)).await;
    let t = step(c, &m, enter("back00")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.broadcasts.len(), 1, "told in pink: {:?}", t.broadcasts);

    // Tru hears of it and sends the Aran to Lilin, who teaches Final Charge.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(START, MU_LUNG_STONE, TRU),
    )
    .await;
    assert_eq!(t.states.get(&MU_LUNG_STONE), Some(&1));
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        LILIN,
        build_quest_action(SCRIPT_END, MU_LUNG_STONE, LILIN),
    )
    .await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&MU_LUNG_STONE), Some(&2));
    assert_eq!(t.gains, [20000]);
    assert_eq!(t.skills, [(FINAL_CHARGE, 0)]);

    // Back to the past, which Tru ends.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(SCRIPT_START, BACK_TO_THE_PAST, TRU),
    )
    .await;
    assert_eq!(t.prompts, [(TRU, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&BACK_TO_THE_PAST), Some(&1));
    let t = step(c, &m, build_quest_completion(BACK_TO_THE_PAST, TRU, -1)).await;
    assert_eq!(t.states.get(&BACK_TO_THE_PAST), Some(&2));
    assert_eq!(t.gains, [500]);

    // Athena Pierce of the past remembers the Aran, and the stone's key
    // is found.
    assert_eq!(
        warp(c, ELLINIA_OF_OLD, "", &mut trace).await,
        ELLINIA_OF_OLD
    );
    let t = quest(
        c,
        &m,
        ELLINIA_OF_OLD,
        EURYTH,
        build_quest_action(START, SHE_REMEMBERS_ME, EURYTH),
    )
    .await;
    assert_eq!(t.states.get(&SHE_REMEMBERS_ME), Some(&1));
    assert_eq!(
        warp(c, CONFERENCE_HALL, "", &mut trace).await,
        CONFERENCE_HALL
    );
    let t = quest(
        c,
        &m,
        CONFERENCE_HALL,
        ATHENA_OF_ELLIN,
        build_quest_action(SCRIPT_END, SHE_REMEMBERS_ME, ATHENA_OF_ELLIN),
    )
    .await;
    assert_eq!(t.prompts, [(ATHENA_OF_ELLIN, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&SHE_REMEMBERS_ME), Some(&2));
    let t = step(
        c,
        &m,
        build_quest_action(START, ELLIN_STONE, ATHENA_OF_ELLIN),
    )
    .await;
    assert_eq!(t.states.get(&ELLIN_STONE), Some(&1));

    // The warehouse holds nothing yet.
    let mut trace = Trace::default();
    step(c, &m, walk_to(334, -159)).await;
    let t = step(c, &m, enter("in00")).await;
    assert_eq!(t.warps, [WAREHOUSE]);
    read_for(c, &mut trace, 500).await;
    assert!(t.field.alive(GENTLEMAN).is_empty() && trace.field.alive(GENTLEMAN).is_empty());
    assert!(t.kills.get(&WAREHOUSE_RECORD).is_none(), "{t:?}");

    assert_eq!(warp(c, KANDERUN_MAP, "", &mut trace).await, KANDERUN_MAP);
    let t = quest(
        c,
        &m,
        KANDERUN_MAP,
        KANDERUN,
        build_quest_action(START, FIND_THE_KEY, KANDERUN),
    )
    .await;
    assert_eq!(t.states.get(&FIND_THE_KEY), Some(&1));
    let t = step(c, &m, build_quest_completion(FIND_THE_KEY, KANDERUN, -1)).await;
    assert_eq!(t.states.get(&FIND_THE_KEY), Some(&2));
    assert_eq!(
        warp(c, CONFERENCE_HALL, "", &mut trace).await,
        CONFERENCE_HALL
    );
    let t = quest(
        c,
        &m,
        CONFERENCE_HALL,
        ATHENA_OF_ELLIN,
        build_quest_completion(ELLIN_STONE, ATHENA_OF_ELLIN, -1),
    )
    .await;
    assert_eq!(t.states.get(&ELLIN_STONE), Some(&2));
    assert_eq!(t.gains, [56800]);

    // Now the Gentleman is caught leaving with it.
    step(c, &m, walk_to(334, -159)).await;
    send(c, enter("in00")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [WAREHOUSE] && t.field.alive(GENTLEMAN).len() == 1
    })
    .await;
    assert_eq!(trace.kills[&WAREHOUSE_RECORD], "1");

    // Athena Pierce keeps a letter, which Tru ends with 20,000 EXP.
    assert_eq!(
        warp(c, CONFERENCE_HALL, "", &mut trace).await,
        CONFERENCE_HALL
    );
    let t = quest(
        c,
        &m,
        CONFERENCE_HALL,
        ATHENA_OF_ELLIN,
        build_quest_action(SCRIPT_START, LOST_SEAL_STONE, ATHENA_OF_ELLIN),
    )
    .await;
    assert_eq!(t.prompts, [(ATHENA_OF_ELLIN, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&LOST_SEAL_STONE), Some(&1));
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_completion(LOST_SEAL_STONE, TRU, -1),
    )
    .await;
    assert_eq!(t.states.get(&LOST_SEAL_STONE), Some(&2));
    assert_eq!(t.gains, [20000]);

    // In the Aran's own time, Athena Pierce hands it over.
    let mut trace = Trace::default();
    assert_eq!(warp(c, BOWMAN_SCHOOL, "", &mut trace).await, BOWMAN_SCHOOL);
    read_for(c, &mut trace, 300).await;
    assert_eq!(trace.kills[&ATHENA_FOUND_RECORD], "2");
    let t = quest(
        c,
        &m,
        BOWMAN_SCHOOL,
        ATHENA_PIERCE,
        build_quest_action(SCRIPT_START, ANOTHER_SEAL_STONE, ATHENA_PIERCE),
    )
    .await;
    assert_eq!(t.prompts, [(ATHENA_PIERCE, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&ANOTHER_SEAL_STONE), Some(&1));
    assert_eq!(t.items, [(ATHENAS_LETTER, 1)]);
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_completion(ANOTHER_SEAL_STONE, TRU, -1),
    )
    .await;
    assert_eq!(t.states.get(&ANOTHER_SEAL_STONE), Some(&2));
    assert_eq!(t.items, [(ATHENAS_LETTER, -1)]);
    assert_eq!(t.gains, [56800]);

    // Lilin's letter, for Neinheart.
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        LILIN,
        build_quest_action(START, NEINHEART_THE_STRATEGIST, LILIN),
    )
    .await;
    assert_eq!(t.states.get(&NEINHEART_THE_STRATEGIST), Some(&1));
    assert_eq!(t.items, [(LETTER_TO_THE_EMPRESS, 1)]);
    assert_eq!(warp(c, EREVE, "", &mut trace).await, EREVE);
    let t = quest(
        c,
        &m,
        EREVE,
        NEINHEART,
        build_quest_action(SCRIPT_END, NEINHEART_THE_STRATEGIST, NEINHEART),
    )
    .await;
    assert_eq!(t.prompts, [(NEINHEART, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&NEINHEART_THE_STRATEGIST), Some(&2));
    assert_eq!(t.items, [(LETTER_TO_THE_EMPRESS, -1)]);
    assert_eq!(t.gains, [1000]);
}
