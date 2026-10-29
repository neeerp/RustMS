//! The Holiday PQ's snowman: in the arena no mob comes until the run's
//! leader has the Snow Spirit bring the snowman out, as everyone hears;
//! then Scrooge's men come, and now and then (always after five kills that
//! left none) one leaves Snow Vigor, real or fake, for anyone. Dropped by
//! the snowman, a fake shrinks it, as everyone hears, and a real one heals
//! it or grows it into its next form; mobs that run into it melt it back a
//! form, and once it is whole every mob goes and the stage clears. The harness world runs events
//! 60 times as fast (`RUSTMS_EVENT_SCALE`), so the clock reads seconds.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::events::{armed_with, enter_holiday, party, said, talk, throw_down, told};
use integration_harness::events::{cleared, waiting_in};
use integration_harness::hunting::{attack, move_mob, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, Trace};
use integration_harness::WorldSession;
use net::events::{
    holiday_snowman, FAKE_SNOW_VIGOR, HOLIDAY, HOLIDAY_FORMS, HOLIDAY_SNOWMAN_AT, SNOW_VIGOR,
};
use net::npc_scripts::SNOW_SPIRIT;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const EASY: u8 = 1;
const HARD: u8 = 3;
const PINK: u8 = 5;
const HIT: i32 = 5_000;

/// MOB_DAMAGE_MOB_FRIENDLY as the v83 client sends it (0x667e40): mob
/// `attacker` ran into the friendly mob `victim` near `character_id`.
fn mob_damage_friendly(attacker: i32, character_id: i32, victim: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MobDamageMobFriendly as i16)
        .unwrap();
    p.write_int(attacker).unwrap();
    p.write_int(character_id).unwrap();
    p.write_int(victim).unwrap();
    p
}

fn line(key: &str) -> (u8, String) {
    (PINK, net::content::text().get(key, &[]))
}

/// The snowman's mob ids in difficulty `level`.
fn snowmen(level: u8) -> Vec<i32> {
    (1..=HOLIDAY_FORMS)
        .map(|form| holiday_snowman(level, form))
        .collect()
}

/// A party of three of difficulty `level`'s levels, the leader holding
/// `items`, in its arena: the team, and the leader's trace since.
async fn in_the_arena(level: u8, items: &[(i32, i16)]) -> (Vec<WorldSession>, Trace) {
    let event = &HOLIDAY[usize::from(level) - 1];
    let low = *event.levels.start() as i16;
    let mut team = vec![armed_with(event.lobby, low, items).await];
    for at in [low + 4, low + 9] {
        team.push(waiting_in(event.lobby, at, &[]).await);
    }
    party(&mut team).await;
    let mut seen = enter_holiday(&mut team, level).await;
    for (player, trace) in team.iter_mut().zip(seen.iter_mut()) {
        until(&mut player.connection, trace, |t| {
            t.warps.contains(&event.entry)
        })
        .await;
    }
    (team, seen.remove(0))
}

/// The leader has the Snow Spirit bring the snowman out: its first form,
/// out in `seen`.
async fn bring_out(leader: &mut WorldSession, seen: &mut Trace, level: u8) -> i32 {
    let entry = HOLIDAY[usize::from(level) - 1].entry;
    let t = told(leader, talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_INTRO), "{t:?}");
    send(&mut leader.connection, answer(0, 1, None)).await;
    let first = holiday_snowman(level, 1);
    let out = line(keys::HOLIDAY_SNOWMAN_OUT);
    until(&mut leader.connection, seen, |t| {
        !t.field.alive(first).is_empty() && t.broadcasts.contains(&out)
    })
    .await;
    seen.field.alive(first)[0]
}

/// The leader stands by the snowman, where it eats Snow Vigor.
async fn by_the_snowman(c: &mut MapleTestConnection) {
    let (x, y) = HOLIDAY_SNOWMAN_AT;
    send(c, walk_to(x, y)).await;
}

/// Mobs alive in `seen` that aren't the snowman.
fn scrooges_men(seen: &Trace, level: u8) -> Vec<i32> {
    let snowmen = snowmen(level);
    (seen.field.mobs.iter())
        .filter(|(oid, mob)| !snowmen.contains(mob) && !seen.field.killed.contains(oid))
        .map(|&(oid, _)| oid)
        .collect()
}

#[tokio::test]
async fn the_leader_brings_out_the_snowman_and_scrooges_men_leave_snow_vigor() {
    let items = [(FAKE_SNOW_VIGOR, 2)];
    let (mut team, mut seen) = in_the_arena(EASY, &items).await;

    // No mob comes before the snowman.
    read_for(&mut team[0].connection, &mut seen, 1_000).await;
    assert!(seen.field.mobs.is_empty(), "{seen:?}");

    // Another hears only the leader may begin.
    let entry = HOLIDAY[0].entry;
    told(&mut team[1], talk(entry, SNOW_SPIRIT)).await;
    let t = told(&mut team[1], answer(0, 1, None)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_LEADER), "{t:?}");
    assert!(t.field.mobs.is_empty(), "{t:?}");

    // The leader brings it out, and Scrooge's men come.
    let snowman = bring_out(&mut team[0], &mut seen, EASY).await;
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| !scrooges_men(t, EASY).is_empty()).await;

    // Six kills leave Snow Vigor, real or fake, for anyone, where the mob
    // fell.
    let c = &mut team[0].connection;
    let tokens = |t: &Trace| {
        (t.field.drops.iter())
            .filter(|d| !d.meso && [SNOW_VIGOR, FAKE_SNOW_VIGOR].contains(&d.value))
            .count()
    };
    for _ in 0..6 {
        if tokens(&seen) > 0 {
            break;
        }
        until(c, &mut seen, |t| !scrooges_men(t, EASY).is_empty()).await;
        let mob = scrooges_men(&seen, EASY)[0];
        for _ in 0..20 {
            if seen.field.killed.contains(&mob) {
                break;
            }
            let bars = seen.field.hp_bars.len();
            send(c, attack(mob, HIT)).await;
            until(c, &mut seen, |t| {
                t.field.hp_bars.len() > bars || t.field.killed.contains(&mob)
            })
            .await;
        }
        assert!(seen.field.killed.contains(&mob), "{seen:?}");
        read_for(c, &mut seen, 100).await;
    }
    let token = (seen.field.drops.iter())
        .find(|d| [SNOW_VIGOR, FAKE_SNOW_VIGOR].contains(&d.value))
        .copied();
    let token = token.unwrap_or_else(|| panic!("six kills left no Snow Vigor: {seen:?}"));
    assert_eq!(token.ownership, 2, "{token:?}");

    // A fake dropped by it shrinks the snowman, as everyone hears.
    by_the_snowman(c).await;
    send(c, throw_down(1, 1)).await;
    let fake = line(keys::HOLIDAY_FAKE_VIGOR);
    until(c, &mut seen, |t| t.broadcasts.contains(&fake)).await;
    assert!(!seen.field.killed.contains(&snowman), "{seen:?}");

    // Now the Snow Spirit only urges it grown.
    let t = told(&mut team[1], talk(entry, SNOW_SPIRIT)).await;
    assert!(said(&t, keys::HOLIDAY_ARENA_GROW), "{t:?}");
}

#[tokio::test]
async fn snow_vigor_grows_the_snowman_whole_unless_scrooges_men_melt_it() {
    let items = [(SNOW_VIGOR, 60)];
    let (mut team, mut seen) = in_the_arena(HARD, &items).await;
    let snowmen = snowmen(HARD);
    bring_out(&mut team[0], &mut seen, HARD).await;
    let c = &mut team[0].connection;
    by_the_snowman(c).await;

    // Six grow its first form into its second (2 more than 3 times 1, and
    // the one that grows it).
    for _ in 0..6 {
        send(c, throw_down(1, 1)).await;
    }
    until(c, &mut seen, |t| !t.field.alive(snowmen[1]).is_empty()).await;
    assert!(seen.field.alive(snowmen[0]).is_empty(), "{seen:?}");
    let second = seen.field.alive(snowmen[1])[0];
    let at = seen.field.mob_positions[&second];

    // A mob its controller walks into it melts it back to its first form,
    // as everyone hears.
    let mut traces = vec![std::mem::take(&mut seen)];
    for player in team[1..].iter_mut() {
        let mut t = Trace::default();
        read_for(&mut player.connection, &mut t, 200).await;
        traces.push(t);
    }
    let mut controller = None;
    for _ in 0..50 {
        for (i, (player, t)) in team.iter_mut().zip(traces.iter_mut()).enumerate() {
            read_for(&mut player.connection, t, 20).await;
            let men = scrooges_men(t, HARD);
            if let Some(&oid) = t.field.controlled.iter().find(|oid| men.contains(oid)) {
                controller = Some((i, oid));
            }
        }
        if controller.is_some() {
            break;
        }
    }
    let (i, attacker) = controller.expect("a player controls one of Scrooge's men");
    let character_id = team[i].character_id;
    let c = &mut team[i].connection;
    send(c, move_mob(attacker, at, false, 0, 0)).await;
    let t = &mut traces[i];
    for _ in 0..1_000 {
        if t.field.killed.contains(&second) {
            break;
        }
        send(c, mob_damage_friendly(attacker, character_id, second)).await;
        read_for(c, t, 20).await;
    }
    assert!(t.field.killed.contains(&second), "{t:?}");
    let melted = line(keys::HOLIDAY_SNOWMAN_MELTED);
    let mut seen = traces.swap_remove(0);
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        !t.field.alive(snowmen[0]).is_empty() && t.broadcasts.contains(&melted)
    })
    .await;

    // From there, 6, 9, 12 and 15 make it whole: Scrooge's men go and the
    // stage clears.
    for _ in 0..42 {
        send(c, throw_down(1, 1)).await;
    }
    until(c, &mut seen, |t| {
        !t.field.alive(snowmen[4]).is_empty() && cleared(t)
    })
    .await;
    for form in &snowmen[..4] {
        assert!(seen.field.alive(*form).is_empty(), "{seen:?}");
    }
    until(c, &mut seen, |t| scrooges_men(t, HARD).is_empty()).await;
    read_for(c, &mut seen, 1_000).await;
    assert!(scrooges_men(&seen, HARD).is_empty(), "{seen:?}");
}
