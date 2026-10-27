//! The timed titles (`QuestInfo` `timeLimit2`): starting Diligent Explorer
//! (29003) at Dalair makes its started record the date it ends, 30 days on,
//! "yyyyMMddHHmmss" in UTC, which the client's Quest Log counts down to
//! (#Qdaylimit#...). The record is saved as it stands. Its record beside
//! the quest log counts the days with three hours played (`LT`, `DONE`,
//! `PT`, `CNT`), and the medal waits for 27 of them.

use integration_harness::database::{quest_progress, set_job, set_quest_ex};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, COMPLETE, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const DILIGENT_EXPLORER: i16 = 29003;
const THIRTY_DAYS: i64 = 2_592_000;
const DILIGENT_EXPLORER_MEDAL: i32 = 1142000;

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

/// "yyyyMMddHHmmss" for a Unix time, in UTC.
fn date_of(unix: i64) -> String {
    let (days, seconds) = (unix.div_euclid(86_400), unix.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}{:02}{:02}{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
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

    let mut session = visit(&config).await;
    let c = &mut session.connection;
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

/// Log in, then go to Dalair in Perion.
async fn visit(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    let npcs = &maps().field(PERION).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == DALAIR).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, &config.character_name, &mut trace).await;
    session
}

#[tokio::test]
async fn diligent_explorer_counts_days_of_three_hours_played() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();

    // The start writes a record with nothing played yet.
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let before = date_of(unix_now());
    let t = step(c, &m, build_quest_action(START, DILIGENT_EXPLORER, DALAIR)).await;
    let after = date_of(unix_now());
    let (quest, record) = t.quest_records.last().unwrap().clone();
    assert_eq!(quest, DILIGENT_EXPLORER, "{t:?}");
    let (last, rest) = record
        .strip_prefix("LT=")
        .and_then(|r| r.split_once(';'))
        .unwrap();
    assert!(
        (before.as_str()..=after.as_str()).contains(&last),
        "{record}"
    );
    assert_eq!(rest, "DONE=0;PT=0;CNT=0");

    // With no days counted yet, Dalair keeps the medal.
    let t = step(
        c,
        &m,
        build_quest_action(COMPLETE, DILIGENT_EXPLORER, DALAIR),
    )
    .await;
    assert_eq!(t.states.get(&DILIGENT_EXPLORER), None, "{t:?}");
    assert!(t.items.is_empty(), "{t:?}");

    // Five seconds short of a 27th day, a few seconds online make it.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let today = date_of(unix_now());
    set_quest_ex(
        id,
        DILIGENT_EXPLORER,
        &format!("LT={today};DONE=0;PT=10795;CNT=26"),
    )
    .await
    .unwrap();
    let session = relogin(&config).await;
    tokio::time::sleep(Duration::from_secs(6)).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(
        c,
        &m,
        build_quest_action(COMPLETE, DILIGENT_EXPLORER, DALAIR),
    )
    .await;
    assert_eq!(t.states.get(&DILIGENT_EXPLORER), Some(&2), "{t:?}");
    assert_eq!(t.items, [(DILIGENT_EXPLORER_MEDAL, 1)], "{t:?}");
}
