//! The way to Zakum: a level-50 Fighter earns Tylus's approval, crosses
//! the Breath of Lava on the Dead Mine's Fire Ore, has Adobis forge the
//! Eyes of Fire, and walks through the gate, which turns them back until
//! then.

use game_data::GameData;
use integration_harness::database::{grant_stack, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const TYLUS: i32 = 2020008;
const CHIEFS_RESIDENCE: i32 = 211000001;
const ADOBIS: i32 = 2030008;
const DOOR_TO_ZAKUM: i32 = 211042300;
const ENTRANCE_TO_ZAKUM: i32 = 211042400;
const BREATH_OF_LAVA: i32 = 280020000;
const LAVA_END: i32 = 280020001;
const LIRA: i32 = 2032003;
const FIRE_ORE: i32 = 4031061;
const BREATH_OF_FIRE: i32 = 4031062;
const GOLD_TOOTH: i32 = 4000082;
const EYE_OF_FIRE: i32 = 4001017;
const ETC: i16 = 4;
const PINK: u8 = 5;

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

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn pink(trace: &Trace) -> bool {
    matches!(trace.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty())
}

#[tokio::test]
async fn a_fighter_is_approved_forges_the_eyes_and_passes_the_gate() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 50, 50).await.expect("level 50 Fighter");
    grant_stack(id, ETC, 20, FIRE_ORE, 1)
        .await
        .expect("the Dead Mine's Fire Ore");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // Unapproved, the gate stays shut.
    let mut t = Trace::default();
    warp(c, DOOR_TO_ZAKUM, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, enter("ps00")).await;
    assert!(pink(&t) && t.arrivals.is_empty(), "{t:?}");

    // Tylus approves a Fighter of 50 at once.
    let mut t = Trace::default();
    warp(c, CHIEFS_RESIDENCE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(CHIEFS_RESIDENCE, TYLUS)).await;
    assert_eq!(t.prompts, [(TYLUS, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    // Adobis sends the Fire Ore's holder into the Breath of Lava.
    let mut t = Trace::default();
    warp(c, DOOR_TO_ZAKUM, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(DOOR_TO_ZAKUM, ADOBIS)).await;
    assert_eq!(t.prompts, [(ADOBIS, 4)]);
    let t = step(c, &m, answer(4, 1, Some(1))).await;
    assert_eq!(t.prompts, [(ADOBIS, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.arrivals, [(BREATH_OF_LAVA, 0, None)]);

    // Lira, at its end, gives the Breath of Fire and sends them back.
    let mut t = Trace::default();
    warp(c, LAVA_END, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(LAVA_END, LIRA)).await;
    assert_eq!(t.prompts, [(LIRA, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(BREATH_OF_FIRE, 1)]);
    assert_eq!(
        t.arrivals.iter().map(|a| a.0).collect::<Vec<_>>(),
        [DOOR_TO_ZAKUM]
    );

    // Not yet through the trials: the gate stays shut.
    let mut t = Trace::default();
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, enter("ps00")).await;
    assert!(pink(&t) && t.arrivals.is_empty(), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 21, GOLD_TOOTH, 30)
        .await
        .expect("30 Gold Teeth");

    // Adobis forges five Eyes of Fire, and the gate opens.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, DOOR_TO_ZAKUM, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    step(c, &m, talk(DOOR_TO_ZAKUM, ADOBIS)).await;
    let t = step(c, &m, answer(4, 1, Some(2))).await;
    assert_eq!(
        t.items,
        [
            (FIRE_ORE, -1),
            (BREATH_OF_FIRE, -1),
            (GOLD_TOOTH, -30),
            (EYE_OF_FIRE, 5)
        ]
    );
    assert_eq!(t.prompts, [(ADOBIS, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, enter("ps00")).await;
    assert_eq!(
        t.arrivals.iter().map(|a| a.0).collect::<Vec<_>>(),
        [ENTRANCE_TO_ZAKUM]
    );
}
