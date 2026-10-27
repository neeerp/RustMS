//! The timed titles (`QuestInfo` `timeLimit2`): starting Diligent Explorer
//! (29003) at Dalair makes its started record the date it ends, 30 days on,
//! "yyyyMMddHHmmss" in UTC, which the client's Quest Log counts down to
//! (#Qdaylimit#...). The record is saved as it stands.

use integration_harness::database::{quest_progress, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const DILIGENT_EXPLORER: i16 = 29003;
const THIRTY_DAYS: i64 = 2_592_000;

/// The Unix time "yyyyMMddHHmmss" names, in UTC.
fn unix_time(date: &str) -> i64 {
    let part = |at: usize, length: usize| date[at..at + length].parse::<i64>().unwrap();
    let (month, day) = (part(4, 2), part(6, 2));
    let year = part(0, 4) - i64::from(month <= 2);
    // Howard Hinnant's days_from_civil.
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    days * 86_400 + part(8, 2) * 3600 + part(10, 2) * 60 + part(12, 2)
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::test]
async fn diligent_explorer_starts_with_the_date_it_ends() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    let npcs = &maps().field(PERION).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == DALAIR).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, &m, &mut trace).await;

    let before = unix_now();
    let t = step(c, &m, build_quest_action(START, DILIGENT_EXPLORER, DALAIR)).await;
    let after = unix_now();
    assert_eq!(t.progress, [(DILIGENT_EXPLORER, 1)]);
    let ends = t.kills[&DILIGENT_EXPLORER].clone();
    assert!(
        ends.len() == 14 && ends.bytes().all(|b| b.is_ascii_digit()),
        "{ends:?}"
    );
    let end = unix_time(&ends);
    assert!(
        (before + THIRTY_DAYS..=after + THIRTY_DAYS).contains(&end),
        "{ends} is not 30 days on"
    );
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert_eq!(
        quest_progress(id, DILIGENT_EXPLORER).await.unwrap(),
        Some(ends.clone())
    );
    // Logging in again keeps it, and so does the next save.
    let session = relogin(&config).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        quest_progress(id, DILIGENT_EXPLORER).await.unwrap(),
        Some(ends)
    );
}
