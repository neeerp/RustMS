//! Amoria's engagement ring (report b064): Moony sends a groom to his
//! parents in "Meet the Parents", who want four different Proofs of Love,
//! which the Nanas of the towns give for 50 of their town's drop. With the
//! parents' blessing, Moony makes a ring box for its stones and fee.

use game_data::GameData;
use integration_harness::database::{grant_stack, set_gender, set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const AMORIA: i32 = 680_000_000;
const MEET_THE_PARENTS: i32 = 680_000_004;
const MOONY: i32 = 9201000;
const MOM_AND_DAD: i32 = 9201003;
/// Four of the Nanas: (town, Nana, her drop, the EXP of her first proof,
/// her proof).
const NANAS: [(i32, i32, i32, i32, i32); 4] = [
    (100_000_000, 9201001, 4000001, 2000, 4031367),
    (103_000_000, 9201023, 4000037, 5000, 4031368),
    (102_000_000, 9201027, 4000215, 10000, 4031369),
    (101_000_000, 9201024, 4000026, 17000, 4031370),
];
/// The Silver Swan ring box, its stone, and the diamond every box takes.
const SILVER_SWAN_BOX: i32 = 2240003;
const SILVER: i32 = 4011004;
const DIAMOND: i32 = 4021007;
const SILVER_SWAN_FEE: i32 = 5000;
const ETC: i16 = 4;

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
        .unwrap_or_else(|| panic!("NPC {npc} on map {map}"));
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

#[tokio::test]
async fn a_groom_earns_the_blessing_and_buys_a_ring_box_from_moony() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.expect("job");
    set_gender(id, 0).await.expect("gender");
    set_meso(id, SILVER_SWAN_FEE).await.expect("mesos");
    for (slot, &(_, _, drop, ..)) in NANAS.iter().enumerate() {
        grant_stack(id, ETC, slot as i16 + 1, drop, 50)
            .await
            .expect("drops");
    }
    grant_stack(id, ETC, 5, SILVER, 1).await.expect("stone");
    grant_stack(id, ETC, 6, DIAMOND, 1).await.expect("diamond");
    let mut session = relogin(&config).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let mut t = Trace::default();
    let moony = talk(AMORIA, MOONY);

    // Moony sends the player to the parents, and while they are sought,
    // only says so.
    assert_eq!(warp(c, AMORIA, "", &mut t).await, AMORIA);
    let t = step(c, &format!("{m} moony"), moony.clone()).await;
    assert_eq!(t.prompts, [(MOONY, 4)]);
    let t = step(c, &format!("{m} make"), answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MOONY, 0)]);
    let t = step(c, &format!("{m} ok"), answer(0, 1, None)).await;
    assert_eq!(t.prompts, []);
    step(c, &format!("{m} moony"), moony.clone()).await;
    let t = step(c, &format!("{m} make"), answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MOONY, 0)]);
    assert!(!t.prompt_texts[0].contains("#L"));
    step(c, &format!("{m} ok"), answer(0, 1, None)).await;

    // The parents ask for four Proofs of Love, and wait for them.
    let parents = talk(MEET_THE_PARENTS, MOM_AND_DAD);
    let mut t = Trace::default();
    assert_eq!(
        warp(c, MEET_THE_PARENTS, "", &mut t).await,
        MEET_THE_PARENTS
    );
    let t = step(c, &format!("{m} parents"), parents.clone()).await;
    assert_eq!(t.prompts, [(MOM_AND_DAD, 0)]);
    for _ in 0..3 {
        step(c, &format!("{m} next"), answer(0, 1, None)).await;
    }
    let t = step(c, &format!("{m} parents"), parents.clone()).await;
    assert_eq!(t.prompts, [(MOM_AND_DAD, 0)]);
    step(c, &format!("{m} ok"), answer(0, 1, None)).await;

    // Each Nana's favour, then her proof for the drops, with EXP.
    for (town, nana, drop, exp, proof) in NANAS {
        let mut t = Trace::default();
        assert_eq!(warp(c, town, "", &mut t).await, town);
        let t = step(c, &format!("{m} nana"), talk(town, nana)).await;
        assert_eq!(t.prompts, [(nana, 1)]);
        let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
        assert_eq!(t.prompts, [(nana, 0)]);
        assert!(t.prompt_texts[0].contains(&format!("#t{drop}#")));
        step(c, &format!("{m} ok"), answer(0, 1, None)).await;
        let t = step(c, &format!("{m} nana"), talk(town, nana)).await;
        assert_eq!(t.prompts, [(nana, 0)]);
        assert_eq!(t.gains, [exp]);
        assert_eq!(t.items, [(drop, -50), (proof, 1)]);
        step(c, &format!("{m} ok"), answer(0, 1, None)).await;
    }

    // The parents take the four proofs and bless the player.
    let mut t = Trace::default();
    assert_eq!(
        warp(c, MEET_THE_PARENTS, "", &mut t).await,
        MEET_THE_PARENTS
    );
    let t = step(c, &format!("{m} parents"), parents).await;
    assert_eq!(t.prompts, [(MOM_AND_DAD, 0)]);
    let t = step(c, &format!("{m} next"), answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(MOM_AND_DAD, 0)]);
    assert_eq!(t.gains, [20_000]);
    let taken: Vec<_> = NANAS.iter().map(|&(.., proof)| (proof, -1)).collect();
    assert_eq!(t.items, taken);
    step(c, &format!("{m} ok"), answer(0, 1, None)).await;
    drop(session);

    // The blessing outlives a relogin; Moony makes the box for its stones
    // and fee, one at a time, and throws it away.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, AMORIA, "", &mut t).await, AMORIA);
    step(c, &format!("{m} moony"), moony.clone()).await;
    let t = step(c, &format!("{m} make"), answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MOONY, 4)]);
    assert_eq!(t.prompt_texts[0].matches("#L").count(), 4);
    let t = step(c, &format!("{m} silver swan"), answer(4, 1, Some(3))).await;
    assert_eq!(t.prompts, [(MOONY, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(MOONY, 0)]);
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.items, [(SILVER, -1), (DIAMOND, -1), (SILVER_SWAN_BOX, 1)]);
    step(c, &format!("{m} ok"), answer(0, 1, None)).await;

    step(c, &format!("{m} moony"), moony.clone()).await;
    let t = step(c, &format!("{m} make"), answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MOONY, 0)], "one box at a time");
    step(c, &format!("{m} ok"), answer(0, 1, None)).await;

    step(c, &format!("{m} moony"), moony).await;
    let t = step(c, &format!("{m} discard"), answer(4, 1, Some(1))).await;
    assert_eq!(t.prompts, [(MOONY, 0)]);
    assert_eq!(t.items, [(SILVER_SWAN_BOX, -1)]);
}
