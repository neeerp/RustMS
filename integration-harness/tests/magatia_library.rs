//! Romeo and Juliet's party quest, its first two stages: a side's library
//! hides, among its 26 books, 7 bags of mesos, 7 lessons worth EXP, its
//! side's letter and the switch, each found once. The way on stays shut
//! until the switch is found, which lights the library's `d00` and clears
//! the first stage; the hall beyond keeps its way on shut until all 42 of
//! its mobs are dead, and the last kill clears the second stage.

use game_data::server_content::keys;
use integration_harness::events::{armed_in, cleared, maps, party, talk, talk_to, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, Trace};
use integration_harness::WorldSession;
use net::events::{EventKind, MagatiaSide, ZENUMIST_LOBBY};
use net::npc_scripts::{ROMEO, ZENUMIST_BOOK};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PINK: u8 = 5;
/// The hall's mobs.
const HALL: [i32; 2] = [9300145, 9300146];

fn text(key: &str, args: &[(&str, &str)]) -> String {
    net::content::text().get(key, args)
}

/// Four at level 75 in Romeo's room, partied up and taken in.
async fn four_in() -> Vec<WorldSession> {
    let event = EventKind::Magatia(MagatiaSide::Zenumist).event();
    let mut team = Vec::new();
    for _ in 0..4 {
        team.push(armed_in(ZENUMIST_LOBBY, 75).await);
    }
    party(&mut team).await;
    let mut seen = None;
    for _ in 0..60 {
        told(&mut team[0], talk(ZENUMIST_LOBBY, ROMEO)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == ROMEO) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Romeo never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&event.entry)
        })
        .await;
    }
    team
}

/// CHANGE_MAP_SPECIAL: walk to `map`'s portal `name` and press Up; what
/// came of it.
async fn take(player: &mut WorldSession, map: i32, name: &str) -> Trace {
    let portal = maps().field(map).unwrap().portal_by_name(name).unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    let mut trace = Trace::default();
    let c = &mut player.connection;
    send(c, p).await;
    until(c, &mut trace, |t| {
        !t.warps.is_empty() || !t.broadcasts.is_empty()
    })
    .await;
    read_for(c, &mut trace, 300).await;
    trace
}

/// The way on is shut, in pink.
fn shut(trace: &Trace) -> bool {
    let shut = text(keys::MAGATIA_PORTAL_SHUT, &[]);
    trace.warps.is_empty() && trace.broadcasts.contains(&(PINK, shut))
}

/// Open the book that is the `n`th NPC on the library's list and close
/// it; what it said. What came of it goes into `seen`.
async fn open(player: &mut WorldSession, n: usize, seen: &mut Trace) -> Trace {
    let said = told(player, talk_to(1_000_000_000 + n as i32)).await;
    let closed = told(player, answer(0, 1, None)).await;
    for trace in [&said, &closed] {
        seen.meso_gains.extend(&trace.meso_gains);
        seen.gains.extend(&trace.gains);
        seen.added.extend(&trace.added);
        seen.field.reactor_hits.extend(&trace.field.reactor_hits);
        seen.field_effects
            .extend(trace.field_effects.iter().cloned());
    }
    said
}

#[tokio::test]
async fn the_switch_among_the_books_and_the_halls_last_kill_open_the_ways_on() {
    let stages = MagatiaSide::Zenumist.stages();
    let mut team = four_in().await;

    // Before the switch is found, the library's way on is shut.
    assert!(shut(&take(&mut team[0], stages[0], "pt00").await));

    // Every book holds something once, or nothing; one is the switch.
    let books: Vec<usize> = (maps().field(stages[0]).unwrap().map_npcs.iter())
        .enumerate()
        .filter(|(_, npc)| npc.npc_id == ZENUMIST_BOOK)
        .map(|(n, _)| n)
        .collect();
    assert_eq!(books.len(), 26);
    let nothing = text(keys::MAGATIA_BOOK_NOTHING, &[]);
    let mesos = text(keys::MAGATIA_BOOK_MESOS, &[("mesos", "500")]);
    let exp = text(keys::MAGATIA_BOOK_EXP, &[("exp", "500")]);
    let letter = text(keys::MAGATIA_BOOK_LETTER, &[]);
    let pass = text(keys::MAGATIA_BOOK_PASS, &[]);
    let mut said = Vec::new();
    let mut seen = Trace::default();
    for &n in &books {
        let trace = open(&mut team[0], n, &mut seen).await;
        assert_eq!(trace.prompts, [(ZENUMIST_BOOK, 0)], "{trace:?}");
        said.push(trace.prompt_texts[0].clone());
    }
    let count = |line: &String| said.iter().filter(|s| *s == line).count();
    assert_eq!(
        [&nothing, &mesos, &exp, &letter, &pass].map(count),
        [10, 7, 7, 1, 1],
        "{said:?}"
    );
    assert_eq!(seen.meso_gains, [500; 7]);
    assert!(seen.gains.iter().filter(|&&g| g == 500).count() >= 7);
    assert!(
        (seen.added.iter()).any(|&(_, _, item_id)| item_id == 4_001_131),
        "{:?}",
        seen.added
    );
    // The switch lights the library's `d00` and clears the stage.
    until(&mut team[0].connection, &mut seen, |t| {
        t.field.reactor_hits.iter().any(|&(_, state)| state == 1) && cleared(t)
    })
    .await;
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, cleared).await;
    }
    // A book once found holds nothing more.
    let again = open(&mut team[0], books[0], &mut Trace::default()).await;
    assert_eq!(again.prompt_texts, [nothing]);

    // Now the way on leads two of them into the hall, whose own way on
    // stays shut while its mobs live.
    let mut seen = take(&mut team[0], stages[0], "pt00").await;
    assert_eq!(seen.warps, [stages[1]], "{seen:?}");
    let went = take(&mut team[1], stages[0], "pt00").await;
    assert_eq!(went.warps, [stages[1]], "{went:?}");
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        HALL.iter()
            .map(|&id| t.field.alive(id).len())
            .sum::<usize>()
            == 42
    })
    .await;
    assert!(shut(&take(&mut team[0], stages[1], "pt00").await));

    // The last kill clears the stage, for 10000 EXP each.
    let c = &mut team[0].connection;
    let hall: Vec<i32> = (HALL.iter()).flat_map(|&id| seen.field.alive(id)).collect();
    for &oid in &hall {
        send(c, attack(oid, 20_000)).await;
        send(c, attack(oid, 20_000)).await;
    }
    until(c, &mut seen, |t| {
        hall.iter().all(|oid| t.field.killed.contains(oid)) && cleared(t)
    })
    .await;
    // The two left in the library get the EXP without the hall's effect.
    until(&mut team[0].connection, &mut seen, |t| {
        t.gains.contains(&10_000)
    })
    .await;
    for (n, player) in team.iter_mut().enumerate().skip(1) {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.gains.contains(&10_000)
        })
        .await;
        read_for(&mut player.connection, &mut trace, 300).await;
        assert_eq!(cleared(&trace), n == 1, "{:?}", trace.field_effects);
    }
    let went = take(&mut team[0], stages[1], "pt00").await;
    assert_eq!(went.warps, [stages[2]], "{went:?}");
}
