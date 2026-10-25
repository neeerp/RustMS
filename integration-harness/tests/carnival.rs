//! The Monster Carnival. Spiegelmann on the Carnival Field sends a party's
//! leader and their party into one of six rooms' waiting rooms, where the
//! next party's leader to pick it challenges them. The harness world lets a
//! party of one play (`RUSTMS_CARNIVAL_SOLO`) and runs events 60 times as
//! fast, so the 3 minutes' wait for a challenge take 3 seconds, the 10
//! before the battle a sixth of one, and the battle's 610 about 10. Once
//! accepted, both parties go to the battle field, where each member's
//! screen is told their side and the score, and a kill earns its mob's CP
//! for the killer and their side, which they spend on mobs that hunt the
//! other side, skills that fall on it, and guardians that strengthen their
//! own side's mobs until the other side breaks them. When time runs out
//! everyone goes to the exit room.

use integration_harness::events::{armed_in, party_of_one, talk, told};
use integration_harness::hunting::{attack, hit_reactor, walk_to};
use integration_harness::protocol::WorldSession;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, Trace};
use net::carnival::{battle_map, room_map, CARNIVAL_EXIT, CARNIVAL_LOBBY};
use net::npc_scripts::{CARNIVAL_ROOM_NPC, SPIEGELMANN};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

/// A mob worth 1 CP, which Maple Blue summons for Maple Red to hunt.
const BLUE_MOB: i32 = 9300127;

/// The Monster Carnival packets with opcode `op`, by their bodies.
fn carnival(seen: &Trace, op: SendOpcode) -> Vec<Vec<u8>> {
    let op = op as i16;
    let of = seen.carnival.iter().filter(|(of, _)| *of == op);
    of.map(|(_, body)| body.clone()).collect()
}

/// One of the field's Maple Blue mobs worth 1 CP.
fn blue_mob(seen: &Trace) -> Option<i32> {
    let mut mobs = seen.field.mobs.iter();
    let blue = |oid: &i32| seen.field.teams.get(oid) == Some(&1);
    mobs.find(|(oid, id)| *id == BLUE_MOB && blue(oid))
        .map(|&(oid, _)| oid)
}

/// MONSTER_CARNIVAL: spend CP on `index` of the window's tab `tab`.
fn spend(tab: u8, index: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MonsterCarnival as i16).unwrap();
    p.write_byte(tab).unwrap();
    p.write_byte(index).unwrap();
    p
}

/// The bodies of the Monster Carnival packets seen since the first `from`.
fn since(seen: &Trace, op: SendOpcode, from: usize) -> Vec<Vec<u8>> {
    carnival(seen, op).split_off(from)
}

/// SUMMON's body: `name` spent CP on `index` of tab `tab`.
fn summoned(tab: u8, index: u8, name: &str) -> Vec<u8> {
    let length = (name.len() as u16).to_le_bytes();
    [vec![tab, index], length.to_vec(), name.as_bytes().to_vec()].concat()
}

/// Little-endian shorts.
fn shorts(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn text(key: &str) -> String {
    net::content::text().get(key, &[])
}

/// A party of one's leader on the Carnival Field.
async fn leader() -> WorldSession {
    let mut player = armed_in(CARNIVAL_LOBBY, 40).await;
    party_of_one(&mut player).await;
    player
}

/// `player` asks Spiegelmann to Join and picks room `room`, sending the
/// pick; the trace from there on.
async fn pick(player: &mut WorldSession, room: u8) -> Trace {
    told(player, talk(CARNIVAL_LOBBY, SPIEGELMANN)).await;
    told(player, answer(4, 1, Some(0))).await;
    let mut seen = Trace::default();
    let index = i32::from(room) - 1;
    send(&mut player.connection, answer(4, 1, Some(index))).await;
    until(&mut player.connection, &mut seen, |t| {
        !t.prompts.is_empty() || !t.warps.is_empty()
    })
    .await;
    seen
}

/// `player`'s party takes room `room` and waits there, told so.
async fn take(player: &mut WorldSession, room: u8) -> Trace {
    let mut seen = pick(player, room).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&room_map(room))
            && t.item_lines
                .iter()
                .any(|(line, _)| *line == text("carnival.waiting"))
    })
    .await;
    seen
}

/// `waiting`'s party is challenged by `challenger`'s for room `room`, and
/// the challenger hears it was put.
async fn challenge(waiting: &mut WorldSession, challenger: &mut WorldSession, room: u8) {
    let put = pick(challenger, room).await;
    assert!(
        put.prompt_texts
            .iter()
            .any(|said| *said == text("carnival.spiegelmann.sent")),
        "{:?}",
        put.prompt_texts
    );
    let mut asked = Trace::default();
    until(&mut waiting.connection, &mut asked, |t| {
        t.prompts.iter().any(|&(npc, _)| npc == CARNIVAL_ROOM_NPC)
    })
    .await;
    let name = &challenger.character_name;
    assert!(asked
        .prompt_texts
        .iter()
        .any(|said| said.contains(name.as_str())));
}

#[tokio::test]
async fn a_party_not_challenged_goes_back_to_the_carnival_field() {
    let player = &mut leader().await;
    let mut seen = take(player, 3).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&CARNIVAL_LOBBY)
    })
    .await;
    assert!(!seen.countdowns.is_empty());
    assert!(!seen.warps.contains(&CARNIVAL_EXIT));
}

#[tokio::test]
async fn a_declined_challenge_tells_the_challenger() {
    let red = &mut leader().await;
    let blue = &mut leader().await;
    take(red, 2).await;
    challenge(red, blue, 2).await;

    send(&mut red.connection, answer(1, 0, None)).await;
    let mut told_no = Trace::default();
    let declined = text("carnival.challenge.declined");
    until(&mut blue.connection, &mut told_no, |t| {
        t.broadcasts.contains(&(5, declined.clone()))
    })
    .await;
    assert!(!told_no.warps.contains(&room_map(2)));
}

#[tokio::test]
async fn a_challenged_party_battles_for_cp_until_its_time_runs_out() {
    let red = &mut leader().await;
    let blue = &mut leader().await;
    take(red, 1).await;
    challenge(red, blue, 1).await;

    // Yes: the challengers come into the waiting room, and both sides go
    // on to the battle field with its clock.
    let mut seen = Trace::default();
    send(&mut red.connection, answer(1, 1, None)).await;
    let mut theirs = Trace::default();
    let field = battle_map(1);
    let started = |t: &Trace| {
        t.warps.contains(&field)
            && !t.countdowns.is_empty()
            && !carnival(t, SendOpcode::CarnivalStart).is_empty()
    };
    until(&mut blue.connection, &mut theirs, |t| started(t)).await;
    assert!(theirs.warps.contains(&room_map(1)));
    until(&mut red.connection, &mut seen, |t| {
        started(t)
            && blue_mob(t).is_some()
            && t.players
                .iter()
                .any(|p| p.character_id == blue.character_id)
    })
    .await;
    assert!(
        seen.countdowns.last().is_some_and(|&left| left <= 11),
        "{:?}",
        seen.countdowns
    );

    // Each side is told theirs, and the challengers show as Maple Blue.
    let heard = |t: &Trace, key| t.item_lines.iter().any(|(line, _)| *line == text(key));
    assert!(heard(&seen, "carnival.team.red"));
    assert!(heard(&theirs, "carnival.team.blue"));
    let challenger = seen
        .players
        .iter()
        .rev()
        .find(|p| p.character_id == blue.character_id);
    assert_eq!(challenger.map(|p| p.team), Some(1));

    // Each screen shows its side with no CP yet, and none of the field's
    // mobs summoned.
    let summonable = integration_harness::events::maps()
        .field(field)
        .and_then(|f| f.carnival.as_ref())
        .map_or(0, |rules| rules.mobs.len());
    assert!(summonable > 0);
    let start = |side| [vec![side], shorts(&[0; 6]), vec![0; summonable]].concat();
    assert_eq!(carnival(&seen, SendOpcode::CarnivalStart), [start(0)]);
    assert_eq!(carnival(&theirs, SendOpcode::CarnivalStart), [start(1)]);

    // The field's own mobs play for both sides.
    let mob = blue_mob(&seen).unwrap();
    assert!(seen.field.teams.values().any(|&team| team == 0));

    // Killing a Maple Blue mob earns its CP for the player and Maple Red,
    // which both sides see.
    send(&mut red.connection, attack(mob, 30_000)).await;
    until(&mut red.connection, &mut seen, |t| {
        t.field.killed.contains(&mob) && !carnival(t, SendOpcode::CarnivalPartyCp).is_empty()
    })
    .await;
    assert_eq!(
        carnival(&seen, SendOpcode::CarnivalObtainedCp),
        [shorts(&[1, 1])]
    );
    let red_cp = [vec![0], shorts(&[1, 1])].concat();
    assert_eq!(
        carnival(&seen, SendOpcode::CarnivalPartyCp),
        [red_cp.clone()]
    );
    until(&mut blue.connection, &mut theirs, |t| {
        !carnival(t, SendOpcode::CarnivalPartyCp).is_empty()
    })
    .await;
    assert_eq!(carnival(&theirs, SendOpcode::CarnivalPartyCp), [red_cp]);
    assert!(carnival(&theirs, SendOpcode::CarnivalObtainedCp).is_empty());

    // The field's first mob costs 7 CP, its first skill 17 and its first
    // guardian too: with 1 the window hears there isn't enough of any.
    let (mob_id, cost) = integration_harness::events::maps()
        .field(field)
        .and_then(|f| f.carnival.as_ref())
        .and_then(|rules| rules.mobs.first().copied())
        .unwrap();
    assert_eq!((mob_id, cost), (BLUE_MOB, 7));
    let messages = |t: &Trace| carnival(t, SendOpcode::CarnivalMessage);
    send(&mut red.connection, spend(0, 0)).await;
    until(&mut red.connection, &mut seen, |t| !messages(t).is_empty()).await;
    send(&mut red.connection, spend(1, 0)).await;
    send(&mut red.connection, spend(2, 0)).await;
    until(&mut red.connection, &mut seen, |t| messages(t).len() == 3).await;
    assert_eq!(messages(&seen), [vec![1], vec![1], vec![1]]);

    // Red hunts the rest of Maple Blue's mobs for enough CP.
    let theirs_left: Vec<i32> = seen
        .field
        .teams
        .iter()
        .filter(|&(oid, &team)| team == 1 && !seen.field.killed.contains(oid))
        .map(|(&oid, _)| oid)
        .collect();
    for &oid in &theirs_left {
        send(&mut red.connection, attack(oid, 30_000)).await;
    }
    let earned = |t: &Trace| {
        let last = carnival(t, SendOpcode::CarnivalObtainedCp).pop();
        last.map_or(0, |body| i16::from_le_bytes([body[0], body[1]]))
    };
    until(&mut red.connection, &mut seen, |t| {
        theirs_left.iter().all(|oid| t.field.killed.contains(oid))
    })
    .await;
    // The kills' CP follows them from the event.
    read_for(&mut red.connection, &mut seen, 500).await;
    let total = earned(&seen);
    assert!(total >= cost as i16, "{total}");

    // Spending it brings out a mob for Maple Red, which everyone hears
    // Red summoned, and leaves Red and their side the rest.
    let theirs_from = carnival(&theirs, SendOpcode::CarnivalSummon).len();
    let mobs_before = seen.field.mobs.len();
    let ours_out = |t: &Trace| {
        let mut new = t.field.mobs[mobs_before..].iter();
        new.any(|(oid, id)| *id == BLUE_MOB && t.field.teams.get(oid) == Some(&0))
    };
    let rest = shorts(&[total - cost as i16, total]);
    let red_rest = [vec![0], rest.clone()].concat();
    send(&mut red.connection, spend(0, 0)).await;
    until(&mut red.connection, &mut seen, |t| {
        ours_out(t)
            && carnival(t, SendOpcode::CarnivalObtainedCp).last() == Some(&rest)
            && carnival(t, SendOpcode::CarnivalPartyCp).last() == Some(&red_rest)
    })
    .await;
    let summon = [
        vec![0, 0],
        (red.character_name.len() as u16).to_le_bytes().to_vec(),
        red.character_name.as_bytes().to_vec(),
    ]
    .concat();
    assert_eq!(
        carnival(&seen, SendOpcode::CarnivalSummon),
        [summon.clone()]
    );
    until(&mut blue.connection, &mut theirs, |t| {
        carnival(t, SendOpcode::CarnivalSummon).len() > theirs_from
    })
    .await;
    assert_eq!(
        since(&theirs, SendOpcode::CarnivalSummon, theirs_from),
        [summon]
    );

    // Red's own side's mobs earn Red nothing.
    let ours = seen
        .field
        .teams
        .iter()
        .find(|&(oid, &team)| team == 0 && !seen.field.killed.contains(oid))
        .map(|(&oid, _)| oid)
        .unwrap();
    let points = carnival(&seen, SendOpcode::CarnivalObtainedCp).len();
    send(&mut red.connection, attack(ours, 30_000)).await;
    until(&mut red.connection, &mut seen, |t| {
        t.field.killed.contains(&ours)
    })
    .await;
    read_for(&mut red.connection, &mut seen, 300).await;
    assert_eq!(
        carnival(&seen, SendOpcode::CarnivalObtainedCp).len(),
        points
    );

    // Time runs out, and both go to the exit room.
    until(&mut red.connection, &mut seen, |t| {
        t.warps.contains(&CARNIVAL_EXIT)
    })
    .await;
    until(&mut blue.connection, &mut theirs, |t| {
        t.warps.contains(&CARNIVAL_EXIT)
    })
    .await;
}

/// Red's party battles Blue's in room `room`, and Red hunts the mobs not
/// on their side: what each has seen, and the CP Red earned, at least 12.
async fn hunted(red: &mut WorldSession, blue: &mut WorldSession, room: u8) -> (Trace, Trace, i16) {
    take(red, room).await;
    challenge(red, blue, room).await;
    let mut seen = Trace::default();
    send(&mut red.connection, answer(1, 1, None)).await;
    let mut theirs = Trace::default();
    let field = battle_map(room);
    let started =
        |t: &Trace| t.warps.contains(&field) && !carnival(t, SendOpcode::CarnivalStart).is_empty();
    until(&mut blue.connection, &mut theirs, |t| started(t)).await;
    until(&mut red.connection, &mut seen, |t| {
        started(t) && !t.field.mobs.is_empty()
    })
    .await;
    let earned = |t: &Trace| {
        let last = carnival(t, SendOpcode::CarnivalObtainedCp).pop();
        last.map_or(0, |body| i16::from_le_bytes([body[0], body[1]]))
    };
    // The rest of the field's mobs come out over the next packets, and the
    // killed ones respawn, so hunt in rounds until the CP is in.
    for _ in 0..6 {
        read_for(&mut red.connection, &mut seen, 500).await;
        if earned(&seen) >= 12 {
            break;
        }
        let prey: Vec<i32> = seen
            .field
            .mobs
            .iter()
            .filter(|(oid, _)| seen.field.teams.get(oid) != Some(&0))
            .map(|&(oid, _)| oid)
            .filter(|oid| !seen.field.killed.contains(oid))
            .collect();
        for &oid in &prey {
            send(&mut red.connection, attack(oid, 30_000)).await;
        }
        until(&mut red.connection, &mut seen, |t| {
            prey.iter().all(|oid| t.field.killed.contains(oid))
        })
        .await;
    }
    // The kills' CP follows them from the event.
    read_for(&mut red.connection, &mut seen, 500).await;
    let total = earned(&seen);
    assert!(total >= 12, "{total}");
    (seen, theirs, total)
}

#[tokio::test]
async fn a_carnival_skill_falls_on_the_other_side() {
    let red = &mut leader().await;
    let blue = &mut leader().await;
    // Room five's third skill is Curse, on all of the other side for 12 CP.
    let rules = integration_harness::events::maps()
        .field(battle_map(5))
        .and_then(|f| f.carnival.clone())
        .unwrap();
    assert_eq!(rules.skills.get(2), Some(&4));
    let (mut seen, mut theirs, total) = hunted(red, blue, 5).await;

    // Casting it curses Blue, everyone hears Red cast it, and Red and
    // their side keep the rest.
    let rest = shorts(&[total - 12, total]);
    send(&mut red.connection, spend(1, 2)).await;
    let cast = summoned(1, 2, &red.character_name);
    until(&mut red.connection, &mut seen, |t| {
        carnival(t, SendOpcode::CarnivalSummon).contains(&cast)
            && carnival(t, SendOpcode::CarnivalObtainedCp).last() == Some(&rest)
    })
    .await;
    let curse = 124 | 7 << 16;
    until(&mut blue.connection, &mut theirs, |t| {
        carnival(t, SendOpcode::CarnivalSummon).contains(&cast)
            && t.buffs.iter().any(|&(skill, _, _)| skill == curse)
    })
    .await;
    assert!(!seen.buffs.iter().any(|&(skill, _, _)| skill == curse));
    assert!(carnival(&seen, SendOpcode::CarnivalMessage).is_empty());
}

#[tokio::test]
async fn a_guardian_strengthens_its_side_until_the_other_breaks_it() {
    let red = &mut leader().await;
    let blue = &mut leader().await;
    // Room six's seventh guardian is Speed UP, for 12 CP.
    let rules = integration_harness::events::maps()
        .field(battle_map(6))
        .and_then(|f| f.carnival.clone())
        .unwrap();
    assert_eq!(rules.guardians.get(6), Some(&6));
    let (mut seen, mut theirs, total) = hunted(red, blue, 6).await;
    let ours: Vec<i32> = seen
        .field
        .mobs
        .iter()
        .filter(|(oid, _)| seen.field.teams.get(oid) == Some(&0))
        .map(|&(oid, _)| oid)
        .collect();
    assert!(!ours.is_empty());

    // Placing it stands Maple Red's guardian on the field, everyone hears
    // Red placed it, and Red's side's mobs speed up.
    let speed_up = 156 | 1 << 16;
    let rest = shorts(&[total - 12, total]);
    send(&mut red.connection, spend(2, 6)).await;
    let placed = summoned(2, 6, &red.character_name);
    let guardian = |t: &Trace| {
        t.field
            .reactors
            .iter()
            .find(|reactor| reactor.reactor_id == 9_980_000)
            .copied()
    };
    until(&mut red.connection, &mut seen, |t| {
        carnival(t, SendOpcode::CarnivalSummon).contains(&placed)
            && carnival(t, SendOpcode::CarnivalObtainedCp).last() == Some(&rest)
            && guardian(t).is_some()
            && ours.iter().all(|oid| {
                t.field
                    .stat_sets
                    .iter()
                    .any(|&(of, _, _, skill)| of == *oid && skill == speed_up)
            })
    })
    .await;
    let reactor = guardian(&seen).unwrap();
    assert_eq!(reactor.state, 1);
    until(&mut blue.connection, &mut theirs, |t| {
        carnival(t, SendOpcode::CarnivalSummon).contains(&placed) && guardian(t).is_some()
    })
    .await;

    // Red's own side can't break it; Blue does, in four hits, and Red's
    // mobs slow down again.
    send(&mut red.connection, walk_to(reactor.x, reactor.y)).await;
    send(&mut red.connection, hit_reactor(reactor.object_id)).await;
    read_for(&mut red.connection, &mut seen, 500).await;
    assert!(seen.field.reactor_hits.is_empty());
    send(&mut blue.connection, walk_to(reactor.x, reactor.y)).await;
    for _ in 0..4 {
        send(&mut blue.connection, hit_reactor(reactor.object_id)).await;
    }
    until(&mut red.connection, &mut seen, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == reactor.object_id)
            && ours
                .iter()
                .all(|oid| t.field.stat_resets.iter().any(|&(of, _)| of == *oid))
    })
    .await;
    assert_eq!(
        seen.field.reactor_hits,
        [
            (reactor.object_id, 2),
            (reactor.object_id, 3),
            (reactor.object_id, 4)
        ]
    );
}
