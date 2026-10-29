//! The Holiday PQ's end: once the snowman is whole, only the run's leader
//! may have the Snow Spirit bring out Scrooge, as everyone hears; his
//! death wins it, and every member takes a Holiday Buff. The Snow Spirit
//! then trades a Force of the Spirit for a Maplemas gift, each time, and
//! gives each member one prize of the difficulty's, once. The harness
//! world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`).

use game_data::server_content::keys;
use integration_harness::events::{armed_with, enter_holiday, party, said, talk, throw_down, told};
use integration_harness::events::{cleared, waiting_in};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use net::events::{
    holiday_scrooge, holiday_snowman, holiday_win_buff, FORCE_OF_THE_SPIRIT, HOLIDAY,
    HOLIDAY_FORMS, HOLIDAY_GIFTS, HOLIDAY_REWARDS, HOLIDAY_SNOWMAN_AT, SNOW_VIGOR,
};
use net::npc_scripts::SNOW_SPIRIT;

const EASY: u8 = 1;
const PINK: u8 = 5;
const HIT: i32 = 5_000;
const MENU: u8 = 4;
/// Snow Vigor that grows the easy snowman whole from its first form: 4,
/// 5, 6 and 7.
const TO_WHOLE: i16 = 22;

/// Whether `t` saw the player get `item_id`.
fn got(t: &Trace, item_id: i32) -> bool {
    t.added.iter().any(|&(_, _, id)| id == item_id) || t.items.iter().any(|&(id, _)| id == item_id)
}

#[tokio::test]
async fn the_leader_brings_out_scrooge_whose_death_wins_a_buff_and_prizes() {
    let event = &HOLIDAY[usize::from(EASY) - 1];
    let low = *event.levels.start() as i16;
    let items = [(SNOW_VIGOR, TO_WHOLE), (FORCE_OF_THE_SPIRIT, 1)];
    let mut team = vec![armed_with(event.lobby, low, &items).await];
    for at in [low + 4, low + 9] {
        team.push(waiting_in(event.lobby, at, &[]).await);
    }
    party(&mut team).await;
    let mut seen = enter_holiday(&mut team, EASY).await.remove(0);
    let entry = event.entry;
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| t.warps.contains(&entry)).await;

    // The leader brings out the snowman and grows it whole.
    let t = told(&mut team[0], talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_INTRO), "{t:?}");
    send(&mut team[0].connection, answer(0, 1, None)).await;
    let whole = holiday_snowman(EASY, HOLIDAY_FORMS);
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        !t.field.alive(holiday_snowman(EASY, 1)).is_empty()
    })
    .await;
    let (x, y) = HOLIDAY_SNOWMAN_AT;
    send(c, walk_to(x, y)).await;
    for _ in 0..TO_WHOLE {
        send(c, throw_down(1, 1)).await;
    }
    until(c, &mut seen, |t| {
        !t.field.alive(whole).is_empty() && cleared(t)
    })
    .await;

    // Another is sent to the leader.
    told(&mut team[1], talk(entry, SNOW_SPIRIT)).await;
    let t = told(&mut team[1], answer(0, 1, None)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_LEADER), "{t:?}");

    // The leader brings out Scrooge, as everyone hears.
    let t = told(&mut team[0], talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_GROWN), "{t:?}");
    send(&mut team[0].connection, answer(0, 1, None)).await;
    let scrooge = holiday_scrooge(EASY);
    let out = (
        PINK,
        net::content::text().get(keys::HOLIDAY_SCROOGE_OUT, &[]),
    );
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        !t.field.alive(scrooge).is_empty() && t.broadcasts.contains(&out)
    })
    .await;
    let t = told(&mut team[1], talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_SCROOGE), "{t:?}");

    // His death wins it: the clear, and the difficulty's Holiday Buff for
    // all.
    let c = &mut team[0].connection;
    let boss = seen.field.alive(scrooge)[0];
    let clears = |t: &Trace| {
        (t.field_effects.iter())
            .filter(|(_, e)| e == "quest/party/clear")
            .count()
    };
    for _ in 0..40 {
        if seen.field.killed.contains(&boss) {
            break;
        }
        let bars = seen.field.hp_bars.len();
        send(c, attack(boss, HIT)).await;
        until(c, &mut seen, |t| {
            t.field.hp_bars.len() > bars || t.field.killed.contains(&boss)
        })
        .await;
    }
    assert!(seen.field.killed.contains(&boss), "{seen:?}");
    let buff = holiday_win_buff(EASY);
    until(c, &mut seen, |t| {
        clears(t) == 2 && t.buffs.iter().any(|&(id, _, _)| id == -buff)
    })
    .await;
    for player in team[1..].iter_mut() {
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            t.buffs.iter().any(|&(id, _, _)| id == -buff)
        })
        .await;
    }

    // The leader's Force of the Spirit buys a gift.
    let t = told(&mut team[0], talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_WON), "{t:?}");
    let t = told(&mut team[0], answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(SNOW_SPIRIT, MENU)], "{t:?}");
    let t = told(&mut team[0], answer(MENU, 1, Some(0))).await;
    let ((potion, _), (hat, _)) = HOLIDAY_GIFTS[0];
    assert!(got(&t, potion) && got(&t, hat), "{t:?}");

    // Then a prize of the difficulty's, once.
    let t = told(&mut team[0], talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_WON), "{t:?}");
    let t = told(&mut team[0], answer(0, 1, None)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_PRIZE), "{t:?}");
    let t = told(&mut team[0], answer(0, 1, None)).await;
    let prizes = HOLIDAY_REWARDS[usize::from(EASY) - 1];
    assert!(prizes.iter().any(|&(id, _)| got(&t, id)), "{t:?}");
    let t = told(&mut team[0], talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_WON), "{t:?}");
    let t = told(&mut team[0], answer(0, 1, None)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_THANKS), "{t:?}");
}
