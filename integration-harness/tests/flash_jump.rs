//! Flash Jump (4111006): the movement is the client's, so the server only
//! spends the level's MP (60 at level 1, 13 at level 20), with no buff and
//! no cooldown. A level not learned, a skill not learned or too little MP
//! is refused and spends nothing.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// A map no other test uses, without the movement-skill field limit.
const FIELD: i32 = 1_000_002;
const HERMIT: i16 = 411;
const FLASH_JUMP: i32 = 4_111_006;
const NIGHT_WALKERS_FLASH_JUMP: i32 = 14_101_004;
const COST_AT_20: i64 = 13;

/// USE_SKILL as the client sends Flash Jump: tick, skill, level and a
/// trailing zero short (report 194-flash-jump).
fn flash_jump(skill: i32, level: u8) -> Packet {
    let mut p = use_skill(skill, level);
    p.write_short(0).unwrap();
    p
}

/// A Hermit with Flash Jump at level 20 and `mp` MP, on the test map.
async fn hermit(mp: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, HERMIT, 70, mp).await.expect("hermit");
    grant_skill(id, FLASH_JUMP, 20).await.expect("Flash Jump");

    let mut session = relogin(&config).await;
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut session.connection, FIELD, "", &mut t).await,
        FIELD
    );
    session
}

/// Sends a Flash Jump and checks it gave no buff or cooldown; the MP it
/// left, if any. Both tests share the map, so each has its own `marker`
/// (a barrier ends on anyone's chat).
async fn jump(c: &mut MapleTestConnection, marker: &str, skill: i32, level: u8) -> Vec<i64> {
    let t = step(c, marker, flash_jump(skill, level)).await;
    assert!(t.buffs.is_empty(), "Flash Jump gave a buff");
    assert!(t.cooldowns.is_empty(), "Flash Jump started a cooldown");
    t.stat_values(stat::MP)
}

#[tokio::test]
async fn each_flash_jump_spends_its_levels_mp_and_nothing_else() {
    let mut session = hermit(1000).await;
    let c = &mut session.connection;
    const MARK: &str = "flash jumps";

    assert_eq!(jump(c, MARK, FLASH_JUMP, 20).await, [1000 - COST_AT_20]);
    assert_eq!(jump(c, MARK, FLASH_JUMP, 20).await, [1000 - 2 * COST_AT_20]);

    // A level other than the learned one, or another class's Flash Jump, spends
    // nothing; the next real jump shows the MP is where it was.
    assert!(jump(c, MARK, FLASH_JUMP, 19).await.is_empty());
    assert!(jump(c, MARK, NIGHT_WALKERS_FLASH_JUMP, 1).await.is_empty());
    assert_eq!(jump(c, MARK, FLASH_JUMP, 20).await, [1000 - 3 * COST_AT_20]);
}

#[tokio::test]
async fn a_flash_jump_without_its_mp_is_refused() {
    let mut session = hermit(10).await;
    let c = &mut session.connection;
    const MARK: &str = "flash jump without mp";

    assert!(jump(c, MARK, FLASH_JUMP, 20).await.is_empty());
}
