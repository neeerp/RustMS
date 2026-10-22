//! Meso Explosion (4211006): a Chief Bandit sets off the mesos they threw,
//! each going off a tenth of a second after the one before, and a line
//! hurts only as much as its drop's mesos allow (ceil(50·x·ratio), 1038
//! for 100 mesos and 342 for 10 at level 30). An explosion whose drops and
//! lines don't add up is refused and sets off nothing.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, replace_item, set_job, set_meso};
use integration_harness::hunting::{meso_drop, meso_explosion};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use tokio::time::Duration;

/// A snail map no other test uses.
const FIELD: i32 = 1_020_001;
const SNAILS: [i32; 2] = [100100, 100101];
const CHIEF_BANDIT: i16 = 421;
const MESO_EXPLOSION: i32 = 4211006;
const MP_COST: i64 = 30;
const DAGGER: i32 = 1332000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const DELAY: i16 = 300;

/// A Chief Bandit with Meso Explosion at level 30, 1000 MP and 100,000
/// mesos, on the test map.
async fn chief_bandit() -> (WorldSession, i32) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, CHIEF_BANDIT, 70, 1000)
        .await
        .expect("chief bandit");
    grant_skill(id, MESO_EXPLOSION, 30)
        .await
        .expect("Meso Explosion");
    set_meso(id, 100_000).await.expect("mesos");
    replace_item(id, EQUIPPED, WEAPON, DAGGER)
        .await
        .expect("give a dagger");

    let mut session = relogin(&config).await;
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut session.connection, FIELD, "", &mut t).await,
        FIELD
    );
    (session, id)
}

/// Which of `drops` went off, and when. The other test's explosions on
/// the map show too.
fn went_off(t: &Trace, drops: &[i32]) -> Vec<(i32, i16)> {
    (t.field.exploded.iter().copied())
        .filter(|(drop, _)| drops.contains(drop))
        .collect()
}

/// Throws `amount` mesos where the player stands; the drop's object id.
async fn throw(c: &mut MapleTestConnection, h: &mut Trace, owner: i32, amount: i32) -> i32 {
    let mine = |t: &Trace| {
        (t.field.drops.iter())
            .filter(|d| d.meso && d.owner == owner && d.value == amount)
            .map(|d| d.object_id)
            .collect::<Vec<_>>()
    };
    let before = mine(h).len();
    send(c, meso_drop(amount)).await;
    until(c, h, |t| mine(t).len() > before).await;
    *mine(h).last().unwrap()
}

#[tokio::test]
async fn an_explosion_sets_off_its_drops_and_their_mesos_hurt() {
    let (mut session, id) = chief_bandit().await;
    let c = &mut session.connection;
    const MARK: &str = "meso explosion";
    let mut h = Trace::default();
    let alive = |t: &Trace| {
        SNAILS
            .iter()
            .flat_map(|&mob| t.field.alive(mob))
            .collect::<Vec<_>>()
    };
    until(c, &mut h, |t| alive(t).len() >= 2).await;
    let (strong, weak) = (alive(&h)[0], alive(&h)[1]);
    let hundred = throw(c, &mut h, id, 100).await;
    let ten = throw(c, &mut h, id, 10).await;

    // 1000 from the hundred is under its 1038 and kills; 400 from the ten
    // is over its 342 and does nothing. The second drop goes 100 ms later.
    let t = step(
        c,
        MARK,
        meso_explosion(
            &[(strong, &[1000]), (weak, &[400])],
            &[(hundred, 0b01), (ten, 0b10)],
            DELAY,
        ),
    )
    .await;
    assert_eq!(t.stat_values(stat::MP), [1000 - MP_COST]);
    assert_eq!(
        went_off(&t, &[hundred, ten]),
        [(hundred, DELAY), (ten, DELAY + 100)]
    );
    assert!(
        t.field.killed.contains(&strong),
        "1000 from 100 mesos kills"
    );
    assert!(
        !t.field.killed.contains(&weak),
        "400 from 10 mesos is too much"
    );

    // A drop already set off is gone: the explosion still costs its MP.
    let t = step(c, MARK, meso_explosion(&[], &[(hundred, 0)], DELAY)).await;
    assert_eq!(t.stat_values(stat::MP), [1000 - 2 * MP_COST]);
    assert!(went_off(&t, &[hundred]).is_empty(), "a drop went off twice");
}

#[tokio::test]
async fn an_explosion_that_doesnt_add_up_sets_off_nothing() {
    let (mut session, id) = chief_bandit().await;
    let c = &mut session.connection;
    const MARK: &str = "meso explosion refused";
    let mut h = Trace::default();
    let bag = throw(c, &mut h, id, 100).await;
    // No mob of the other test's: these are refused before any is hurt.
    let nobody = 1;
    let too_many: Vec<(i32, u8)> = std::iter::once((bag, 0))
        .chain((1..=20).map(|fake| (-fake, 0)))
        .collect();

    let refused = [
        // The same drop twice.
        meso_explosion(&[], &[(bag, 0), (bag, 0)], DELAY),
        // A line from no drop.
        meso_explosion(&[(nobody, &[1])], &[(bag, 0)], DELAY),
        // A drop hurting a target that isn't there.
        meso_explosion(&[], &[(bag, 0b1)], DELAY),
        // More drops than its 20.
        meso_explosion(&[], &too_many, DELAY),
    ];
    for packet in refused {
        let t = step(c, MARK, packet).await;
        assert!(
            t.stat_values(stat::MP).is_empty(),
            "an explosion took its MP"
        );
        assert!(
            went_off(&t, &[bag]).is_empty(),
            "a refused explosion went off"
        );
    }

    // The drop is still there to set off.
    let t = step(c, MARK, meso_explosion(&[], &[(bag, 0)], DELAY)).await;
    assert_eq!(t.stat_values(stat::MP), [1000 - MP_COST]);
    assert_eq!(went_off(&t, &[bag]), [(bag, DELAY)]);
}
