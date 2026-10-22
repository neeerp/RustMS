//! Orbis's party quest, its cloud stage and hub: the team drops 20 cloud
//! pieces on the light, which brings out Eak; he sends the others up to the
//! stage's top, and tells the leader of Minerva and clears the stage, which
//! takes everyone there. On the hub only the leader hears him, and its
//! portals lead into the rooms and back, all but the shut storage. The
//! harness world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the
//! quest lasts a minute.

use game_data::server_content::keys;
use integration_harness::events::{
    enter_opq, maps, party, talk, talk_to, throw_down, waiting_in, EAK,
};
use integration_harness::hunting::walk_to;
use integration_harness::quests::answer;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{OPQ, OPQ_HUB};
use net::packet::op::RecvOpcode;
use net::reactors::OPQ_LIGHT;
use packet::io::write::PktWrite;
use packet::Packet;

const CLOUD_PIECE: i32 = 4001063;

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// The name of `map`'s portal running `script`.
fn scripted(map: i32, script: &str) -> String {
    let field = maps().field(map).unwrap();
    let portal = field
        .portals()
        .find(|p| p.script.as_deref() == Some(script));
    portal.unwrap().name.clone()
}

/// Where taking `map`'s portal running `script` leads: the map and portal.
async fn through(player: &mut WorldSession, map: i32, script: &str) -> (i32, u8) {
    let trace = told(player, enter(&scripted(map, script))).await;
    let [(map, portal, _)] = trace.arrivals[..] else {
        panic!("{trace:?}");
    };
    (map, portal)
}

#[tokio::test]
async fn the_light_brings_out_eak_who_clears_the_cloud_stage() {
    let mut a = vec![waiting_in(OPQ.lobby, 51, &[(CLOUD_PIECE, 20)]).await];
    for _ in 0..4 {
        a.push(waiting_in(OPQ.lobby, 51, &[]).await);
    }
    party(&mut a).await;
    let mut traces = enter_opq(&mut a).await;

    // The light waits for the cloud pieces.
    let (bystander, seen) = (&mut a[1], &mut traces[1]);
    until(&mut bystander.connection, seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == OPQ_LIGHT)
    })
    .await;
    let light = *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == OPQ_LIGHT)
        .unwrap();
    assert_eq!(light.state, 20);
    send(&mut a[0].connection, walk_to(light.x, light.y - 10)).await;
    send(&mut a[0].connection, throw_down(1, 20)).await;

    // It goes, and Eak steps out, as everyone is told.
    let notice = net::content::text().get(keys::OPQ_NOTICE_LIGHT, &[]);
    let (bystander, seen) = (&mut a[1], &mut traces[1]);
    until(&mut bystander.connection, seen, |t| {
        t.field.npc(EAK).is_some()
            && t.field
                .reactors_removed
                .iter()
                .any(|&(oid, _)| oid == light.object_id)
    })
    .await;
    assert!(
        seen.broadcasts.iter().any(|(_, text)| *text == notice),
        "{seen:?}"
    );
    let eak = seen.field.npc(EAK).unwrap();

    // Anyone but the leader is sent up to the top.
    let top = maps()
        .field(OPQ.entry)
        .unwrap()
        .portal_by_name("st00")
        .unwrap()
        .id as u8;
    let trace = told(&mut a[1], talk_to(eak)).await;
    assert!(trace.prompts.is_empty(), "{trace:?}");
    assert_eq!(
        trace.arrivals.first().map(|a| (a.0, a.1)),
        Some((OPQ.entry, top))
    );

    // The leader hears of Minerva, then the stage clears and everyone on it
    // goes up, sees it and gets its EXP.
    let trace = told(&mut a[0], talk_to(eak)).await;
    assert_eq!(trace.prompts, [(EAK, 0)]);
    let cloud = net::content::text().get(keys::OPQ_EAK_CLOUD, &[]);
    assert!(trace.prompt_texts[0].contains(&cloud), "{trace:?}");
    send(&mut a[0].connection, answer(0, 1, None)).await;
    for player in a.iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.field_effects
                .iter()
                .any(|(_, e)| e == "quest/party/clear")
        })
        .await;
        assert_eq!(
            trace.arrivals.first().map(|a| (a.0, a.1)),
            Some((OPQ.entry, top))
        );
    }

    // After, he just sends the leader up too.
    let trace = told(&mut a[0], talk_to(eak)).await;
    assert!(trace.prompts.is_empty(), "{trace:?}");
    assert_eq!(trace.arrivals.len(), 1, "{trace:?}");

    // On the hub only the leader hears him.
    for player in a[..2].iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut player.connection, OPQ_HUB, "", &mut trace).await,
            OPQ_HUB
        );
    }
    let said = |trace: &Trace, key| {
        let text = net::content::text().get(key, &[]);
        trace.prompt_texts.iter().any(|said| said.contains(&text))
    };
    let trace = told(&mut a[0], talk(OPQ_HUB, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_GATHER), "{trace:?}");
    let trace = told(&mut a[1], talk(OPQ_HUB, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_LEADER_ONLY), "{trace:?}");

    // The hub's portals lead into its rooms and back.
    let leader = &mut a[0];
    for (room, script, portal, back) in [
        (920010200, "party3_room1", 13, 4),
        (920010600, "party3_room5", 17, 15),
    ] {
        assert_eq!(through(leader, OPQ_HUB, script).await, (room, portal));
        assert_eq!(
            through(leader, room, "party3_roomout").await,
            (OPQ_HUB, back)
        );
    }
    let storage = scripted(OPQ_HUB, "party3_room8");
    let trace = told(leader, enter(&storage)).await;
    let shut = net::content::text().get(keys::OPQ_NOTICE_STORAGE_CLOSED, &[]);
    assert!(trace.arrivals.is_empty(), "{trace:?}");
    assert!(
        trace.broadcasts.iter().any(|(_, text)| *text == shut),
        "{trace:?}"
    );
}
