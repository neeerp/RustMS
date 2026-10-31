//! The Aran's combo (report b360): each hit an Aran's polearm lands with
//! Combo Ability learned (0xA3) counts one, shown with SHOW_COMBO, and as
//! stat 68 at the first and every tenth. Combo Drain waits for 30 and
//! spends it all, and the combo goes 3 seconds after the last hit.

use integration_harness::database::{grant_skill, replace_item, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until_within, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ARAN: i16 = 2110;
const COMBO_ABILITY: i32 = 21000000;
const COMBO_DRAIN: i32 = 21100005;
const POLEARM: i32 = 1442000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
/// Stat 68 in CANCEL_BUFF's first long, where bits 64 to 95 sit 32 up
/// (`net::packet::build::world::buff::mask`).
const COMBO_BIT: i64 = 1 << (68 - 32);
const MARK: &str = "aran combo";

fn landed() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AranComboCounter as i16).unwrap();
    p
}

/// The values GIVE_BUFF showed for `skill`.
fn given(t: &Trace, skill: i32) -> Vec<i16> {
    (t.buffs.iter())
        .filter(|&&(id, _, _)| id == skill)
        .map(|&(_, value, _)| value)
        .collect()
}

#[tokio::test]
async fn polearm_hits_build_the_combo_that_combo_drain_spends() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, ARAN, 70, 1000).await.expect("aran");
    grant_skill(id, COMBO_ABILITY, 10).await.expect("ability");
    grant_skill(id, COMBO_DRAIN, 20).await.expect("drain");
    replace_item(id, EQUIPPED, WEAPON, POLEARM)
        .await
        .expect("give a polearm");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // The first hit shows 1 and gives it as stat 68.
    let t = step(c, MARK, landed()).await;
    assert_eq!(t.combos, [1]);
    assert_eq!(given(&t, COMBO_ABILITY), [1]);
    for count in 2..=29 {
        let t = step(c, MARK, landed()).await;
        assert_eq!(t.combos, [count]);
        let shown: &[i16] = if count % 10 == 0 {
            &[count as i16]
        } else {
            &[]
        };
        assert_eq!(given(&t, COMBO_ABILITY), shown);
    }

    // 29 is too few for Combo Drain.
    let t = step(c, MARK, use_skill(COMBO_DRAIN, 20)).await;
    assert!(given(&t, COMBO_DRAIN).is_empty(), "29 drained");

    // At 30 it goes off and spends the combo.
    let t = step(c, MARK, landed()).await;
    assert_eq!(
        (&t.combos[..], &given(&t, COMBO_ABILITY)[..]),
        (&[30][..], &[30][..])
    );
    let t = step(c, MARK, use_skill(COMBO_DRAIN, 20)).await;
    assert_eq!(given(&t, COMBO_DRAIN), [5]);
    assert!(t.cancelled_high_buffs.contains(&COMBO_BIT), "{t:?}");
    let t = step(c, MARK, use_skill(COMBO_DRAIN, 20)).await;
    assert!(given(&t, COMBO_DRAIN).is_empty(), "drained again at 0");

    // A new combo starts at 1 and ends by itself 3 seconds on.
    let mut t = step(c, MARK, landed()).await;
    assert_eq!(t.combos, [1]);
    let ended = |t: &Trace| t.cancelled_high_buffs.contains(&COMBO_BIT);
    until_within(c, &mut t, Duration::from_secs(10), ended).await;
}
