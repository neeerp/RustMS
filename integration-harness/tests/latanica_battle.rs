//! Capt. Latanica on the ghost ship: on its last deck (541010060) a
//! party's leader holding a White Essence takes the party through the
//! `in00` portal to its own copy of the Engine Room, one party a channel at
//! a time. The essence dropped on the room's reactor brings out the
//! captain, whose death wins the fight and stops its clock; Bob takes the
//! winners up to the Peaceful Ship. The fight lasts until its last member
//! leaves, or its ten minutes are up, when those left go to the Peaceful
//! Ship too. The harness world runs events 60 times as fast.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_str, set_weapon_attack,
};
use integration_harness::events::{cleared, said};
use integration_harness::events::{party, party_of_one, talk, throw_down, told, waiting_in};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{ENGINE_ROOM, GHOST_SHIP_DECK, LATANICA_AT, PEACEFUL_SHIP, WHITE_ESSENCE};
use net::mobs::LATANICA;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const BOB: i32 = 9270033;
const PINK: u8 = 5;
/// Under the most a level-50 character with 32000 STR and a 1999-attack
/// sword may hit for.
const HIT: i32 = 5_000_000;

/// A new level-50 character hitting very hard and holding a White
/// Essence, logged in on the ghost ship's last deck.
async fn hunter() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.unwrap();
    grant_stack(id, 4, 1, WHITE_ESSENCE, 1).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, GHOST_SHIP_DECK, "", &mut seen).await,
        GHOST_SHIP_DECK
    );
    session
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `in00`.
fn enter() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("in00").unwrap();
    p.write_short(0).unwrap();
    p
}

/// `player` tries the portal and is told the line `key`, staying put.
async fn turned_away(player: &mut WorldSession, key: &str, vars: &[(&str, &str)]) {
    let line = net::content::text().get(key, vars);
    let t = told(player, enter()).await;
    assert!(
        t.broadcasts.contains(&(PINK, line)) && t.arrivals.is_empty(),
        "{t:?}"
    );
}

/// Every player in `team` arrives in the Engine Room, on its clock.
async fn all_arrive(team: &mut [WorldSession]) -> Vec<Trace> {
    let mut traces = Vec::new();
    for player in team {
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == ENGINE_ROOM) && !t.countdowns.is_empty()
        })
        .await;
        assert!((5..=10).contains(&t.countdowns[0]), "{t:?}");
        traces.push(t);
    }
    traces
}

/// Hit `object_id` until `seen` sees it die, five hits at a time.
async fn beat(c: &mut MapleTestConnection, name: &str, seen: &mut Trace, object_id: i32) {
    for _ in 0..20 {
        if seen.field.killed.contains(&object_id) {
            return;
        }
        for _ in 0..5 {
            send(c, attack(object_id, HIT)).await;
        }
        barrier(c, name, seen).await;
    }
    panic!("{object_id} lives: {seen:?}");
}

/// Wait until `player` arrives on `map_id`.
async fn arrives(player: &mut WorldSession, map_id: i32) {
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == map_id)
    })
    .await;
}

#[tokio::test]
async fn the_white_essence_brings_out_latanica_whose_death_wins_the_fight() {
    let essence = [(WHITE_ESSENCE, 1)];
    let mut team = vec![
        hunter().await,
        waiting_in(GHOST_SHIP_DECK, 30, &essence).await,
    ];
    let mut next = waiting_in(GHOST_SHIP_DECK, 30, &essence).await;
    let mut bare = waiting_in(GHOST_SHIP_DECK, 30, &[]).await;

    // The essence comes first, then a party, then its leader.
    party_of_one(&mut bare).await;
    turned_away(&mut bare, keys::LATANICA_PORTAL_NO_ESSENCE, &[]).await;
    turned_away(&mut team[1], keys::LATANICA_PORTAL_NO_PARTY, &[]).await;
    party(&mut team).await;
    party_of_one(&mut next).await;
    turned_away(&mut team[1], keys::LATANICA_PORTAL_NOT_LEADER, &[]).await;

    // The leader takes the party in, and at once drops the essence on the
    // reactor, which takes it a few seconds later and calls out the captain.
    send(&mut team[0].connection, enter()).await;
    let mut traces = all_arrive(&mut team).await;
    let (mut ht, mut mt) = (traces.remove(0), traces.remove(0));
    let (hunter, member) = team.split_at_mut(1);
    let (hunter, member) = (&mut hunter[0], &mut member[0]);
    let name = hunter.character_name.clone();
    send(
        &mut hunter.connection,
        walk_to(LATANICA_AT.0, LATANICA_AT.1 - 5),
    )
    .await;
    send(&mut hunter.connection, throw_down(1, 1)).await;

    // Meanwhile the next party is turned away, and Bob asks first.
    turned_away(&mut next, keys::LATANICA_PORTAL_BUSY, &[]).await;
    let t = told(member, talk(ENGINE_ROOM, BOB)).await;
    assert!(said(&t, keys::LATANICA_BOB_LEAVE_ASK), "{t:?}");
    told(member, answer(1, 0, None)).await;

    // The member's talks read its packets, so the hunter watches for the
    // captain.
    let h = &mut hunter.connection;
    let summoned = net::content::text().get(keys::LATANICA_SUMMONED, &[]);
    until(h, &mut ht, |t| {
        t.broadcasts.contains(&(PINK, summoned.clone())) && !t.field.alive(LATANICA).is_empty()
    })
    .await;
    assert!(
        ht.field_effects
            .contains(&(6, "Bgm09/TimeAttack".to_string())),
        "{ht:?}"
    );

    // Its death wins the fight, and the clock stops.
    let m = &mut member.connection;
    let captain = ht.field.alive(LATANICA)[0];
    beat(h, &name, &mut ht, captain).await;
    until(h, &mut ht, cleared).await;
    until(m, &mut mt, cleared).await;
    tokio::time::sleep(Duration::from_secs(11)).await;
    barrier(m, &member.character_name, &mut mt).await;
    assert!(!mt.arrivals.iter().any(|a| a.0 == PEACEFUL_SHIP), "{mt:?}");

    // Bob takes the winners up to the Peaceful Ship; the last one out ends
    // the fight, and the next party may go in.
    for player in [member, hunter] {
        let t = told(player, talk(ENGINE_ROOM, BOB)).await;
        assert!(said(&t, keys::LATANICA_BOB_WON), "{t:?}");
        let mut t = told(player, answer(0, 1, None)).await;
        until(&mut player.connection, &mut t, |t| {
            t.arrivals.iter().any(|a| a.0 == PEACEFUL_SHIP)
        })
        .await;
    }
    send(&mut next.connection, enter()).await;
    all_arrive(std::slice::from_mut(&mut next)).await;

    // Its time up, the next party goes to the Peaceful Ship too.
    arrives(&mut next, PEACEFUL_SHIP).await;
}
