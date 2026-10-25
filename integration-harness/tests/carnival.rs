//! The Monster Carnival. Spiegelmann on the Carnival Field sends a party's
//! leader and their party into one of six rooms' waiting rooms, where the
//! next party's leader to pick it challenges them. The harness world lets a
//! party of one play (`RUSTMS_CARNIVAL_SOLO`) and runs events 60 times as
//! fast, so the 3 minutes' wait for a challenge take 3 seconds, the 10
//! before the battle a sixth of one, and the battle's 610 about 10. Once
//! accepted, both parties go to the battle field, where each member's
//! screen is told their side and the score, and a kill earns its mob's CP
//! for the killer and their side. When time runs out everyone goes to the
//! exit room.

use integration_harness::events::{armed_in, party_of_one, talk, told};
use integration_harness::hunting::attack;
use integration_harness::protocol::WorldSession;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use net::carnival::{battle_map, room_map, CARNIVAL_EXIT, CARNIVAL_LOBBY};
use net::npc_scripts::{CARNIVAL_ROOM_NPC, SPIEGELMANN};
use net::packet::op::SendOpcode;

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
