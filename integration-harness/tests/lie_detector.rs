//! The Lie Detector (2190000, report c025): used on a character on the
//! user's map who has just attacked, it shows them a picture to read
//! within a minute and uses the item up. A wrong answer fails them, and
//! up to 7000 of their mesos go to the tester; for an hour after, they
//! can't be tested again.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_meso};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const LIE_DETECTOR: i32 = 2_190_000;
/// Henesys Hunting Ground I, not a town.
const HUNTING_GROUND: i32 = 100_010_000;
/// MESO's flag in a stat update.
const MESO: i32 = 0x40000;

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("lie detector"))
            .await
            .expect("timed out")
            .expect("read packet")
            .packet;
        if let Some(picked) = pick(&p) {
            return picked;
        }
    }
}

/// The body of the next ANTI_MACRO_RESULT.
async fn result(c: &mut MapleTestConnection) -> Vec<u8> {
    expect(c, |p| {
        (p.opcode() == SendOpcode::AntiMacroResult as i16).then(|| p.bytes[2..].to_vec())
    })
    .await
}

/// The mesos the next stat update with them says the player has.
async fn mesos(c: &mut MapleTestConnection) -> i32 {
    expect(c, |p| {
        if p.opcode() != SendOpcode::StatChange as i16 {
            return None;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        r.read_byte().unwrap();
        (r.read_int().unwrap() == MESO).then(|| r.read_int().unwrap())
    })
    .await
}

fn use_lie_detector(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseLieDetector as i16).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(1).unwrap();
    p.write_int(LIE_DETECTOR).unwrap();
    p
}

fn answer(typed: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AnswerLieDetector as i16).unwrap();
    p.write_str_with_length(typed).unwrap();
    p
}

#[tokio::test]
async fn a_wrong_answer_fines_the_tested_player_for_the_tester() {
    let tester_config = load_harness_config_or_fail().await;
    let tester_id = login_to_world_session(&tester_config)
        .await
        .expect("first login failed")
        .character_id;
    let tested_config = load_harness_config_or_fail().await;
    let tested_id = login_to_world_session(&tested_config)
        .await
        .expect("second login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(tester_id, 2, 1, LIE_DETECTOR, 3)
        .await
        .expect("lie detectors");
    set_meso(tester_id, 100).await.expect("tester's mesos");
    set_meso(tested_id, 10_000).await.expect("tested mesos");
    let name = tested_config.character_name.clone();
    let tester_name = tester_config.character_name.clone();

    let mut tested = relogin(&tested_config).await;
    let mut tester = relogin(&tester_config).await;
    let b = &mut tested.connection;
    let a = &mut tester.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(b, HUNTING_GROUND, "", &mut trace).await,
        HUNTING_GROUND
    );
    assert_eq!(
        warp(a, HUNTING_GROUND, "", &mut trace).await,
        HUNTING_GROUND
    );

    // Not fighting, so not tested; nor is a name no one has.
    send(a, use_lie_detector(&name)).await;
    assert_eq!(result(a).await, [1, 1]);
    send(a, use_lie_detector("nobody")).await;
    assert_eq!(result(a).await, [0, 1]);

    // Fighting: the test begins, naming the tester, and the detector goes.
    send(b, attack(0, 1)).await;
    barrier(b, &name, &mut Trace::default()).await;
    send(a, use_lie_detector(&name)).await;
    let quantity = expect(a, |p| {
        (p.opcode() == SendOpcode::InventoryOperation as i16)
            .then(|| i16::from_le_bytes([p.bytes[8], p.bytes[9]]))
    })
    .await;
    assert_eq!(quantity, 2);
    let mut used = vec![5, 1, tester_name.len() as u8, 0];
    used.extend(tester_name.as_bytes());
    assert_eq!(result(b).await, used);
    let challenge = result(b).await;
    assert_eq!(challenge[..3], [6, 1, 1]);
    let len = i32::from_le_bytes(challenge[3..7].try_into().unwrap()) as usize;
    assert_eq!(challenge.len(), 7 + len);
    assert_eq!(challenge[7..9], [0xFF, 0xD8]);

    // A second test waits for the first.
    send(a, use_lie_detector(&name)).await;
    assert_eq!(result(a).await, [3, 1]);

    // A wrong answer fails: 7000 mesos from one to the other.
    send(b, answer("wrong!")).await;
    assert_eq!(mesos(b).await, 3000);
    assert_eq!(result(b).await, [7, 1]);
    assert_eq!(mesos(a).await, 7100);
    assert_eq!(result(a).await, [11, 1]);

    // Tested now, so not again for a while.
    send(b, attack(0, 1)).await;
    barrier(b, &name, &mut Trace::default()).await;
    send(a, use_lie_detector(&name)).await;
    assert_eq!(result(a).await, [2, 1]);
    drop(tester);
    drop(tested);

    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(relogin(&tested_config).await.meso, 3000);
    let after = relogin(&tester_config).await;
    assert_eq!(after.meso, 7100);
    assert_eq!(after.items.get(&LIE_DETECTOR), Some(&2));
}
