//! The Forest of Poison Haze's third and fourth stages. Each of the maze's
//! rooms leads on to the next, or one time in ten to its end, where Ellin
//! clears the maze for whichever member finds her and takes the team on.
//! On the fourth, Ellin hands out Purification Marbles; one catches a
//! Poison Flower only once it is weakened below 40% of its HP, and makes a
//! Monster Marble; the last flower caught clears the stage, and the
//! leader's twenty Monster Marbles take the team on. The harness world
//! runs events 60 times as fast, so the forest lasts 30 seconds.

use game_data::server_content::keys;
use integration_harness::events::{
    armed_in, cleared, enter_ellin, maps, party, talk, told, waiting_in,
};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{ELLIN_LOBBY, ELLIN_STAGES, MONSTER_MARBLE, PURIFICATION_MARBLE};
use net::npc_scripts::ELLIN_GUIDE;
use net::packet::op::RecvOpcode;
use net::scripts::ELLIN_MAZE_END;
use packet::io::write::PktWrite;
use packet::Packet;

const POISON_FLOWER: i32 = 9_300_175;
const USE: u8 = 2;
const ETC: u8 = 4;

/// Ellin's line `key`, with `n` filled in.
fn line(key: &str, n: &str) -> String {
    net::content::text().get(key, &[("n", n)])
}

/// CHANGE_MAP_SPECIAL: walk to `portal` and press Up.
async fn through(player: &mut WorldSession, portal: &game_data::PortalTemplate) {
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(&portal.name).unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

/// USE_CATCH_ITEM of the item in USE slot `slot` at the mob `object_id`.
fn throw(slot: i16, item_id: i32, object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCatchItem as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_int(object_id).unwrap();
    p
}

#[tokio::test]
async fn the_maze_leads_room_by_room_to_ellin_who_takes_the_team_on() {
    let mut team = Vec::new();
    for _ in 0..4 {
        team.push(waiting_in(ELLIN_LOBBY, 50, &[]).await);
    }
    party(&mut team).await;
    let mut traces = enter_ellin(&mut team).await;

    // A member, not the leader, goes into the maze, at its first room.
    let maze = ELLIN_STAGES[2];
    let field = maps().field(maze).unwrap();
    let c = &mut team[1].connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, maze, "", &mut seen).await, maze);
    let mut room = 1;
    for _ in 0..100 {
        let script = format!("party6_stage5{room:02}");
        let way = (field.portals())
            .find(|portal| portal.script.as_deref() == Some(script.as_str()))
            .unwrap();
        let (before, sounds) = (seen.arrivals.len(), seen.portal_sounds);
        through(&mut team[1], way).await;
        let c = &mut team[1].connection;
        until(c, &mut seen, |t| t.arrivals.len() > before).await;
        let (map, spot, _) = *seen.arrivals.last().unwrap();
        assert_eq!(map, maze);
        assert!(
            spot == room % 15 + 1 || spot == ELLIN_MAZE_END,
            "{room} to {spot}"
        );
        assert!(seen.portal_sounds > sounds, "{seen:?}");
        room = spot;
        if room == ELLIN_MAZE_END {
            break;
        }
    }
    assert_eq!(room, ELLIN_MAZE_END, "never at the maze's end");

    // Ellin there clears the maze and takes the whole team on.
    let t = told(&mut team[1], talk(maze, ELLIN_GUIDE)).await;
    let end = net::content::text().get(keys::ELLIN_GUIDE_MAZE_END, &[]);
    assert!(t.prompt_texts.contains(&end), "{t:?}");
    let c = &mut team[1].connection;
    let mut seen = t;
    until(c, &mut seen, cleared).await;
    send(c, answer(0, 1, None)).await;
    for (player, trace) in team.iter_mut().zip(&mut traces) {
        until(&mut player.connection, trace, |t| {
            t.warps.contains(&ELLIN_STAGES[3])
        })
        .await;
    }
}

#[tokio::test]
async fn weakened_poison_flowers_are_caught_and_the_leaders_monster_marbles_take_the_team_on() {
    let mut team = vec![armed_in(ELLIN_LOBBY, 50).await];
    for _ in 0..3 {
        team.push(waiting_in(ELLIN_LOBBY, 50, &[]).await);
    }
    party(&mut team).await;
    let mut traces = enter_ellin(&mut team).await;

    let stage = ELLIN_STAGES[3];
    let c = &mut team[0].connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, stage, "", &mut seen).await, stage);
    until(c, &mut seen, |t| t.field.alive(POISON_FLOWER).len() == 20).await;
    let flowers = seen.field.alive(POISON_FLOWER);

    // Ellin tells the leader what to do, then gives ten marbles.
    let t = told(&mut team[0], talk(stage, ELLIN_GUIDE)).await;
    let hint = line(keys::ELLIN_GUIDE_FLOWERS, "20");
    assert!(t.prompt_texts.contains(&hint), "{t:?}");
    let t = told(&mut team[0], answer(0, 1, None)).await;
    assert!(
        t.prompt_texts
            .contains(&line(keys::ELLIN_GUIDE_MARBLES, "10")),
        "{t:?}"
    );
    let marbles = |t: &Trace| {
        (t.added.iter())
            .find(|&&(kind, _, item)| kind == USE && item == PURIFICATION_MARBLE)
            .map(|&(_, slot, _)| slot)
    };
    let mut slot = marbles(&t).expect("no marbles given");
    seen.field.catches.clear();

    // At full HP a flower is too strong: nothing is caught, the marble
    // kept.
    let first = flowers[0];
    let (x, y) = seen.field.mob_positions[&first];
    send(&mut team[0].connection, walk_to(x, y)).await;
    let t = told(&mut team[0], throw(slot, PURIFICATION_MARBLE, first)).await;
    assert!(t.field.catches.is_empty() && t.removed.is_empty(), "{t:?}");

    // Weakened below 40%, each is caught and makes a Monster Marble; the
    // last clears the stage. Ellin gives ten more once the first are gone.
    for (caught, &flower) in flowers.iter().enumerate() {
        if caught == 10 {
            let t = told(&mut team[0], talk(stage, ELLIN_GUIDE)).await;
            slot = marbles(&t).expect("no more marbles given");
        }
        let (x, y) = seen.field.mob_positions[&flower];
        let c = &mut team[0].connection;
        send(c, walk_to(x, y)).await;
        send(c, attack(flower, 5_000)).await;
        send(c, throw(slot, PURIFICATION_MARBLE, flower)).await;
        until(c, &mut seen, |t| {
            t.field
                .catches
                .contains(&(flower, PURIFICATION_MARBLE, true))
        })
        .await;
        assert!(!cleared(&seen), "{caught}: {seen:?}");
    }
    let c = &mut team[0].connection;
    until(c, &mut seen, cleared).await;
    let (_, made, _) = *(seen.added.iter())
        .find(|&&(kind, _, item)| kind == ETC && item == MONSTER_MARBLE)
        .unwrap();
    until(c, &mut seen, |t| t.quantities.contains(&(ETC, made, 20))).await;

    // With all twenty, the leader hands them in and the team goes on.
    let t = told(&mut team[0], talk(stage, ELLIN_GUIDE)).await;
    let ready = net::content::text().get(keys::ELLIN_GUIDE_HAND_IN, &[]);
    assert!(t.prompt_texts.contains(&ready), "{t:?}");
    send(&mut team[0].connection, answer(0, 1, None)).await;
    for (player, trace) in team.iter_mut().zip(&mut traces) {
        until(&mut player.connection, trace, |t| {
            t.warps.contains(&ELLIN_STAGES[4])
        })
        .await;
    }
}
