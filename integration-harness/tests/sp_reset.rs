//! SP Reset: one skill level moves to a skill of the item's advancement and
//! the item is used up; a move the skills don't allow keeps it.

use integration_harness::database::{grant_skill, grant_stack, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const CASH: i16 = 5;
const FIRST_JOB_RESET: i32 = 5_050_001;
const POWER_STRIKE: i32 = 1_001_004;
const SLASH_BLAST: i32 = 1_001_005;

fn reset(to: i32, from: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(FIRST_JOB_RESET).unwrap();
    p.write_int(to).unwrap();
    p.write_int(from).unwrap();
    p.write_int(0).unwrap();
    p
}

#[tokio::test]
async fn an_sp_reset_moves_a_skill_level() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 20, 100).await.expect("a warrior");
    grant_skill(id, POWER_STRIKE, 20)
        .await
        .expect("power strike");
    grant_skill(id, SLASH_BLAST, 1).await.expect("slash blast");
    grant_stack(id, CASH, 1, FIRST_JOB_RESET, 2)
        .await
        .expect("grant SP Resets");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // Power Strike is full: refused, and only the client is unlocked.
    let t = step(c, &m, reset(POWER_STRIKE, SLASH_BLAST)).await;
    assert_eq!(t.stat_updates, [Default::default()]);
    assert!(t.skills.is_empty() && t.quantities.is_empty());

    let t = step(c, &m, reset(SLASH_BLAST, POWER_STRIKE)).await;
    assert_eq!(t.skills, [(SLASH_BLAST, 2), (POWER_STRIKE, 19)]);
    assert_eq!(t.quantities, [(CASH as u8, 1, 1)]);

    let t = step(c, &m, reset(POWER_STRIKE, SLASH_BLAST)).await;
    assert_eq!(t.skills, [(POWER_STRIKE, 20), (SLASH_BLAST, 1)]);
    assert_eq!(t.removed, [(CASH as u8, 1)]);

    // None left.
    let t = step(c, &m, reset(SLASH_BLAST, POWER_STRIKE)).await;
    assert_eq!(t.stat_updates, [Default::default()]);
}
