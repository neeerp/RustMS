//! Families (report b039): the client gets the Family window's numbers on
//! entering the game. A senior asks a character on their map to be their
//! junior (ADD_FAMILY), who gets one card however often they are asked, and
//! may say no or yes (ACCEPT_FAMILY). The server refuses a name not online,
//! another map, too low a level, a junior who has a senior, and a senior of
//! the same family. The Family Tree (OPEN_FAMILY_PEDIGREE) shows who is in
//! it and online, and relatives hear who logs in or out. The leader alone
//! may change the precepts, which the whole family sees. Parting from a
//! junior or a senior (SEPARATE_FAMILY_BY_SENIOR/JUNIOR, report b048) costs
//! the asker a fee by the levels between them, and the seniors Rep by the
//! junior's level.

use integration_harness::database::{set_family_reputation, set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const LITH_HARBOR: i32 = 104_000_000;
const HENESYS: i32 = 100_000_000;

const NOT_FOUND: i32 = 0x41;
const SAME_FAMILY: i32 = 0x42;
const NOT_SAME_MAP: i32 = 0x45;
const HAS_SENIOR: i32 = 0x46;
const TOO_LOW: i32 = 0x4d;

fn request(op: RecvOpcode, name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(op as i16).unwrap();
    p.write_str_with_length(name).unwrap();
    p
}

fn answer(inviter: i32, name: &str, accept: bool) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AcceptFamily as i16).unwrap();
    p.write_int(inviter).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_byte(u8::from(accept)).unwrap();
    p
}

fn str_bytes(text: &str) -> Vec<u8> {
    let mut bytes = (text.len() as u16).to_le_bytes().to_vec();
    bytes.extend(text.as_bytes());
    bytes
}

/// FAMILY_INFO_RESULT with no rep yet.
fn info(juniors: u16, leader: i32, name: &str) -> (i16, Vec<u8>) {
    info_with(0, juniors, leader, name, "")
}

fn info_with(rep: i32, juniors: u16, leader: i32, name: &str, precepts: &str) -> (i16, Vec<u8>) {
    let mut body = [rep, 0, 0].map(i32::to_le_bytes).concat();
    body.extend(juniors.to_le_bytes());
    body.extend(2u16.to_le_bytes());
    body.extend(0u16.to_le_bytes());
    body.extend(leader.to_le_bytes());
    body.extend(str_bytes(name));
    body.extend(str_bytes(precepts));
    body.extend(0i32.to_le_bytes());
    (SendOpcode::FamilyInfo as i16, body)
}

fn refusal(code: i32) -> (i16, Vec<u8>) {
    let body = [code, 0].map(i32::to_le_bytes).concat();
    (SendOpcode::FamilyResult as i16, body)
}

fn with_name(op: SendOpcode, head: &[u8], name: &str) -> (i16, Vec<u8>) {
    let mut body = head.to_vec();
    body.extend(str_bytes(name));
    (op as i16, body)
}

/// Make the character behind `config` at `level` and log it out; its id.
async fn new_character(config: &HarnessConfig, level: i16) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.expect("level");
    id
}

/// Log in and go to `map`; the family packets of entering the game.
async fn enter(config: &HarnessConfig, map: i32) -> (WorldSession, Trace) {
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    until(&mut session.connection, &mut t, |t| t.family.len() >= 2).await;
    assert_eq!(warp(&mut session.connection, map, "", &mut t).await, map);
    (session, t)
}

/// The next `count` family packets `session` gets.
async fn family(session: &mut WorldSession, count: usize) -> Vec<(i16, Vec<u8>)> {
    let mut t = Trace::default();
    until(&mut session.connection, &mut t, |t| t.family.len() >= count).await;
    read_for(&mut session.connection, &mut t, 300).await;
    t.family
}

#[tokio::test]
async fn a_senior_takes_a_junior_on_their_map_who_says_yes() {
    let (a_config, b_config, c_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let a_id = new_character(&a_config, 30).await;
    let b_id = new_character(&b_config, 15).await;
    new_character(&c_config, 1).await;
    let (an, bn, cn) = (
        a_config.character_name.clone(),
        b_config.character_name.clone(),
        c_config.character_name.clone(),
    );

    // Out of a family: no leader and no name, and nothing else on login.
    let mut a = relogin(&a_config).await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| t.family.len() >= 2).await;
    read_for(&mut a.connection, &mut at, 300).await;
    assert_eq!(
        at.family,
        [
            info(0, 0, ""),
            (SendOpcode::FamilyPrivilegeList as i16, vec![0; 4])
        ]
    );
    assert_eq!(
        warp(&mut a.connection, LITH_HARBOR, "", &mut at).await,
        LITH_HARBOR
    );
    let (mut b, _) = enter(&b_config, HENESYS).await;
    let (c, _) = enter(&c_config, LITH_HARBOR).await;

    send(
        &mut a.connection,
        request(RecvOpcode::AddFamily, "nobody here"),
    )
    .await;
    assert_eq!(family(&mut a, 1).await, [refusal(NOT_FOUND)]);
    send(&mut a.connection, request(RecvOpcode::AddFamily, &bn)).await;
    assert_eq!(family(&mut a, 1).await, [refusal(NOT_SAME_MAP)]);
    send(&mut a.connection, request(RecvOpcode::AddFamily, &cn)).await;
    assert_eq!(family(&mut a, 1).await, [refusal(TOO_LOW)]);
    drop(c);

    // B says no, then yes.
    let mut bt = Trace::default();
    assert_eq!(
        warp(&mut b.connection, LITH_HARBOR, "", &mut bt).await,
        LITH_HARBOR
    );
    let asked = with_name(SendOpcode::FamilyJoinRequest, &a_id.to_le_bytes(), &an);
    // Asking again while the card is up shows no second card.
    send(&mut a.connection, request(RecvOpcode::AddFamily, &bn)).await;
    send(&mut a.connection, request(RecvOpcode::AddFamily, &bn)).await;
    assert_eq!(family(&mut b, 1).await, [asked.clone()]);
    send(&mut b.connection, answer(a_id, &an, false)).await;
    let declined = with_name(SendOpcode::FamilyJoinRequestResult, &[0], &bn);
    assert_eq!(family(&mut a, 1).await, [declined]);

    send(&mut a.connection, request(RecvOpcode::AddFamily, &bn)).await;
    assert_eq!(family(&mut b, 1).await, [asked]);
    send(&mut b.connection, answer(a_id, &an, true)).await;
    let accepted = with_name(SendOpcode::FamilyJoinRequestResult, &[1], &bn);
    assert_eq!(family(&mut a, 2).await, [accepted, info(1, a_id, &an)]);
    let joined = with_name(SendOpcode::FamilyJoinAccepted, &[], &an);
    assert_eq!(family(&mut b, 2).await, [joined, info(0, a_id, &an)]);

    // Neither may ask the other again.
    send(&mut a.connection, request(RecvOpcode::AddFamily, &bn)).await;
    assert_eq!(family(&mut a, 1).await, [refusal(HAS_SENIOR)]);
    send(&mut b.connection, request(RecvOpcode::AddFamily, &an)).await;
    assert_eq!(family(&mut b, 1).await, [refusal(SAME_FAMILY)]);

    // A's tree: A leads, B below, both online in Lith Harbor.
    send(
        &mut a.connection,
        request(RecvOpcode::OpenFamilyPedigree, &an),
    )
    .await;
    let mut chart = a_id.to_le_bytes().to_vec();
    chart.extend(3i32.to_le_bytes());
    // The leader heads the tree and comes again as the one viewed.
    for (id, senior, level, name) in [
        (a_id, 0, 30u8, &an),
        (a_id, 0, 30, &an),
        (b_id, a_id, 15, &bn),
    ] {
        chart.extend(id.to_le_bytes());
        chart.extend(senior.to_le_bytes());
        chart.extend(0i16.to_le_bytes());
        chart.extend([level, 1]);
        chart.extend([0i32; 4].map(i32::to_le_bytes).concat());
        chart.extend(LITH_HARBOR.to_le_bytes());
        chart.extend(0i32.to_le_bytes());
        chart.extend(str_bytes(name));
    }
    chart.extend(3i32.to_le_bytes());
    for (key, count) in [(-1, 2), (0, 0), (b_id, 1)] {
        chart.extend([key, count].map(i32::to_le_bytes).concat());
    }
    chart.extend(0i32.to_le_bytes());
    chart.extend(2u16.to_le_bytes());
    assert_eq!(
        family(&mut a, 1).await,
        [(SendOpcode::FamilyChart as i16, chart)]
    );

    // A hears B leave and come back, who has their family on login.
    drop(b);
    let online = |online| with_name(SendOpcode::FamilyNotifyLoginOrLogout, &[online], &bn);
    assert_eq!(family(&mut a, 1).await, [online(0)]);
    let mut b = relogin(&b_config).await;
    assert_eq!(family(&mut a, 1).await, [online(1)]);
    let entered = family(&mut b, 2).await;
    assert_eq!(entered[0], info(0, a_id, &an));
}

/// `senior` asks `junior`, both on one map, who says yes.
async fn take_junior(
    senior: &mut WorldSession,
    senior_name: &str,
    junior: &mut WorldSession,
    junior_name: &str,
) {
    send(
        &mut senior.connection,
        request(RecvOpcode::AddFamily, junior_name),
    )
    .await;
    family(junior, 1).await;
    send(
        &mut junior.connection,
        answer(senior.character_id, senior_name, true),
    )
    .await;
    family(senior, 2).await;
    family(junior, 2).await;
}

fn result(code: i32, arg: i32) -> (i16, Vec<u8>) {
    let body = [code, arg].map(i32::to_le_bytes).concat();
    (SendOpcode::FamilyResult as i16, body)
}

#[tokio::test]
async fn the_leader_sets_the_precepts_and_parting_costs_rep_and_mesos() {
    let (a_config, b_config, c_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let a_id = new_character(&a_config, 50).await;
    let b_id = new_character(&b_config, 35).await;
    let c_id = new_character(&c_config, 20).await;
    set_meso(a_id, 100_000).await.expect("meso");
    set_meso(b_id, 0).await.expect("meso");
    set_meso(c_id, 100_000).await.expect("meso");
    let (an, bn, cn) = (
        a_config.character_name.clone(),
        b_config.character_name.clone(),
        c_config.character_name.clone(),
    );
    let (mut a, _) = enter(&a_config, LITH_HARBOR).await;
    let (mut b, _) = enter(&b_config, LITH_HARBOR).await;
    let (mut c, _) = enter(&c_config, LITH_HARBOR).await;
    take_junior(&mut a, &an, &mut b, &bn).await;
    take_junior(&mut b, &bn, &mut c, &cn).await;
    set_family_reputation(a_id, 5000).await.expect("rep");
    set_family_reputation(b_id, 5000).await.expect("rep");

    // Only the leader's precepts count, and everyone sees them.
    let precepts = |text: &str| {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::ChangeFamilyMessage as i16)
            .unwrap();
        p.write_str_with_length(text).unwrap();
        p
    };
    send(&mut b.connection, precepts("Not mine to say")).await;
    send(&mut a.connection, precepts("Be kind")).await;
    assert_eq!(
        family(&mut a, 1).await,
        [info_with(5000, 1, a_id, &an, "Be kind")]
    );
    assert_eq!(
        family(&mut b, 1).await,
        [info_with(5000, 1, a_id, &an, "Be kind")]
    );
    assert_eq!(
        family(&mut c, 1).await,
        [info_with(0, 0, a_id, &an, "Be kind")]
    );

    // B can't pay to part from C: 15 levels between them.
    let mut cut = Packet::new_empty();
    cut.write_short(RecvOpcode::SeparateFamilyBySenior as i16)
        .unwrap();
    cut.write_int(c_id).unwrap();
    send(&mut b.connection, cut).await;
    assert_eq!(family(&mut b, 1).await, [result(0x51, (15 + 2500) * 15)]);

    // C parts from B and pays. B loses 2 * (20 / 20 + 10) * 20, A half.
    let mut leave = Packet::new_empty();
    leave
        .write_short(RecvOpcode::SeparateFamilyByJunior as i16)
        .unwrap();
    let mut ct = Trace::default();
    send(&mut c.connection, leave).await;
    until(&mut c.connection, &mut ct, |t| t.family.len() >= 2).await;
    assert_eq!(ct.family, [result(1, 0), info(0, 0, "")]);
    let meso = ct
        .stat_updates
        .iter()
        .find_map(|u| u.get(&integration_harness::trace::stat::MESO));
    assert_eq!(meso, Some(&(100_000 - 37_725)));
    assert_eq!(
        family(&mut b, 1).await,
        [info_with(5000 - 440, 0, a_id, &an, "Be kind")]
    );
    assert_eq!(
        family(&mut a, 1).await,
        [info_with(5000 - 220, 1, a_id, &an, "Be kind")]
    );

    // A cuts off B, with no senior above to lose half. Neither is left in
    // a family.
    let mut cut = Packet::new_empty();
    cut.write_short(RecvOpcode::SeparateFamilyBySenior as i16)
        .unwrap();
    cut.write_int(b_id).unwrap();
    let mut at = Trace::default();
    send(&mut a.connection, cut).await;
    until(&mut a.connection, &mut at, |t| t.family.len() >= 2).await;
    let a_rep = 5000 - 220 - 2 * (35 / 20 + 10) * 35;
    assert_eq!(at.family, [result(1, 0), info_with(a_rep, 0, 0, "", "")]);
    let meso = at
        .stat_updates
        .iter()
        .find_map(|u| u.get(&integration_harness::trace::stat::MESO));
    assert_eq!(meso, Some(&(100_000 - 37_725)));
    assert_eq!(
        family(&mut b, 1).await,
        [info_with(5000 - 440, 0, 0, "", "")]
    );
}
