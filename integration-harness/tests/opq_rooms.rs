//! Orbis's party quest, the rooms where Eak takes what the team gathered:
//! on the walkway 30 of the pixies' pieces and in the lounge 40 more, each
//! for a piece of Minerva's statue, which the leader then sets in its scar
//! on the hub. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts a minute.

use game_data::server_content::keys;
use integration_harness::events::{enter_opq, party, talk, throw_down, waiting_in, EAK};
use integration_harness::hunting::walk_to;
use integration_harness::quests::answer;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{OPQ, OPQ_HUB};
use packet::Packet;

const WALKWAY: i32 = 920010200;
const LOUNGE: i32 = 920010600;
const ETC: u8 = 4;

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

fn said(trace: &Trace, key: &str, vars: &[(&str, &str)]) -> bool {
    let text = net::content::text().get(key, vars);
    trace.prompt_texts.iter().any(|said| said.contains(&text))
}

#[tokio::test]
async fn eak_trades_the_rooms_gatherings_for_pieces_of_the_statue() {
    let mut a = vec![waiting_in(OPQ.lobby, 51, &[(4001050, 30), (4001052, 40)]).await];
    for _ in 0..4 {
        a.push(waiting_in(OPQ.lobby, 51, &[]).await);
    }
    party(&mut a).await;
    enter_opq(&mut a).await;
    for player in a[..2].iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut player.connection, WALKWAY, "", &mut trace).await,
            WALKWAY
        );
    }

    // Only the leader may hand them in.
    let trace = told(&mut a[1], talk(WALKWAY, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_LEADER_ONLY, &[]), "{trace:?}");

    let mut slots = Vec::new();
    for (room, item, n, piece) in [
        (WALKWAY, 4001050, 30, 4001044),
        (LOUNGE, 4001052, 40, 4001048),
    ] {
        if room != WALKWAY {
            let mut trace = Trace::default();
            assert_eq!(warp(&mut a[0].connection, room, "", &mut trace).await, room);
        }
        // Eak takes them at once, gives the piece and clears the room.
        let mut trace = told(&mut a[0], talk(room, EAK)).await;
        let piece_s = piece.to_string();
        assert!(
            said(&trace, keys::OPQ_EAK_PIECE, &[("piece", &piece_s)]),
            "{trace:?}"
        );
        until(&mut a[0].connection, &mut trace, |t| {
            t.items.contains(&(piece, 1))
                && t.field_effects
                    .iter()
                    .any(|(_, e)| e == "quest/party/clear")
        })
        .await;
        assert!(trace.items.contains(&(item, -n)), "{trace:?}");
        let &(_, slot, _) = trace.added.iter().find(|a| a.2 == piece).unwrap();
        slots.push(slot);
        told(&mut a[0], answer(0, 1, None)).await;

        // After, he only sends the team on.
        let trace = told(&mut a[0], talk(room, EAK)).await;
        assert!(said(&trace, keys::OPQ_EAK_ROOM_CLEARED, &[]), "{trace:?}");
        assert!(trace.items.is_empty(), "{trace:?}");
    }

    // The leader sets both pieces in their scars on the hub, which keep them.
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut a[0].connection, OPQ_HUB, "", &mut seen).await,
        OPQ_HUB
    );
    until(&mut a[0].connection, &mut seen, |t| {
        t.field
            .reactors
            .iter()
            .filter(|r| (2008000..=2008005).contains(&r.reactor_id))
            .count()
            == 6
    })
    .await;
    for (scar, slot) in [(2008003, slots[0]), (2008004, slots[1])] {
        let at = *seen
            .field
            .reactors
            .iter()
            .find(|r| r.reactor_id == scar)
            .unwrap();
        assert_eq!(at.state, 0);
        send(&mut a[0].connection, walk_to(at.x, at.y - 10)).await;
        send(&mut a[0].connection, throw_down(slot, 1)).await;
        until(&mut a[0].connection, &mut seen, |t| {
            t.field.reactor_hits.contains(&(at.object_id, 1))
        })
        .await;
        assert!(
            !seen
                .field
                .reactors_removed
                .iter()
                .any(|&(oid, _)| oid == at.object_id),
            "{seen:?}"
        );
        assert!(seen.removed.contains(&(ETC, slot)), "{seen:?}");
    }
}
