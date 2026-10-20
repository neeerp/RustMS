//! Shadow Stars costs 200 of the star the client names, summed over its
//! stacks, and its buff then throws that star from a claw for nothing:
//! the shot carries display bit 0x40 and the star after its ammo slot.

use integration_harness::database::{
    grant_skill, grant_stack, replace_item, set_job, set_weapon_attack,
};
use integration_harness::hunting::{ranged_attack, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const SHADOW_STARS: i32 = 4121006;
const GARNIER: i32 = 1472000;
const SUBI: i32 = 2070000;
const ILBI: i32 = 2070006;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const USE: i16 = 2;

/// A shot under Shadow Stars from `slot`, naming `star`.
fn starred_shot(slot: i16, star: i32, snail: i32, damage: i32) -> Packet {
    let mut p = ranged_attack(0, slot, snail, damage);
    p.bytes[16] |= 0x40;
    p.bytes.splice(30..30, star.to_le_bytes());
    p
}

#[tokio::test]
async fn shadow_stars_costs_200_stars_then_throws_them_for_nothing() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 412, 120, 1000).await.expect("night lord");
    grant_skill(id, SHADOW_STARS, 30)
        .await
        .expect("Shadow Stars");
    replace_item(id, EQUIPPED, WEAPON, GARNIER)
        .await
        .expect("give a claw");
    set_weapon_attack(id, 10).await.expect("claw attack");
    for (slot, star, count) in [(1, SUBI, 150), (2, ILBI, 100), (3, ILBI, 150)] {
        grant_stack(id, USE, slot, star, count)
            .await
            .expect("give stars");
    }

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];

    // 150 Subis aren't enough.
    let mut cast = use_skill(SHADOW_STARS, 30);
    cast.write_int(SUBI).unwrap();
    let t = step(c, "subis", cast).await;
    assert!(t.buffs.is_empty(), "refused: {:?}", t.buffs);

    // 250 Ilbis are: the first stack goes, the second keeps 50.
    let mut cast = use_skill(SHADOW_STARS, 30);
    cast.write_int(ILBI).unwrap();
    let t = step(c, "ilbis", cast).await;
    assert_eq!(t.buffs.len(), 1, "given: {:?}", t.buffs);
    assert_eq!((t.buffs[0].0, t.buffs[0].1), (SHADOW_STARS, 7));
    assert_eq!(t.removed, [(2, 2)]);
    assert_eq!(t.quantities, [(2, 3, 50)]);

    // The shot must come from a slot of Ilbis, and costs none.
    let t = step(c, "subi shot", starred_shot(1, ILBI, snail, 8)).await;
    assert!(t.field.killed.is_empty(), "refused from the Subis");
    let t = step(c, "ilbi shot", starred_shot(3, ILBI, snail, 8)).await;
    assert!(t.field.killed.contains(&snail), "the shot lands");
    assert!(
        t.quantities.is_empty() && t.removed.is_empty(),
        "and costs nothing"
    );
}
