//! The pirates' morph skills (reports 225b, 225c): Oak Barrel hides its
//! user in Morph.nx 1002, shown to the map until cancelled, and
//! Transformation gives its defense, speed and jump with a superman
//! template, a woman's 100 on from a man's. The Wind Archer's Eagle Eye
//! does the same with 1003.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, set_gender, set_job};
use integration_harness::hunting::{cancel_buff, use_skill};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const LITH_HARBOR: i32 = 104_000_000;
const OAK_BARREL: i32 = 5_101_007;
const TRANSFORMATION: i32 = 5_111_005;
const EAGLE_EYE: i32 = 13_111_005;
/// Morph's flag (bit 33), in the second half of a buff mask.
const MORPH: i64 = 2;

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("morph"))
            .await
            .expect("timed out")
            .expect("read packet")
            .packet;
        if let Some(picked) = pick(&p) {
            return picked;
        }
    }
}

/// The body of `p` if it is an `op`.
fn body(p: &Packet, op: SendOpcode) -> Option<Cursor<&Vec<u8>>> {
    if p.opcode() != op as i16 {
        return None;
    }
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    Some(r)
}

/// GIVE_BUFF's values and reasons when it carries Morph, in mask order.
fn given(p: &Packet) -> Option<Vec<(i16, i32)>> {
    let mut r = body(p, SendOpcode::GiveBuff)?;
    let first = r.read_long().unwrap();
    let second = r.read_long().unwrap();
    if second & MORPH == 0 {
        return None;
    }
    let stats = first.count_ones() + second.count_ones();
    Some(
        (0..stats)
            .map(|_| {
                let value = r.read_short().unwrap();
                let reason = r.read_int().unwrap();
                r.read_int().unwrap();
                (value, reason)
            })
            .collect(),
    )
}

/// A foreign buff's (character, morph) when it is Morph's.
fn foreign_morph(p: &Packet) -> Option<(i32, i16)> {
    let mut r = body(p, SendOpcode::GiveForeignBuff)?;
    let who = r.read_int().unwrap();
    (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH)
        .then(|| (who, r.read_short().unwrap()))
}

#[tokio::test]
async fn oak_barrel_shows_to_the_map_until_cancelled() {
    let config = load_multi_harness_config_or_fail().await;
    let brawler_config = config.player("sender").unwrap();
    let watcher_config = config.player("recipient").unwrap();

    let brawler_id = login_to_world_session(&brawler_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&brawler_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(brawler_id, 510, 30, 500).await.expect("brawler");
    grant_skill(brawler_id, OAK_BARREL, 1)
        .await
        .expect("teach Oak Barrel");

    let mut brawler = relogin(&brawler_config).await;
    let mut b = Trace::default();
    assert_eq!(
        warp(&mut brawler.connection, LITH_HARBOR, "", &mut b).await,
        LITH_HARBOR
    );
    let mut watcher = login_to_world_session(&watcher_config)
        .await
        .expect("watcher login failed");
    let mut w = Trace::default();
    assert_eq!(
        warp(&mut watcher.connection, LITH_HARBOR, "", &mut w).await,
        LITH_HARBOR
    );

    send(&mut brawler.connection, use_skill(OAK_BARREL, 1)).await;
    let stats = expect(&mut brawler.connection, given).await;
    assert_eq!(stats, vec![(1002, OAK_BARREL)]);
    let seen = expect(&mut watcher.connection, foreign_morph).await;
    assert_eq!(seen, (brawler_id, 1002));

    send(&mut brawler.connection, cancel_buff(OAK_BARREL)).await;
    expect(&mut brawler.connection, |p| {
        let mut r = body(p, SendOpcode::CancelBuff)?;
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then_some(())
    })
    .await;
    let off = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::CancelForeignBuff)?;
        let who = r.read_int().unwrap();
        (r.read_long().unwrap() == 0 && r.read_long().unwrap() == MORPH).then_some(who)
    })
    .await;
    assert_eq!(off, brawler_id);
}

#[tokio::test]
async fn a_womans_transformation_is_template_1100() {
    let config = load_multi_harness_config_or_fail().await;
    let marauder_config = config.player("sender").unwrap();
    let id = login_to_world_session(&marauder_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&marauder_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 511, 120, 2000).await.expect("marauder");
    set_gender(id, 1).await.expect("a woman");
    grant_skill(id, TRANSFORMATION, 20)
        .await
        .expect("teach Transformation");

    let mut marauder = relogin(&marauder_config).await;
    send(&mut marauder.connection, use_skill(TRANSFORMATION, 20)).await;
    let stats = expect(&mut marauder.connection, given).await;
    // Weapon and magic defense, speed, jump, then the morph.
    let values: Vec<i16> = stats.iter().map(|&(value, _)| value).collect();
    assert_eq!(values, vec![40, 40, 40, 20, 1100]);
    assert!(stats.iter().all(|&(_, reason)| reason == TRANSFORMATION));
}

#[tokio::test]
async fn a_womans_eagle_eye_is_template_1103() {
    let config = load_multi_harness_config_or_fail().await;
    let archer_config = config.player("sender").unwrap();
    let id = login_to_world_session(&archer_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&archer_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1311, 70, 2000).await.expect("wind archer");
    set_gender(id, 1).await.expect("a woman");
    grant_skill(id, EAGLE_EYE, 10)
        .await
        .expect("teach Eagle Eye");

    let mut archer = relogin(&archer_config).await;
    send(&mut archer.connection, use_skill(EAGLE_EYE, 10)).await;
    let stats = expect(&mut archer.connection, given).await;
    let values: Vec<i16> = stats.iter().map(|&(value, _)| value).collect();
    assert_eq!(values, vec![30, 30, 40, 20, 1103]);
    assert!(stats.iter().all(|&(_, reason)| reason == EAGLE_EYE));
}
