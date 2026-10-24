//! Reporting a player (CLAIM, CLAIM_RESULT): the client is told on entry that
//! reports are taken all day; a report costs 300 mesos, names someone else,
//! and is kept, and the reply says how many are left this week.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_meso;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// STAT_CHANGED's mask for mesos alone.
const MESO_MASK: [u8; 4] = 0x40000i32.to_le_bytes();

/// A conversation report of `victim`, for swearing (reason 1).
fn report(victim: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::Claim as i16).unwrap();
    p.write_byte(1).unwrap();
    p.write_str_with_length(victim).unwrap();
    p.write_byte(1).unwrap();
    p.write_str_with_length("said bad words").unwrap();
    p.write_str_with_length(&format!("{victim} : bad words"))
        .unwrap();
    p
}

/// The body of the next CLAIM_RESULT, and the mesos of any stat change
/// before it.
async fn result(c: &mut MapleTestConnection) -> (Vec<u8>, Option<i32>) {
    let mut t = Trace::default();
    let mut meso = None;
    loop {
        let p = read(c, &mut t).await;
        if p.opcode() == SendOpcode::StatChange as i16 && p.bytes[3..7] == MESO_MASK {
            meso = Some(i32::from_le_bytes(p.bytes[7..11].try_into().unwrap()));
        }
        if p.opcode() == SendOpcode::ClaimResult as i16 {
            return (p.bytes[2..].to_vec(), meso);
        }
    }
}

#[tokio::test]
async fn a_report_costs_300_mesos_and_is_kept() {
    let reporter_config = load_harness_config_or_fail().await;
    let victim_config = load_harness_config_or_fail().await;

    let reporter_id = login_to_world_session(&reporter_config)
        .await
        .expect("first login failed")
        .character_id;
    let victim = login_to_world_session(&victim_config)
        .await
        .expect("victim login failed");
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(reporter_id, 0).await.expect("empty the purse");

    // Entry says reports are taken, from 0 to 0 o'clock: all day.
    let mut reporter = relogin(&reporter_config).await;
    assert_eq!(reporter.claims.0, [0, 0], "all day");
    assert_eq!(reporter.claims.1, [1], "reports are taken");
    let c = &mut reporter.connection;
    let mut t = Trace::default();

    // Without 300 mesos the report isn't made.
    send(c, report(&victim.character_name)).await;
    assert_eq!(result(c).await, (vec![0x43], None), "not enough mesos");

    drop(reporter);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(reporter_id, 1_000).await.expect("fill the purse");
    let mut reporter = relogin(&reporter_config).await;
    let c = &mut reporter.connection;

    // A name nobody has, or the reporter's own, is to be re-checked.
    let own = reporter.character_name.clone();
    for name in ["NoSuchVictim", own.as_str()] {
        send(c, report(name)).await;
        assert_eq!(result(c).await, (vec![0x42], None), "{name}");
    }

    // The victim's name, in any case, is kept for 300 mesos, leaving 9.
    send(c, report(&victim.character_name.to_uppercase())).await;
    assert_eq!(
        result(c).await,
        (vec![2, 1, 9, 0, 0, 0], Some(700)),
        "kept, with 9 left this week"
    );
    send(c, report(&victim.character_name)).await;
    assert_eq!(result(c).await, (vec![2, 1, 8, 0, 0, 0], Some(400)));
}
