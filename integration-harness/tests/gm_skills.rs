//! The GM skills: a GM's Holy Symbol reaches everyone around them, in their
//! party or not, as Echo of Hero does; a player who is not a GM can't use
//! it even with the skill. A GM's Hide takes the GM from others' sight.

use integration_harness::database::{grant_skill, set_gm_level};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const FIELD: i32 = 101_020_001;
const HOLY_SYMBOL: i32 = 9_101_002;
const HIDE: i32 = 9_101_004;
/// Its `x` percent for its `time` of 900 seconds.
const SYMBOL: (i32, i16, i32) = (HOLY_SYMBOL, 50, 900_000);

#[tokio::test]
async fn a_gms_holy_symbol_reaches_strangers_and_no_one_else_casts_it() {
    let config = load_harness_config_or_fail().await;
    let gm_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    let other_id = login_to_world_session(&other_config)
        .await
        .expect("second login failed")
        .character_id;
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");
    for id in [gm_id, other_id] {
        grant_skill(id, HOLY_SYMBOL, 1)
            .await
            .expect("teach Holy Symbol");
    }
    grant_skill(gm_id, HIDE, 1).await.expect("teach Hide");
    for config in [&config, &other_config] {
        let mut session = relogin(config).await;
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut session.connection, FIELD, "", &mut trace).await,
            FIELD
        );
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut gm = relogin(&config).await;
    let mut stranger = relogin(&other_config).await;
    let (g, s) = (&mut gm.connection, &mut stranger.connection);
    let (mut gt, mut st) = (Trace::default(), Trace::default());
    read_for(g, &mut gt, 300).await;
    read_for(s, &mut st, 300).await;

    send(s, use_skill(HOLY_SYMBOL, 1)).await;
    read_for(s, &mut st, 500).await;
    assert!(st.buffs.is_empty(), "a player who is not a GM cast it");

    send(g, use_skill(HOLY_SYMBOL, 1)).await;
    until(g, &mut gt, |t| !t.buffs.is_empty()).await;
    assert_eq!(gt.buffs, [SYMBOL]);
    until(s, &mut st, |t| !t.buffs.is_empty()).await;
    assert_eq!(st.buffs, [SYMBOL]);

    // Hiding takes the GM from the stranger's sight, and from the sight of
    // one who comes in; coming out shows the GM again.
    send(g, use_skill(HIDE, 1)).await;
    until(g, &mut gt, |t| !t.hidden.is_empty()).await;
    assert_eq!(gt.hidden, [true]);
    until(s, &mut st, |t| t.gone.contains(&gm_id)).await;
    drop(stranger);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut stranger = relogin(&other_config).await;
    let s = &mut stranger.connection;
    let mut st = Trace::default();
    read_for(s, &mut st, 500).await;
    assert!(
        st.players.iter().all(|p| p.character_id != gm_id),
        "a newcomer saw the hidden GM"
    );
    send(g, use_skill(HIDE, 1)).await;
    until(g, &mut gt, |t| t.hidden.len() == 2).await;
    assert_eq!(gt.hidden, [true, false]);
    until(s, &mut st, |t| {
        t.players.iter().any(|p| p.character_id == gm_id)
    })
    .await;
}
