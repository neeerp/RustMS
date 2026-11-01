//! Scripted NPCs without speech of their own that only ever say a line,
//! such as Spindle in Ludibrium, say their String.nx default line.

use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const LUDIBRIUM: i32 = 221000000;
const SPINDLE: i32 = 9201082;

#[tokio::test]
async fn spindle_says_the_default_line() {
    let config = load_harness_config_or_fail().await;
    drop(login_to_world_session(&config).await.unwrap());
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, LUDIBRIUM, "", &mut seen).await, LUDIBRIUM);
    let trace = told(&mut session, talk(LUDIBRIUM, SPINDLE)).await;
    assert_eq!(trace.prompts, [(SPINDLE, 0)], "{trace:?}");
    assert!(trace.prompt_texts[0].contains("Spindle"), "{trace:?}");
}
