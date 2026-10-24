//! The Forest of Poison Haze's fifth and sixth stages. One of the fifth's
//! nine boxes, in a new place each time, holds the Purple Stone of Magic,
//! and breaking it clears the stage; the leader shows the stone to the
//! Sprite, who takes the team on. On the sixth, the altar takes the stone
//! and brings out the Poison Golem, which dies into its Charged and then
//! its Super-Charged form; that one's death wins the forest, and the way
//! out then lets the leader take the team to the prize map. The harness
//! world runs events 60 times as fast, so the forest lasts 30 seconds.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::events::{
    armed_with, cleared, enter_ellin, go_to, maps, party, talk, throw_down, told, waiting_in,
};
use integration_harness::hunting::{attack, hit_reactor, pickup, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    ELLIN_ALTAR_AT, ELLIN_EXIT, ELLIN_GOLEM, ELLIN_GOLEM_LAST, ELLIN_LOBBY, ELLIN_STAGES,
    PURPLE_STONE,
};
use net::npc_scripts::ELLIN_SPRITE;
use net::packet::op::RecvOpcode;
use net::reactors::{ELLIN_ALTAR, ELLIN_STONE_BOX};
use packet::io::write::PktWrite;
use packet::Packet;

const EMPTY_BOX: i32 = 3_002_002;
const CHARGED_GOLEM: i32 = 9_300_181;
const PINK: u8 = 5;

fn pink(key: &str) -> (u8, String) {
    (PINK, net::content::text().get(key, &[]))
}

/// CHANGE_MAP_SPECIAL: walk to `map`'s portal `name` and press Up.
async fn through(player: &mut WorldSession, map: i32, name: &str) {
    let field = maps().field(map).unwrap();
    let portal = field.portal_by_name(name).unwrap();
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

/// Hit `object_id` until `seen` sees it die.
async fn beat(c: &mut MapleTestConnection, seen: &mut Trace, object_id: i32) {
    for _ in 0..100 {
        if seen.field.killed.contains(&object_id) {
            return;
        }
        let bars = seen.field.boss_bars.len();
        send(c, attack(object_id, 30_000)).await;
        until(c, seen, |t| {
            t.field.boss_bars.len() > bars || t.field.killed.contains(&object_id)
        })
        .await;
    }
    panic!("{object_id} lives: {seen:?}");
}

#[tokio::test]
async fn the_box_with_the_stone_clears_the_stage_and_the_sprite_takes_the_leader_on() {
    let mut team = Vec::new();
    for _ in 0..4 {
        team.push(waiting_in(ELLIN_LOBBY, 50, &[]).await);
    }
    party(&mut team).await;
    let mut traces = enter_ellin(&mut team).await;

    // The nine boxes stand where the map has them, in some order.
    let stage = ELLIN_STAGES[4];
    let (mut seen, stone_box) = go_to(&mut team[0], stage, ELLIN_STONE_BOX).await;
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| t.field.reactors.len() == 9).await;
    let mut spots: Vec<_> = (seen.field.reactors.iter()).map(|r| (r.x, r.y)).collect();
    let field = maps().field(stage).unwrap();
    let mut placed: Vec<_> = (field.reactors.iter()).map(|r| (r.x, r.y)).collect();
    spots.sort();
    placed.sort();
    assert_eq!(spots, placed);
    let empty = (seen.field.reactors.iter())
        .filter(|r| r.reactor_id == EMPTY_BOX)
        .count();
    assert_eq!(empty, 8, "{seen:?}");

    // A member without the stone may only leave.
    let m = &mut team[1].connection;
    assert_eq!(warp(m, stage, "", &mut Trace::default()).await, stage);
    let t = told(&mut team[1], talk(stage, ELLIN_SPRITE)).await;
    let leave = net::content::text().get(keys::ELLIN_SPRITE_LEAVE, &[]);
    assert!(t.prompt_texts.contains(&leave), "{t:?}");

    // Four hits break the box, which drops the stone and clears the stage.
    let c = &mut team[0].connection;
    send(c, walk_to(stone_box.x, stone_box.y)).await;
    for state in 1..=3u8 {
        send(c, hit_reactor(stone_box.object_id)).await;
        until(c, &mut seen, |t| {
            t.field.reactor_hits.contains(&(stone_box.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(stone_box.object_id)).await;
    until(c, &mut seen, |t| {
        t.field.item_drop(PURPLE_STONE).is_some() && cleared(t)
    })
    .await;
    let stone = seen.field.item_drop(PURPLE_STONE).unwrap();
    send(c, pickup(stone.object_id, stone.x, stone.y)).await;
    until(c, &mut seen, |t| {
        t.added.iter().any(|&(_, _, item)| item == PURPLE_STONE)
    })
    .await;

    // The leader shows it to the Sprite, who takes the whole team on.
    let t = told(&mut team[0], talk(stage, ELLIN_SPRITE)).await;
    let on = net::content::text().get(keys::ELLIN_SPRITE_TO_ALTAR, &[]);
    assert!(t.prompt_texts.contains(&on), "{t:?}");
    send(&mut team[0].connection, answer(0, 1, None)).await;
    for (player, trace) in team.iter_mut().zip(&mut traces) {
        until(&mut player.connection, trace, |t| {
            t.warps.contains(&ELLIN_STAGES[5])
        })
        .await;
    }
}

#[tokio::test]
async fn the_altar_brings_out_the_golem_whose_last_form_wins_the_forest() {
    let mut team = vec![armed_with(ELLIN_LOBBY, 50, &[(PURPLE_STONE, 1)]).await];
    for _ in 0..3 {
        team.push(waiting_in(ELLIN_LOBBY, 50, &[]).await);
    }
    party(&mut team).await;
    let mut traces = enter_ellin(&mut team).await;

    // The way out stays shut while the golem has not fallen.
    let stage = ELLIN_STAGES[5];
    let (mut seen, _) = go_to(&mut team[0], stage, ELLIN_ALTAR).await;
    through(&mut team[0], stage, "out00").await;
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        t.broadcasts.contains(&pink(keys::ELLIN_STAGE6_SHUT))
    })
    .await;

    // The stone on the altar brings out the golem.
    let (x, y) = ELLIN_ALTAR_AT;
    send(c, walk_to(x, y)).await;
    send(c, throw_down(1, 1)).await;
    until(c, &mut seen, |t| {
        t.broadcasts.contains(&pink(keys::ELLIN_ALTAR_GOLEM))
            && !t.field.alive(ELLIN_GOLEM).is_empty()
    })
    .await;

    // It dies into its Charged form, and that into its Super-Charged one,
    // whose death wins the forest.
    for mob_id in [ELLIN_GOLEM, CHARGED_GOLEM, ELLIN_GOLEM_LAST] {
        until(c, &mut seen, |t| !t.field.alive(mob_id).is_empty()).await;
        let golem = seen.field.alive(mob_id)[0];
        assert!(!cleared(&seen), "{mob_id}: {seen:?}");
        beat(c, &mut seen, golem).await;
    }
    until(c, &mut seen, cleared).await;

    // A member is told to wait for the leader, who takes the team out.
    let mut member = Trace::default();
    let m = &mut team[1].connection;
    assert_eq!(warp(m, stage, "", &mut member).await, stage);
    member.warps.clear();
    through(&mut team[1], stage, "out00").await;
    let m = &mut team[1].connection;
    until(m, &mut member, |t| {
        t.broadcasts.contains(&pink(keys::ELLIN_STAGE6_WAIT))
    })
    .await;
    assert!(member.warps.is_empty(), "{member:?}");
    through(&mut team[0], stage, "out00").await;
    for (player, trace) in team.iter_mut().zip(&mut traces) {
        until(&mut player.connection, trace, |t| {
            t.warps.contains(&ELLIN_EXIT)
        })
        .await;
    }
}
