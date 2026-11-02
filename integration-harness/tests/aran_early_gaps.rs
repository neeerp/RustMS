//! The early Aran quests that were missing (21700-21767): Lilin's polearm,
//! Puo's Target Pigs and Combo Ability, Lilin's Baby Steps, the reports to
//! Bruce, Rowen and Ilji, the Puppeteer's invitation to Francis's cave and
//! the warning Tru and Lilin act on, and the secret of John's wooden box.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{set_job, set_quest};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const RIEN: i32 = 140000000;
const LILIN: i32 = 1201000;
const PENGUIN_PORT: i32 = 140010100;
const PUO: i32 = 1202006;
const HENESYS: i32 = 100000000;
const BRUCE: i32 = 1012111;
const ELLINIA: i32 = 101000000;
const ROWEN: i32 = 1032101;
const INFO_SHOP: i32 = 104000004;
const TRU: i32 = 1002104;
const SHADOW: i32 = 1204000;
const PUPPETEERS_CAVE: i32 = 910510200;
const FRANCIS: i32 = 1204001;
const DEEP_FOREST_II: i32 = 105040200;
const SLEEPYWOOD: i32 = 105040300;
const ILJI: i32 = 1061019;
const LITH_HARBOR: i32 = 104000000;
const TEO: i32 = 1002001;
const JOHN: i32 = 20000;
const JOHNS_BOX: i32 = 1204033;
const NEW_BEGINNINGS: i16 = 21700;
const TRAIN_OR_DIE_3: i16 = 21703;
const BABY_STEPS: i16 = 21704;
const FIRST_ASSIGNMENT_DONE: i16 = 21712;
const SECOND_ASSIGNMENT_DONE: i16 = 21716;
const PUPPETEERS_INVITATION: i16 = 21719;
const PUPPETEERS_WARNING: i16 = 21720;
const WARNING_RECORD: i16 = 21760;
const FOURTH_ASSIGNMENT_DONE: i16 = 21729;
const SUSPICIOUS_JOHN: i16 = 21766;
const WOODEN_BOX: i16 = 21767;
const PRACTICE_POLEARM: i32 = 1442077;
const COMBO_ABILITY: i32 = 21000000;
const POLEARM_BOOSTER: i32 = 21001003;
const ARTHRITIS_MEDICINE: i32 = 4032423;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// Walk to NPC `npc` on `map` and send a quest action to it.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    send(c, to(map, npc)).await;
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
async fn an_aran_fills_the_early_gaps_from_rien_to_lith_harbor() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2100, 30, 50).await.unwrap();
    for quest_id in [21702, 21711, 21715, 21728] {
        set_quest(id, quest_id, 2, "").await.unwrap();
    }
    set_quest(id, 21706, 1, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Lilin hands over a polearm, and Puo ends New Beginnings.
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        LILIN,
        build_quest_action(SCRIPT_START, NEW_BEGINNINGS, LILIN),
    )
    .await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.prompts, [(LILIN, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&NEW_BEGINNINGS), Some(&1));
    assert_eq!(t.items, [(PRACTICE_POLEARM, 1)]);
    next(c, &m, 1).await;
    assert_eq!(warp(c, PENGUIN_PORT, "", &mut trace).await, PENGUIN_PORT);
    let t = quest(
        c,
        &m,
        PENGUIN_PORT,
        PUO,
        build_quest_completion(NEW_BEGINNINGS, PUO, -1),
    )
    .await;
    assert_eq!(t.states.get(&NEW_BEGINNINGS), Some(&2));
    assert_eq!(t.gains, [100]);

    // Puo sends the Aran after the Target Pigs.
    let t = quest(
        c,
        &m,
        PENGUIN_PORT,
        PUO,
        build_quest_action(SCRIPT_START, TRAIN_OR_DIE_3, PUO),
    )
    .await;
    assert_eq!(t.prompts, [(PUO, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.prompts, [(PUO, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&TRAIN_OR_DIE_3), Some(&1));
    next(c, &m, 1).await;

    // With the pigs beaten, Puo teaches Combo Ability.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, TRAIN_OR_DIE_3, 1, "030").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, PENGUIN_PORT, "", &mut trace).await, PENGUIN_PORT);
    let t = quest(
        c,
        &m,
        PENGUIN_PORT,
        PUO,
        build_quest_action(SCRIPT_END, TRAIN_OR_DIE_3, PUO),
    )
    .await;
    assert_eq!(t.prompts, [(PUO, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.prompts, [(PUO, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&TRAIN_OR_DIE_3), Some(&2));
    assert_eq!(t.items, [(2000022, 30), (2000023, 30)]);
    assert_eq!(t.gains, [2800]);
    assert_eq!(t.skills, [(COMBO_ABILITY, 0)]);
    next(c, &m, 2).await;

    // Baby Steps ends as soon as Lilin has said it.
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        LILIN,
        build_quest_action(SCRIPT_START, BABY_STEPS, LILIN),
    )
    .await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    let t = next(c, &m, 3).await;
    assert_eq!(t.states.get(&BABY_STEPS), Some(&2));
    assert_eq!(t.gains, [500]);

    // Bruce and Rowen take the Aran's reports for Tru.
    for (map, npc, quest_id) in [
        (HENESYS, BRUCE, FIRST_ASSIGNMENT_DONE),
        (ELLINIA, ROWEN, SECOND_ASSIGNMENT_DONE),
    ] {
        assert_eq!(warp(c, map, "", &mut trace).await, map);
        let t = quest(
            c,
            &m,
            map,
            npc,
            build_quest_action(SCRIPT_START, quest_id, npc),
        )
        .await;
        assert_eq!(t.prompts, [(npc, 0)]);
        let t = next(c, &m, 1).await;
        assert_eq!(t.prompts, [(npc, 12)]);
        let t = step(c, &m, answer(12, 1, None)).await;
        assert_eq!(t.states.get(&quest_id), Some(&1));
        next(c, &m, 2).await;
    }
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_completion(SECOND_ASSIGNMENT_DONE, TRU, -1),
    )
    .await;
    assert_eq!(t.states.get(&SECOND_ASSIGNMENT_DONE), Some(&2));

    // A shadow invites the Aran to the Puppeteer's cave, where Francis
    // warns it off and sends it back to the forest.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, PUPPETEERS_INVITATION, SHADOW),
    )
    .await;
    assert_eq!(t.prompts, [(SHADOW, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.prompts, [(SHADOW, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&PUPPETEERS_INVITATION), Some(&1));
    assert_eq!(t.kills[&WARNING_RECORD], "0");
    assert_eq!(t.warps, [PUPPETEERS_CAVE]);
    send(c, to(PUPPETEERS_CAVE, FRANCIS)).await;
    barrier(c, &m, &mut Trace::default()).await;
    let t = step(c, &m, talk(PUPPETEERS_CAVE, FRANCIS)).await;
    assert_eq!(t.prompts, [(FRANCIS, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.states.get(&PUPPETEERS_INVITATION), Some(&2));
    assert_eq!(t.warps, [DEEP_FOREST_II]);

    // Tru hears the warning, and Lilin teaches Polearm Booster for it.
    assert_eq!(warp(c, INFO_SHOP, "", &mut trace).await, INFO_SHOP);
    let t = quest(
        c,
        &m,
        INFO_SHOP,
        TRU,
        build_quest_action(START, PUPPETEERS_WARNING, TRU),
    )
    .await;
    assert_eq!(t.states.get(&PUPPETEERS_WARNING), Some(&1));
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        LILIN,
        build_quest_action(SCRIPT_END, PUPPETEERS_WARNING, LILIN),
    )
    .await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    let t = next(c, &m, 3).await;
    assert_eq!(t.prompts, [(LILIN, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&PUPPETEERS_WARNING), Some(&2));
    assert_eq!(t.gains, [3900]);
    assert_eq!(t.skills, [(POLEARM_BOOSTER, 0)]);
    next(c, &m, 2).await;

    // Ilji takes the fourth report.
    assert_eq!(warp(c, SLEEPYWOOD, "", &mut trace).await, SLEEPYWOOD);
    let t = quest(
        c,
        &m,
        SLEEPYWOOD,
        ILJI,
        build_quest_action(SCRIPT_START, FOURTH_ASSIGNMENT_DONE, ILJI),
    )
    .await;
    assert_eq!(t.prompts, [(ILJI, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.states.get(&FOURTH_ASSIGNMENT_DONE), Some(&1));

    // Teo wonders about John, whose box holds medicine John asks the Aran
    // to keep quiet about; Teo is told something else.
    assert_eq!(warp(c, LITH_HARBOR, "", &mut trace).await, LITH_HARBOR);
    let t = quest(
        c,
        &m,
        LITH_HARBOR,
        TEO,
        build_quest_action(SCRIPT_START, SUSPICIOUS_JOHN, TEO),
    )
    .await;
    assert_eq!(t.prompts, [(TEO, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.states.get(&SUSPICIOUS_JOHN), Some(&1));
    let t = quest(
        c,
        &m,
        LITH_HARBOR,
        JOHNS_BOX,
        build_quest_action(SCRIPT_START, WOODEN_BOX, JOHNS_BOX),
    )
    .await;
    assert_eq!(t.prompts, [(JOHNS_BOX, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.states.get(&WOODEN_BOX), Some(&1));
    assert_eq!(t.items, [(ARTHRITIS_MEDICINE, 1)]);
    let t = quest(
        c,
        &m,
        LITH_HARBOR,
        JOHN,
        build_quest_completion(WOODEN_BOX, JOHN, -1),
    )
    .await;
    assert_eq!(t.states.get(&WOODEN_BOX), Some(&2));
    assert_eq!(t.gains, [250]);
    assert!(t.items.contains(&(ARTHRITIS_MEDICINE, -1)), "{t:?}");
    let t = quest(
        c,
        &m,
        LITH_HARBOR,
        TEO,
        build_quest_action(SCRIPT_END, SUSPICIOUS_JOHN, TEO),
    )
    .await;
    assert_eq!(t.prompts, [(TEO, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.states.get(&SUSPICIOUS_JOHN), Some(&2));
    assert_eq!(t.gains, [200]);
}
