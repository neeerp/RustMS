//! Third job advancement: Tylus sends a level-70 Fighter to Dances with
//! Balrog and the clone's room, then to the Holy Stone's quiz, and makes
//! them a Crusader once their SP is spent.

use game_data::server_content::ServerText;
use game_data::GameData;
use integration_harness::database::{grant_stack, set_job, set_sp};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const TYLUS: i32 = 2020008;
const CHIEFS_RESIDENCE: i32 = 211000001;
const DANCES_WITH_BALROG: i32 = 1022000;
const WARRIORS_SANCTUARY: i32 = 102000003;
const DOOR_OF_DIMENSION: i32 = 1061009;
const DOOR_MAP: i32 = 105070001;
const ENTRY_MAP: i32 = 108010300;
const CLONE_ROOM: i32 = 108010301;
const SPARKLING_CRYSTAL: i32 = 1061010;
const HOLY_STONE: i32 = 2030006;
const HOLY_GROUND: i32 = 211040401;
const BLACK_CHARM: i32 = 4031059;
const NECKLACE_OF_STRENGTH: i32 = 4031057;
const NECKLACE_OF_WISDOM: i32 = 4031058;
const DARK_CRYSTAL: i32 = 4005004;
const ETC: u8 = 4;
const JOB: i32 = 0x20;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// The menu choice of a Holy Stone question that is its right answer. One
/// question's answer can be a wrong choice of another, so find the
/// question first.
fn right_answer(menu: &str) -> i32 {
    let text = ServerText::load(None).unwrap();
    let (question, choices) = menu.split_once("\n#L").expect("a menu");
    let right = (0..)
        .map(|n| format!("holy_stone.q.{n}"))
        .take_while(|key| text.contains(key))
        .find(|key| text.get(key, &[]) == question)
        .map(|key| text.get(&format!("{key}.right"), &[]))
        .expect("a known question");
    format!("#L{choices}")
        .split("#L")
        .skip(1)
        .find_map(|choice| {
            let (index, label) = choice.split_once("##b")?;
            (label.split_once("#k#l")?.0 == right).then(|| index.parse().unwrap())
        })
        .expect("the right answer among the choices")
}

#[tokio::test]
async fn a_fighter_passes_the_tests_and_becomes_a_crusader() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 70, 50).await.expect("level 70 Fighter");

    // Tylus sends the Fighter to Dances with Balrog, who sends them on.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, CHIEFS_RESIDENCE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(CHIEFS_RESIDENCE, TYLUS)).await;
    assert_eq!(t.prompts, [(TYLUS, 4)], "the test or Zakum");
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(TYLUS, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(TYLUS, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    let mut t = Trace::default();
    warp(c, WARRIORS_SANCTUARY, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    // The door lets in one challenger at a time.
    let other_config = load_harness_config_or_fail().await;
    let mut other = login_to_world_session(&other_config).await.unwrap();
    let o = &mut other.connection;
    let mut t = Trace::default();
    warp(o, CLONE_ROOM, "", &mut t).await;
    barrier(o, &other_config.character_name, &mut t).await;

    let mut t = Trace::default();
    warp(c, DOOR_MAP, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(DOOR_MAP, DOOR_OF_DIMENSION)).await;
    assert_eq!(t.prompts, [(DOOR_OF_DIMENSION, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(DOOR_OF_DIMENSION, 0)], "the room is taken");
    assert!(t.arrivals.is_empty());
    step(c, &m, answer(0, 1, None)).await;
    drop(other);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let t = step(c, &m, talk(DOOR_MAP, DOOR_OF_DIMENSION)).await;
    assert_eq!(t.prompts, [(DOOR_OF_DIMENSION, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.arrivals, [(ENTRY_MAP, 0, None)]);

    // The clone's room has twenty minutes, and a crystal back to the door.
    let mut t = Trace::default();
    warp(c, CLONE_ROOM, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    assert_eq!(t.countdowns, [1200]);
    let t = step(c, &m, talk(CLONE_ROOM, SPARKLING_CRYSTAL)).await;
    assert_eq!(t.prompts, [(SPARKLING_CRYSTAL, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(
        t.arrivals.iter().map(|a| a.0).collect::<Vec<_>>(),
        [DOOR_MAP]
    );
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, i16::from(ETC), 20, BLACK_CHARM, 1)
        .await
        .expect("the clone's charm");
    grant_stack(id, i16::from(ETC), 21, DARK_CRYSTAL, 1)
        .await
        .expect("a dark crystal");

    // Balrog trades the charm for the Necklace of Strength, and Tylus
    // takes it for the Holy Stone's assignment.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, WARRIORS_SANCTUARY, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(BLACK_CHARM, -1), (NECKLACE_OF_STRENGTH, 1)]);

    let mut t = Trace::default();
    warp(c, CHIEFS_RESIDENCE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(CHIEFS_RESIDENCE, TYLUS)).await;
    assert_eq!(t.prompts, [(TYLUS, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.items, [(NECKLACE_OF_STRENGTH, -1)]);
    assert_eq!(t.prompts, [(TYLUS, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    // The Holy Stone takes the crystal and asks five questions.
    let mut t = Trace::default();
    warp(c, HOLY_GROUND, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(HOLY_GROUND, HOLY_STONE)).await;
    assert_eq!(t.prompts, [(HOLY_STONE, 1)]);
    let mut t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.items, [(DARK_CRYSTAL, -1)]);
    for _ in 0..5 {
        assert_eq!(t.prompts, [(HOLY_STONE, 4)]);
        let choice = right_answer(&t.prompt_texts[0]);
        t = step(c, &m, answer(4, 1, Some(choice))).await;
    }
    assert_eq!(t.prompts, [(HOLY_STONE, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(NECKLACE_OF_WISDOM, 1)]);
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Unspent SP holds the advance back.
    set_sp(id, 1).await.expect("an SP left");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, CHIEFS_RESIDENCE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(CHIEFS_RESIDENCE, TYLUS)).await;
    assert_eq!(t.prompts, [(TYLUS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.items.is_empty() && t.stat_values(JOB).is_empty());
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Spent, Tylus takes the necklace: a Crusader with 1 SP and 5 AP.
    set_sp(id, 0).await.expect("no SP left");
    let mut session = relogin(&config).await;
    let ap = session.ap;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, CHIEFS_RESIDENCE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(CHIEFS_RESIDENCE, TYLUS)).await;
    assert_eq!(t.prompts, [(TYLUS, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [111]);
    assert_eq!(t.stat_values(stat::SP), [1]);
    assert_eq!(t.stat_values(stat::AP), [i64::from(ap) + 5]);
    assert_eq!(t.items, [(NECKLACE_OF_WISDOM, -1)]);
    assert_eq!(t.prompts, [(TYLUS, 0)]);
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 111);
    for item in [BLACK_CHARM, NECKLACE_OF_STRENGTH, NECKLACE_OF_WISDOM] {
        assert_eq!(after.items.get(&item), None);
    }
}
