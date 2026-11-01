//! Scripted NPCs without speech of their own that only ever say a line,
//! such as Spindle in Ludibrium, say their String.nx default line.

use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const LUDIBRIUM: i32 = 221000000;
const SPINDLE: i32 = 9201082;
const FOYER: i32 = 682000100;
const MAID: i32 = 2120003;

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

    // The Haunted House's Maid, in its Foyer, says hers too.
    let c = &mut session.connection;
    assert_eq!(warp(c, FOYER, "", &mut seen).await, FOYER);
    let trace = told(&mut session, talk(FOYER, MAID)).await;
    assert_eq!(trace.prompts, [(MAID, 0)], "{trace:?}");
    assert!(trace.prompt_texts[0].contains("Master"), "{trace:?}");
}
