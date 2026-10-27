//! A party quest's record, kept beside the quest log on its info quest
//! (Henesys's is 1200): entering starts the quest and counts a try, a
//! clear counts and keeps the best time, the day and the rank, and the
//! character data carries the record into the next session.

use integration_harness::database::{grant_stack, set_job};
use integration_harness::events::{enter_hpq, party, talk, waiting_in, GROWLIE};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{HPQ, RICE_CAKE};
use tokio::time::Duration;

const RECORD: i16 = 1200;

/// The latest record for Henesys's party quest that `trace` saw.
fn record(trace: &Trace) -> Option<&str> {
    trace
        .quest_records
        .iter()
        .rev()
        .find(|(quest, _)| *quest == RECORD)
        .map(|(_, data)| data.as_str())
}

#[tokio::test]
async fn entering_and_clearing_keep_the_record() {
    // The leader, whose account is kept to log in again.
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.unwrap();
    grant_stack(id, 4, 1, RICE_CAKE, 10).await.unwrap();
    let mut leader = relogin(&config).await;
    assert!(leader.quest_records.is_empty());
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut leader.connection, HPQ.lobby, "", &mut trace).await,
        HPQ.lobby
    );

    let mut a: Vec<WorldSession> = vec![leader];
    for _ in 0..2 {
        a.push(waiting_in(HPQ.lobby, 10, &[]).await);
    }
    party(&mut a).await;

    // Entering starts the info quest and counts a try, for everyone.
    let traces = enter_hpq(&mut a).await;
    for (player, mut trace) in a.iter_mut().zip(traces) {
        until(&mut player.connection, &mut trace, |t| {
            record(t).is_some() && t.states.contains_key(&RECORD)
        })
        .await;
        assert_eq!(record(&trace), Some("try=1;have=0;rank=F"), "{trace:?}");
        assert_eq!(trace.states[&RECORD], 1, "{trace:?}");
    }

    // The leader hands Growlie the cakes: a clear, timed from the entry.
    let name = a[0].character_name.clone();
    step(&mut a[0].connection, &name, talk(HPQ.entry, GROWLIE)).await;
    let mut seen = Some(step(&mut a[0].connection, &name, answer(4, 1, Some(1))).await);
    for player in a.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            record(t).is_some_and(|data| data.contains("cmp="))
        })
        .await;
        let data = record(&trace).unwrap();
        let (kept, date) = data.split_once(";date=").expect(data);
        assert!(
            kept.starts_with("try=1;have=0;rank=D;cmp=1;min=0;sec="),
            "{data}"
        );
        let date: Vec<&str> = date.split('/').collect();
        assert_eq!(
            date.iter().map(|part| part.len()).collect::<Vec<_>>(),
            [4, 2, 2],
            "{data}"
        );
    }

    // The next session's character data carries it.
    let cleared = a[0].character_name.clone();
    drop(a);
    let leader = relogin(&config).await;
    assert_eq!(leader.character_name, cleared);
    let data = &leader.quest_records[&RECORD];
    assert!(data.starts_with("try=1;have=0;rank=D;cmp=1;"), "{data}");
    assert_eq!(leader.quests.get(&RECORD), Some(&1));
}
