//! The Mu Lung Dojo, taken alone. So Gong in the hall sends a challenger
//! to the first floor, where its boss comes out; the door's portal stays
//! shut until the boss dies, then warps the challenger up and opens the
//! door, and the way behind it leads to the second floor. Each challenger
//! has a copy of their own, up to 15 on a channel. The harness world runs
//! events 60 times as fast, so the run lasts 5 seconds and then ends at
//! the exit.

use game_data::server_content::keys;
use integration_harness::events::{armed_in, maps, talk, told};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use integration_harness::WorldSession;
use net::events::{dojo_boss, DOJO, DOJO_BOSS_EFFECT, DOJO_EXIT, DOJO_FIRST, DOJO_UP_PORTAL};
use net::npc_scripts::SO_GONG;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const PINK: u8 = 5;
/// The floors' `dojang_up`, by the door, and `dojang_next`, behind it.
const UP: &str = "out00";
const NEXT: &str = "out001";

fn pink(key: &str) -> (u8, String) {
    (PINK, net::content::text().get(key, &[]))
}

/// A level-30 challenger takes So Gong on alone and lands on the first
/// floor.
async fn challenger() -> (WorldSession, Trace) {
    let mut player = armed_in(DOJO.lobby, 30).await;
    let seen = start(&mut player).await;
    (player, seen)
}

/// `player` takes So Gong on alone and lands on the first floor.
async fn start(player: &mut WorldSession) -> Trace {
    told(player, talk(DOJO.lobby, SO_GONG)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_FIRST)
            && t.field
                .mobs
                .iter()
                .any(|m| Some(m.1) == dojo_boss(DOJO_FIRST))
    })
    .await;
    seen
}

/// Hit `object_id` until `seen` sees the floor cleared.
async fn beat(player: &mut WorldSession, seen: &mut Trace, object_id: i32) {
    let clear = (3, "dojang/end/clear".to_string());
    for _ in 0..50 {
        if seen.field_effects.contains(&clear) {
            return;
        }
        let hits = seen.field.hp_bars.len() + seen.field.boss_bars.len();
        send(&mut player.connection, attack(object_id, 30_000)).await;
        until(&mut player.connection, seen, |t| {
            t.field.hp_bars.len() + t.field.boss_bars.len() > hits
                || t.field_effects.contains(&clear)
        })
        .await;
    }
    panic!("{object_id} lives: {seen:?}");
}

/// CHANGE_MAP_SPECIAL: walk to the first floor's portal `name` and press
/// Up.
async fn through(player: &mut WorldSession, name: &str) {
    let portal = maps()
        .field(DOJO_FIRST)
        .unwrap()
        .portal_by_name(name)
        .unwrap();
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
    send(&mut player.connection, p).await;
}

#[tokio::test]
async fn the_first_floors_boss_opens_the_door_up_to_the_second() {
    let (mut player, mut seen) = challenger().await;

    // The floor's start, and Mano with the Dojo's summon effect.
    assert!(seen
        .field_effects
        .contains(&(3, "dojang/start/stage".to_string())));
    assert!(seen
        .field_effects
        .contains(&(4, "Dojang/start".to_string())));
    let boss = *seen
        .field
        .mobs
        .iter()
        .find(|m| Some(m.1) == dojo_boss(DOJO_FIRST))
        .unwrap();
    assert_eq!(boss.1, 9300184);
    assert_eq!(
        seen.field.appearances.get(&boss.0),
        Some(&(DOJO_BOSS_EFFECT as i8))
    );

    // Both ways stay shut while the boss lives.
    through(&mut player, UP).await;
    until(&mut player.connection, &mut seen, |t| {
        t.broadcasts.contains(&pink(keys::DOJO_UP_MONSTERS))
    })
    .await;
    through(&mut player, NEXT).await;
    until(&mut player.connection, &mut seen, |t| {
        t.broadcasts.contains(&pink(keys::DOJO_NEXT_SHUT))
    })
    .await;
    assert!(seen.dojo_warps.is_empty());

    // The boss's death clears the floor.
    beat(&mut player, &mut seen, boss.0).await;
    assert!(seen.field.killed.contains(&boss.0));
    assert!(seen
        .field_effects
        .contains(&(4, "Dojang/clear".to_string())));

    // The door's portal warps the challenger up and opens the door.
    through(&mut player, UP).await;
    until(&mut player.connection, &mut seen, |t| {
        t.dojo_warps.contains(&DOJO_UP_PORTAL) && t.field.reactor_hits.iter().any(|h| h.1 == 1)
    })
    .await;

    // The way behind it leads to the second floor's start, with the
    // first floor's 2 training points.
    through(&mut player, NEXT).await;
    let points = net::content::text().get(keys::DOJO_POINTS, &[("points", "2"), ("total", "2")]);
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&(DOJO_FIRST + 100)) && t.broadcasts.contains(&(PINK, points.clone()))
    })
    .await;

    // Then time runs out and the challenger is sent to the exit.
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_EXIT)
    })
    .await;
}

#[tokio::test]
async fn two_challengers_on_a_channel_each_fight_their_own_boss() {
    // Both are in the hall before either starts, so neither run's clock
    // runs out while the other logs in.
    let mut first = armed_in(DOJO.lobby, 30).await;
    let mut second = armed_in(DOJO.lobby, 30).await;
    let mut first_seen = start(&mut first).await;
    let mut second_seen = start(&mut second).await;

    // Each copy brings out one boss, and the second sees only theirs.
    let boss = |seen: &Trace| {
        let bosses: Vec<i32> = seen
            .field
            .mobs
            .iter()
            .filter(|m| Some(m.1) == dojo_boss(DOJO_FIRST))
            .map(|m| m.0)
            .collect();
        assert_eq!(bosses.len(), 1, "{bosses:?}");
        bosses[0]
    };
    boss(&first_seen);

    // The second's kill clears only their floor, and the first never sees
    // them.
    let second_boss = boss(&second_seen);
    beat(&mut second, &mut second_seen, second_boss).await;
    through(&mut first, UP).await;
    until(&mut first.connection, &mut first_seen, |t| {
        t.broadcasts.contains(&pink(keys::DOJO_UP_MONSTERS))
    })
    .await;
    assert!(first_seen.players.is_empty());
    assert!(!first_seen.field.killed.contains(&second_boss));
}
