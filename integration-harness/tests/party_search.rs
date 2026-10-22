//! Party searches: a leader's 0xDE invites one partyless character in the
//! same channel who fits the levels and jobs asked for, with the search's
//! flag set, and the party filling up ends the search.

use integration_harness::database::set_job;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const OP_CREATED: u8 = 0x08;
const OP_JOIN: u8 = 0x0F;

/// Levels and jobs no other harness test uses, so only these characters fit.
const LEVEL: i16 = 163;
const CRUSADER: i16 = 111;
const WHITE_KNIGHT: i16 = 121;
/// The Fighter box, which covers its whole branch.
const FIGHTERS: i32 = 1 << 4;

fn party_operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartyOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

fn search(min_level: i32, max_level: i32, members: i32, jobs: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartySearchStart as i16).unwrap();
    for value in [min_level, max_level, members, jobs] {
        p.write_int(value).unwrap();
    }
    p
}

/// v83 accepts a search's invitation with its flag after the party id.
fn accept(party_id: i32) -> Packet {
    let mut p = party_operation(3);
    p.write_int(party_id).unwrap();
    p.write_byte(1).unwrap();
    p
}

/// A character of `job` at `level`, logged in.
async fn character(config: &HarnessConfig, job: i16, level: i16) -> WorldSession {
    let session = login_to_world_session(config).await.expect("login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    set_job(id, job, level, 100).await.expect("set the job");
    relogin(config).await
}

#[tokio::test]
async fn a_party_search_invites_a_fitting_character() {
    let (leader_config, wanted_config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let mut leader = character(&leader_config, CRUSADER, LEVEL).await;
    let mut wanted = character(&wanted_config, CRUSADER, LEVEL + 1).await;
    // Higher, so the first asked if jobs didn't count.
    let mut other = character(&other_config, WHITE_KNIGHT, LEVEL + 2).await;
    let (mut l, mut w, mut o) = (Trace::default(), Trace::default(), Trace::default());

    send(&mut leader.connection, party_operation(1)).await;
    until(&mut leader.connection, &mut l, |t| {
        t.party_ops.contains(&OP_CREATED)
    })
    .await;

    // The search asks the Crusader at once, with the search's flag.
    let level = i32::from(LEVEL);
    send(
        &mut leader.connection,
        search(level - 10, level + 10, 2, FIGHTERS),
    )
    .await;
    until(&mut wanted.connection, &mut w, |t| {
        !t.party_invites.is_empty()
    })
    .await;
    let (party_id, inviter) = w.party_invites[0].clone();
    assert_eq!(inviter, leader.character_name);
    assert_eq!(w.party_invite_searches, [1]);

    send(&mut wanted.connection, accept(party_id)).await;
    until(&mut leader.connection, &mut l, |t| {
        t.party_ops.contains(&OP_JOIN)
    })
    .await;

    // The White Knight was never asked.
    read_for(&mut other.connection, &mut o, 500).await;
    assert!(o.party_invites.is_empty());
}
