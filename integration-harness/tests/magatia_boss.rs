//! Romeo and Juliet's party quest, its last stage and the rooms after: the
//! corridor's way leads into the boss's room, where Yulete waits; talked
//! to, he brings out his Frankenroid, telling everyone. Its fall wins the
//! run, paying each member by whether the escort fainted, sends the guards
//! away, and brings out Romeo or Juliet, who give the side's gift and send
//! each member on; in the last room each is thanked and rewarded on the
//! way home.

use game_data::server_content::keys;
use integration_harness::events::talk_to;
use integration_harness::events::{armed_in, armed_with, cleared, maps, party, talk, told};
use integration_harness::hunting::{attack, move_mob, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{EventKind, MagatiaSide, MAGATIA_REWARDS};
use net::npc_scripts::{magatia_hub_host, yulete, JULIET, MAGATIA_THANKS, ROMEO};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// The boss room's guards.
const GUARD: i32 = 9300150;
const PINK: u8 = 5;

fn text(key: &str) -> String {
    net::content::text().get(key, &[])
}

/// Four at level 75 in `side`'s lobby, partied up and taken in by `host`.
async fn four_in(side: MagatiaSide, host: i32) -> Vec<WorldSession> {
    let event = EventKind::Magatia(side).event();
    let mut team = vec![armed_with(event.lobby, 75, &[]).await];
    for _ in 0..3 {
        team.push(armed_in(event.lobby, 75).await);
    }
    party(&mut team).await;
    let mut seen = None;
    for _ in 0..60 {
        told(&mut team[0], talk(event.lobby, host)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == host) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "{host} never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&event.entry)
        })
        .await;
    }
    team
}

/// MOB_DAMAGE_MOB_FRIENDLY as the v83 client sends it: mob `attacker` ran
/// into the friendly mob `victim` near `character_id`.
fn mob_damage_friendly(attacker: i32, character_id: i32, victim: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MobDamageMobFriendly as i16)
        .unwrap();
    p.write_int(attacker).unwrap();
    p.write_int(character_id).unwrap();
    p.write_int(victim).unwrap();
    p
}

/// The leader goes from the corridor into `side`'s boss room and waits
/// for Yulete, the escort and the guards; what was seen.
async fn into_the_lab(leader: &mut WorldSession, side: MagatiaSide) -> Trace {
    let lab = side.stages()[6];
    let corridor = lab - 1;
    let mut seen = Trace::default();
    let c = &mut leader.connection;
    assert_eq!(warp(c, corridor, "", &mut seen).await, corridor);
    // Its portal `pt00` runs `rnj12_in` (`jnr12_in`).
    let name = "pt00";
    let portal = maps()
        .field(corridor)
        .unwrap()
        .portal_by_name(name)
        .unwrap();
    send(c, walk_to(portal.x as i16, portal.y as i16)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    let mut seen = Trace::default();
    send(c, p).await;
    until(c, &mut seen, |t| {
        t.warps.contains(&lab)
            && t.field.npcs.iter().any(|&(_, id)| id == yulete(side))
            && !t.field.alive(side.escort()).is_empty()
            && t.field.alive(GUARD).len() == 2
    })
    .await;
    assert_eq!(seen.portal_sounds, 1, "{seen:?}");
    seen
}

/// The object id of the NPC `npc` the trace saw.
fn npc(seen: &Trace, npc: i32) -> i32 {
    let (oid, _) = *(seen.field.npcs.iter())
        .find(|&&(_, id)| id == npc)
        .unwrap_or_else(|| panic!("{npc}: {:?}", seen.field.npcs));
    oid
}

/// Yulete is talked to and brings out his Frankenroid `boss`, which every
/// member hears of; the leader kills it, which wins the run for `exp`
/// each, sends the guards away and brings out the side's host.
async fn beat_the_frankenroid(
    team: &mut [WorldSession],
    side: MagatiaSide,
    seen: &mut Trace,
    exp: i32,
) {
    let said = told(&mut team[0], talk_to(npc(seen, yulete(side)))).await;
    assert_eq!(
        said.prompt_texts,
        [text(keys::MAGATIA_YULETE_PRESENTS)],
        "{said:?}"
    );
    send(&mut team[0].connection, answer(0, 1, None)).await;
    // It and the line may come before the talk's own trace ends.
    let boss = side.frankenroid(false);
    let called = (PINK, text(keys::MAGATIA_FRANKENROID));
    let c = &mut team[0].connection;
    if !said.broadcasts.contains(&called) {
        until(c, seen, |t| t.broadcasts.contains(&called)).await;
    }
    read_for(c, seen, 300).await;
    let frankenroid = *(said.field.alive(boss).iter())
        .chain(&seen.field.alive(boss))
        .next()
        .unwrap_or_else(|| panic!("no Frankenroid: {said:?}"));
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.broadcasts.contains(&called)
        })
        .await;
    }
    // Asked again, he bids them behold it.
    let again = told(&mut team[0], talk_to(npc(seen, yulete(side)))).await;
    assert_eq!(
        again.prompt_texts,
        [text(keys::MAGATIA_YULETE_BEHOLD)],
        "{again:?}"
    );
    send(&mut team[0].connection, answer(0, 1, None)).await;

    // A hit is capped by the leader's stats, so it takes a few.
    let guards = seen.field.alive(GUARD);
    let c = &mut team[0].connection;
    for _ in 0..300 {
        if seen.field.killed.contains(&frankenroid) {
            break;
        }
        send(c, attack(frankenroid, 20_000)).await;
        read_for(c, seen, 20).await;
    }
    until(c, seen, |t| {
        t.field.killed.contains(&frankenroid)
            && cleared(t)
            && t.gains.contains(&exp)
            && guards.iter().all(|oid| t.field.killed.contains(oid))
            && t.field
                .npcs
                .iter()
                .any(|&(_, id)| id == magatia_hub_host(side))
    })
    .await;
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.gains.contains(&exp)
        })
        .await;
    }
    // No more guards come.
    read_for(&mut team[0].connection, seen, 1500).await;
    assert!(seen.field.alive(GUARD).is_empty(), "{:?}", seen.field.mobs);
}

/// In the last room, where `seen` began, the one who thanks the party says
/// `key` and gives one of the rewards on the way back to `side`'s lobby.
async fn thanked(player: &mut WorldSession, side: MagatiaSide, mut seen: Trace, key: &str) {
    until(&mut player.connection, &mut seen, |t| {
        t.field.npcs.iter().any(|&(_, id)| id == MAGATIA_THANKS)
    })
    .await;
    let said = told(player, talk_to(npc(&seen, MAGATIA_THANKS))).await;
    assert_eq!(said.prompt_texts, [text(key)], "{said:?}");
    let lobby = EventKind::Magatia(side).event().lobby;
    let mut home = Trace::default();
    let c = &mut player.connection;
    send(c, answer(0, 1, None)).await;
    until(c, &mut home, |t| t.warps.contains(&lobby)).await;
    let rewards: Vec<i32> = MAGATIA_REWARDS.iter().map(|&(id, _)| id).collect();
    assert!(
        home.added
            .iter()
            .any(|&(_, _, item)| rewards.contains(&item)),
        "{home:?}"
    );
}

#[tokio::test]
async fn the_frankenroid_falls_and_romeo_sends_the_party_home_with_gifts() {
    let side = MagatiaSide::Zenumist;
    let mut team = four_in(side, ROMEO).await;
    let mut seen = into_the_lab(&mut team[0], side).await;
    beat_the_frankenroid(&mut team, side, &mut seen, 105_000).await;

    // Won, Yulete is beaten.
    let beaten = told(&mut team[0], talk_to(npc(&seen, yulete(side)))).await;
    assert_eq!(
        beaten.prompt_texts,
        [text(keys::MAGATIA_YULETE_BEATEN)],
        "{beaten:?}"
    );
    send(&mut team[0].connection, answer(0, 1, None)).await;

    // Romeo gives the gift and, Yulete not having been caught, sends the
    // leader straight to the last room.
    let host = npc(&seen, magatia_hub_host(side));
    let saved = told(&mut team[0], talk_to(host)).await;
    assert_eq!(
        saved.prompt_texts,
        [text(keys::MAGATIA_ROMEO_SAVED)],
        "{saved:?}"
    );
    let gift = told(&mut team[0], answer(0, 1, None)).await;
    assert_eq!(gift.prompt_texts, [text(keys::MAGATIA_GIFT)], "{gift:?}");
    let mut on = Trace::default();
    let c = &mut team[0].connection;
    send(c, answer(0, 1, None)).await;
    until(c, &mut on, |t| {
        t.warps.contains(&side.thanks_room())
            && t.added.iter().any(|&(_, _, item)| item == side.gift())
    })
    .await;
    thanked(&mut team[0], side, on, keys::MAGATIA_THANKS_REHAB).await;
}

#[tokio::test]
async fn juliet_fainting_costs_the_run_its_gifts_and_some_exp() {
    let side = MagatiaSide::Alcadno;
    let mut team = four_in(side, JULIET).await;
    let mut seen = into_the_lab(&mut team[0], side).await;

    // A guard the leader controls is walked into the escort until it
    // faints, which the room hears.
    let escort = seen.field.alive(side.escort())[0];
    let at = seen.field.mob_positions[&escort];
    let mut attacker = None;
    for _ in 0..10 {
        for guard in seen.field.alive(GUARD) {
            let c = &mut team[0].connection;
            send(c, move_mob(guard, at, false, 0, 0)).await;
            read_for(c, &mut seen, 100).await;
            if seen.field.move_responses.contains(&guard) {
                attacker = Some(guard);
                break;
            }
        }
        if attacker.is_some() {
            break;
        }
    }
    let attacker = attacker.expect("the leader controls a guard");
    let fainted = (text(keys::MAGATIA_ROMEO_FAINTED), 0);
    let character_id = team[0].character_id;
    let c = &mut team[0].connection;
    for _ in 0..2000 {
        if seen.field.killed.contains(&escort) {
            break;
        }
        send(c, mob_damage_friendly(attacker, character_id, escort)).await;
        read_for(c, &mut seen, 20).await;
    }
    until(c, &mut seen, |t| t.item_lines.contains(&fainted)).await;
    assert_eq!(seen.field.leave_types[&escort], 0);

    beat_the_frankenroid(&mut team, side, &mut seen, 90_000).await;

    // Juliet, hurt, gives nothing and sends the leader to the last room.
    let host = npc(&seen, magatia_hub_host(side));
    let saved = told(&mut team[0], talk_to(host)).await;
    let hurt = text(keys::MAGATIA_JULIET_SAVED_HURT);
    assert_eq!(saved.prompt_texts, [hurt], "{saved:?}");
    let mut on = Trace::default();
    let c = &mut team[0].connection;
    send(c, answer(0, 1, None)).await;
    until(c, &mut on, |t| t.warps.contains(&side.thanks_room())).await;
    assert!(!on.added.iter().any(|&(_, _, item)| item == side.gift()));
    thanked(&mut team[0], side, on, keys::MAGATIA_THANKS_JAIL).await;
}
