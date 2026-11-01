//! The job statues on Lith Harbor's dock send a beginner who could take
//! their first job to the job's instructor, and tell one who can't yet to
//! train.

use game_data::server_content::keys;
use integration_harness::database::{set_job, set_str};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::JOB_STATUES;
use tokio::time::Duration;

const LITH_HARBOR: i32 = 104000000;

#[tokio::test]
async fn the_warrior_statue_sends_a_strong_beginner_to_the_sanctuary() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.unwrap();
    set_str(id, 35).await.unwrap();
    let (warrior, _, _, sanctuary) = JOB_STATUES[0];
    let (magician, ..) = JOB_STATUES[2];

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, LITH_HARBOR, "", &mut seen).await, LITH_HARBOR);
    let trace = told(&mut session, talk(LITH_HARBOR, magician)).await;
    let weak = keys::job_statue("magician", "weak");
    assert!(said(&trace, &weak), "{trace:?}");

    let trace = told(&mut session, talk(LITH_HARBOR, warrior)).await;
    assert!(
        said(&trace, &keys::job_statue("warrior", "ask")),
        "{trace:?}"
    );
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&sanctuary)).await;
}
