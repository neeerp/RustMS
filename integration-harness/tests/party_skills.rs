//! Skills that neither buff nor debuff, on a mob-free path in Ellinia
//! (101020001): a Bishop's Resurrection raises a dead party member with
//! full HP, and a Chief Bandit's Chakra heals them by their LUK.

use integration_harness::database::{grant_skill, set_hp, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const FIELD: i32 = 101_020_001;
const BISHOP: i16 = 232;
const RESURRECTION: i32 = 2321006;
const CHIEF_BANDIT: i16 = 421;
const CHAKRA: i32 = 4211001;
const OP_JOIN: u8 = 0x0F;
/// STAT_CHANGED's HP bit.
const HP: i32 = 0x400;
/// The second party slot's bit in USE_SKILL's party mask.
const SECOND_SLOT: u8 = 0x10;

fn party_operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartyOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

fn hp(trace: &Trace) -> Option<i64> {
    trace
        .stat_updates
        .iter()
        .rev()
        .find_map(|update| update.get(&HP).copied())
}

#[tokio::test]
async fn resurrection_raises_the_dead_and_chakra_heals() {
    let config = load_harness_config_or_fail().await;
    let bishop_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    let dead_id = login_to_world_session(&other_config)
        .await
        .expect("second login failed")
        .character_id;
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(bishop_id, BISHOP, 120, 5000)
        .await
        .expect("make a bishop");
    grant_skill(bishop_id, RESURRECTION, 1)
        .await
        .expect("teach Resurrection");
    set_job(dead_id, CHIEF_BANDIT, 70, 500)
        .await
        .expect("make a chief bandit");
    grant_skill(dead_id, CHAKRA, 30)
        .await
        .expect("teach Chakra");
    // Both go to the field, and one dies there.
    for config in [&config, &other_config] {
        let mut session = relogin(config).await;
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut session.connection, FIELD, "", &mut trace).await,
            FIELD
        );
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(dead_id, 0, 3000).await.expect("kill them");

    let mut bishop = relogin(&config).await;
    let mut dead = relogin(&other_config).await;
    let (b, d) = (&mut bishop.connection, &mut dead.connection);
    let (mut bt, mut dt) = (Trace::default(), Trace::default());
    read_for(b, &mut bt, 300).await;

    let mut invite = party_operation(4);
    invite.write_str_with_length(&dead.character_name).unwrap();
    send(b, invite).await;
    until(d, &mut dt, |t| !t.party_invites.is_empty()).await;
    let mut accept = party_operation(3);
    accept.write_int(dt.party_invites[0].0).unwrap();
    send(d, accept).await;
    until(b, &mut bt, |t| t.party_ops.contains(&OP_JOIN)).await;

    // Resurrection raises the dead member it reached with full HP.
    dt.stat_updates.clear();
    let mut raise = use_skill(RESURRECTION, 1);
    raise.write_byte(SECOND_SLOT).unwrap();
    send(b, raise).await;
    until(d, &mut dt, |t| hp(t).is_some()).await;
    assert_eq!(hp(&dt), Some(3000));
    until(b, &mut bt, |t| {
        t.cooldowns.iter().any(|&(id, _)| id == RESURRECTION)
    })
    .await;

    // Chakra heals from 2.3 to 3.5 times LUK times its y percent (300).
    drop(bishop);
    drop(dead);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(dead_id, 100, 3000).await.expect("hurt them");
    let mut bandit = relogin(&other_config).await;
    let c = &mut bandit.connection;
    let mut ct = Trace::default();
    read_for(c, &mut ct, 300).await;
    ct.stat_updates.clear();
    send(c, use_skill(CHAKRA, 30)).await;
    until(c, &mut ct, |t| hp(t).is_some()).await;
    let healed = hp(&ct).unwrap() - 100;
    assert!((1..=3000).contains(&healed), "Chakra healed {healed}");
}
