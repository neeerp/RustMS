//! Smokescreen: the Shadower's smoke covers the box around them, which the
//! client shelters its party in, and the skill can't be cast again for its
//! cooltime.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::io::write::PktWrite;
use tokio::time::Duration;

const KERNING_CITY: i32 = 103_000_000;
const SHADOWER: i16 = 422;
const SMOKESCREEN: i32 = 4221006;
const SMOKE: i32 = 2;

#[tokio::test]
async fn smokescreen_covers_the_shadower_then_cools_down() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, SHADOWER, 120, 1000).await.expect("shadower");
    grant_skill(id, SMOKESCREEN, 1)
        .await
        .expect("teach Smokescreen");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, KERNING_CITY, "", &mut t).await, KERNING_CITY);
    let cast = || {
        let mut p = use_skill(SMOKESCREEN, 1);
        p.write_short(0).unwrap();
        p
    };

    send(c, cast()).await;
    until(c, &mut t, |t| !t.field.mists.is_empty()).await;
    let mist = t.field.mists[0];
    assert_eq!(
        (mist.kind, mist.owner_id, mist.skill_id, mist.level, mist.delay),
        (SMOKE, id, SMOKESCREEN, 1, 8)
    );
    // Level 1's box, (-110, -82) to (110, 83) around the Shadower.
    let (left, top, right, bottom) = mist.area;
    assert_eq!((right - left, bottom - top), (220, 165));

    // Ten minutes before the next.
    until(c, &mut t, |t| t.cooldowns.contains(&(SMOKESCREEN, 600))).await;
    send(c, cast()).await;
    read_for(c, &mut t, 1000).await;
    assert_eq!(t.field.mists.len(), 1, "no second smoke while cooling down");
}
