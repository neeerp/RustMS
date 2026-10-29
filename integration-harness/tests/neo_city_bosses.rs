//! Neo City's era bosses: the door at the end of each era's way but the
//! first (`right00`, `TD_Boss_enter`) lets the leader of a party of two to
//! six, all of level 70 or more, take it into its own copy of the boss's
//! room, one party at a time an era on a channel. Year 2099's Bergamot
//! comes back twice as it dies; its last form's death wins the fight with
//! one clear effect. The party walks out by `left00` to the era's first
//! map, and one walking out of a fight not won leaves too few, which ends
//! it and sends the rest back to the door.
use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::events::{armed_in, cleared, party, party_of_one, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::packets::build_change_map;
use integration_harness::trace::{barrier, send, until, Trace};
use integration_harness::WorldSession;
use net::events::{neo_boss_door, neo_boss_room, NEO_BOSSES};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PINK: u8 = 5;
/// Year 2099's door, its boss's room, and the era's first map, where the
/// room's `left00` leads.
const DOOR: i32 = neo_boss_door(1);
const ROOM: i32 = neo_boss_room(1);
const ERA_START: i32 = 240_070_200;
const DOOR_AT: (i16, i16) = (1333, 161);
const LEFT_AT: (i16, i16) = (-121, 204);
/// Bergamot's three forms, the last of which wins the fight.
const BERGAMOT: [i32; 3] = [7_220_003, 7_220_004, NEO_BOSSES[0]];
/// Under the most an `armed_in` character may hit for.
const HIT: i32 = 30_000;

/// CHANGE_MAP_SPECIAL: press Up on the door.
fn enter() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("right00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// A level-75 character hitting hard, by the door: above Year 2099's
/// maps' `lvLimit`, 73, to walk out by `left00`.
async fn by_the_door() -> WorldSession {
    let mut player = armed_in(DOOR, 75).await;
    told(&mut player, walk_to(DOOR_AT.0, DOOR_AT.1)).await;
    player
}

/// `player` tries the door and is told the line `key`, staying put.
async fn turned_away(player: &mut WorldSession, key: &str, vars: &[(&str, &str)]) {
    let line = net::content::text().get(key, vars);
    let t = told(player, enter()).await;
    assert!(
        t.broadcasts.contains(&(PINK, line)) && t.arrivals.is_empty(),
        "{t:?}"
    );
}

/// Every player in `team` arrives in the room, on its clock.
async fn all_arrive(team: &mut [WorldSession]) -> Vec<Trace> {
    let mut traces = Vec::new();
    for player in team {
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == ROOM) && !t.countdowns.is_empty()
        })
        .await;
        traces.push(t);
    }
    traces
}

/// Wait until `player` arrives on `map_id`.
async fn arrives(player: &mut WorldSession, map_id: i32) {
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == map_id)
    })
    .await;
}

/// Hit the living `mob_id` until `seen` sees it die.
async fn beat(c: &mut MapleTestConnection, name: &str, seen: &mut Trace, mob_id: i32) {
    until(c, seen, |t| !t.field.alive(mob_id).is_empty()).await;
    let object_id = seen.field.alive(mob_id)[0];
    let (x, y) = seen.field.mob_positions[&object_id];
    send(c, walk_to(x, y)).await;
    for _ in 0..20 {
        if seen.field.killed.contains(&object_id) {
            return;
        }
        for _ in 0..5 {
            send(c, attack(object_id, HIT)).await;
        }
        barrier(c, name, seen).await;
    }
    panic!("{mob_id} lives: {seen:?}");
}

/// `player` walks out of the room by `left00`, to the era's first map.
async fn walk_out(player: &mut WorldSession) {
    let t = told(player, walk_to(LEFT_AT.0, LEFT_AT.1)).await;
    assert!(t.arrivals.is_empty(), "{t:?}");
    send(
        &mut player.connection,
        build_change_map(-1, "left00").unwrap(),
    )
    .await;
    arrives(player, ERA_START).await;
}

#[tokio::test]
async fn a_party_takes_the_door_to_its_eras_boss_whose_last_form_wins_it() {
    let mut team = vec![by_the_door().await, by_the_door().await];
    let mut next = vec![by_the_door().await, by_the_door().await];

    // A party first, of two or more, then its leader.
    turned_away(&mut team[1], keys::NEO_BOSS_NO_PARTY, &[]).await;
    party_of_one(&mut team[0]).await;
    let vars = [("least", "2"), ("most", "6"), ("level", "70")];
    turned_away(&mut team[0], keys::NEO_BOSS_NOT_ELIGIBLE, &vars).await;
    party(&mut team).await;
    party(&mut next).await;
    turned_away(&mut team[1], keys::NEO_BOSS_NOT_LEADER, &[]).await;

    // In, one party at a time.
    send(&mut team[0].connection, enter()).await;
    let mut traces = all_arrive(&mut team).await;
    turned_away(&mut next[0], keys::NEO_BOSS_BUSY, &[]).await;

    // Bergamot comes back twice; its last form's death wins it, once.
    let leader = &mut team[0];
    let name = leader.character_name.clone();
    let seen = &mut traces[0];
    for form in BERGAMOT {
        beat(&mut leader.connection, &name, seen, form).await;
    }
    until(&mut leader.connection, seen, cleared).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    barrier(&mut leader.connection, &name, seen).await;
    let clears = (seen.field_effects.iter())
        .filter(|(_, e)| e == "quest/party/clear")
        .count();
    assert_eq!(clears, 1, "{seen:?}");

    // The winners walk out, which frees the room for the next party.
    for player in &mut team {
        walk_out(player).await;
    }
    told(&mut next[0], walk_to(DOOR_AT.0, DOOR_AT.1)).await;
    send(&mut next[0].connection, enter()).await;
    all_arrive(&mut next).await;

    // One of them walking out of the fight leaves too few, which ends it.
    let (first, rest) = next.split_first_mut().unwrap();
    walk_out(first).await;
    arrives(&mut rest[0], DOOR).await;
}
