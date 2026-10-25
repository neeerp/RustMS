//! The Monster Carnival. Spiegelmann on the Carnival Field sends a party's
//! leader and their party into one of six rooms' battle fields. The harness
//! world lets a party of one play (`RUSTMS_CARNIVAL_SOLO`) and runs events 60
//! times as fast, so a battle's 610 seconds take about 10. Each member's
//! screen is told their side and the score on arrival, and a kill earns its
//! mob's CP for the killer and their side. When time runs out everyone goes
//! to the exit room.

use integration_harness::events::{armed_in, party_of_one, talk, told};
use integration_harness::hunting::attack;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use net::carnival::{battle_map, CARNIVAL_EXIT, CARNIVAL_LOBBY};
use net::npc_scripts::SPIEGELMANN;
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

#[tokio::test]
async fn a_party_battles_for_cp_until_its_time_runs_out() {
    let player = &mut armed_in(CARNIVAL_LOBBY, 40).await;
    party_of_one(player).await;

    // Join, then room 1.
    told(player, talk(CARNIVAL_LOBBY, SPIEGELMANN)).await;
    told(player, answer(4, 1, Some(0))).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;

    // On the battle field with its clock, the screen shows Maple Red with
    // no CP yet, and none of the field's mobs summoned.
    let field = battle_map(1);
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&field)
            && !t.countdowns.is_empty()
            && !carnival(t, SendOpcode::CarnivalStart).is_empty()
            && blue_mob(t).is_some()
    })
    .await;
    assert!(seen.countdowns[0] <= 11, "{:?}", seen.countdowns);
    let start = carnival(&seen, SendOpcode::CarnivalStart);
    let summonable = integration_harness::events::maps()
        .field(field)
        .and_then(|f| f.carnival.as_ref())
        .map_or(0, |rules| rules.mobs.len());
    assert!(summonable > 0);
    let expected = [vec![0], shorts(&[0; 6]), vec![0; summonable]].concat();
    assert_eq!(start, [expected]);

    // The field's own mobs play for both sides.
    let blue = blue_mob(&seen).unwrap();
    assert!(seen.field.teams.values().any(|&team| team == 0));

    // Killing a Maple Blue mob earns its CP for the player and Maple Red.
    send(&mut player.connection, attack(blue, 30_000)).await;
    until(&mut player.connection, &mut seen, |t| {
        t.field.killed.contains(&blue) && !carnival(t, SendOpcode::CarnivalPartyCp).is_empty()
    })
    .await;
    assert_eq!(
        carnival(&seen, SendOpcode::CarnivalObtainedCp),
        [shorts(&[1, 1])]
    );
    assert_eq!(
        carnival(&seen, SendOpcode::CarnivalPartyCp),
        [[vec![0], shorts(&[1, 1])].concat()]
    );

    // Time runs out, and the player goes to the exit room.
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&CARNIVAL_EXIT)
    })
    .await;
}
