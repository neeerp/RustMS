//! The Dead Mine, Zakum's party quest: Adobis takes an approved party in;
//! seven keys thrown on the giant chest open it for the Fire Ore; in the
//! maze a trap box sends its breaker back to the hub and a Mimic box lets
//! out three Mimics; the leader hands Aura the Fire Ore with 30
//! documents, which wins it for everyone with the documents' EXP; each
//! then claims a prize from her, and the hub's way out stays shut until
//! they have, taking the quest's items as they leave.

use game_data::server_content::keys;
use integration_harness::database::{grant_stack, set_job, start_quest};
use integration_harness::events::{enter_zpq, go_to, party, said, talk, throw_down, told, AURA};
use integration_harness::hunting::{hit_reactor, pickup, walk_to, MapReactor};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    DEAD_MINE_SCROLL, FIRE_ORE, FIRE_ORE_PIECE, ZPQ, ZPQ_DOCUMENT, ZPQ_DOCUMENTS_EXP, ZPQ_HUB,
    ZPQ_KEY,
};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ZAKUM_APPROVED: i16 = 32400;
const JUNCTION: i32 = 280011000;
const CHEST_ROOM: i32 = 280011005;
const GIANT_CHEST: i32 = 2112014;
const TRAP: i32 = 2110000;
const MIMIC_BOX: i32 = 2111000;
const MIMIC: i32 = 9300004;
const PINK: u8 = 5;

/// A new level-50 character a chief has approved for Zakum, holding
/// `items` in their Etc tab, waiting by Adobis.
async fn approved(items: &[(i32, i16)]) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.unwrap();
    start_quest(id, ZAKUM_APPROVED).await.unwrap();
    for (slot, &(item_id, quantity)) in (1..).zip(items) {
        grant_stack(id, 4, slot, item_id, quantity).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut session.connection, ZPQ.lobby, "", &mut trace).await,
        ZPQ.lobby
    );
    session
}

/// CHANGE_MAP_SPECIAL: press Up on the hub's way out.
fn way_out() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("ps01").unwrap();
    p.write_short(0).unwrap();
    p
}

/// Whether `trace` holds just the pink line `key`.
fn pink(trace: &Trace, key: &str) -> bool {
    let line = net::content::text().get(key, &[]);
    matches!(trace.broadcasts.as_slice(), [(PINK, text)] if *text == line)
}

/// Pick Aura's menu `choice`.
async fn ask_aura(player: &mut WorldSession, choice: i32) -> Trace {
    let menu = told(player, talk(ZPQ_HUB, AURA)).await;
    assert_eq!(menu.prompts, [(AURA, 4)], "{menu:?}");
    told(player, answer(4, 1, Some(choice))).await
}

/// The first `reactor_id` of the junction's copy, seen from there.
fn reactor(seen: &Trace, reactor_id: i32) -> MapReactor {
    *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == reactor_id)
        .unwrap()
}

/// Hit `box_` until it breaks.
async fn smash(player: &mut WorldSession, seen: &mut Trace, box_: MapReactor) {
    let c = &mut player.connection;
    let gone = |t: &Trace| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == box_.object_id)
    };
    send(c, walk_to(box_.x, box_.y)).await;
    for _ in 0..8 {
        if gone(seen) {
            return;
        }
        let hits = seen.field.reactor_hits.len();
        send(c, hit_reactor(box_.object_id)).await;
        until(c, seen, |t| t.field.reactor_hits.len() > hits || gone(t)).await;
    }
    assert!(gone(seen), "{seen:?}");
}

#[tokio::test]
async fn a_party_wins_the_dead_mine_with_the_documents_and_claims_its_prizes() {
    let mut a = vec![
        approved(&[(ZPQ_DOCUMENT, 30), (ZPQ_KEY, 8)]).await,
        approved(&[]).await,
    ];
    party(&mut a).await;
    enter_zpq(&mut a).await;

    // Only the leader hands Aura anything, and the way out stays shut.
    let t = ask_aura(&mut a[1], 1).await;
    assert!(said(&t, keys::ZAKUM_AURA_LEADER_ONLY), "{t:?}");
    let t = told(&mut a[1], way_out()).await;
    assert!(
        pink(&t, keys::ZAKUM_MINE_NOT_WON) && t.warps.is_empty(),
        "{t:?}"
    );

    // Seven keys thrown on the giant chest open it for the Fire Ore.
    let (mut seen, chest) = go_to(&mut a[0], CHEST_ROOM, GIANT_CHEST).await;
    let c = &mut a[0].connection;
    send(c, walk_to(chest.x, chest.y - 10)).await;
    send(c, throw_down(2, 7)).await;
    until(c, &mut seen, |t| t.field.item_drop(FIRE_ORE).is_some()).await;
    let ore = seen.field.item_drop(FIRE_ORE).unwrap();
    send(c, walk_to(ore.x, ore.y)).await;
    send(c, pickup(ore.object_id, ore.x, ore.y)).await;
    until(c, &mut seen, |t| t.added.iter().any(|a| a.2 == FIRE_ORE)).await;

    // At the junction a Mimic box lets out three Mimics, and a trap box
    // sends its breaker back to the hub.
    let mut seen = Trace::default();
    let c = &mut a[0].connection;
    assert_eq!(warp(c, JUNCTION, "", &mut seen).await, JUNCTION);
    until(c, &mut seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == TRAP)
            && t.field.reactors.iter().any(|r| r.reactor_id == MIMIC_BOX)
    })
    .await;
    let (mimic_box, trap) = (reactor(&seen, MIMIC_BOX), reactor(&seen, TRAP));
    let mut t = Trace::default();
    smash(&mut a[0], &mut t, mimic_box).await;
    let mimics = |t: &Trace| t.field.mobs.iter().filter(|m| m.1 == MIMIC).count();
    until(&mut a[0].connection, &mut t, |t| mimics(t) == 3).await;
    assert!(pink(&t, keys::ZAKUM_MINE_MIMICS), "{t:?}");
    let mut t = Trace::default();
    smash(&mut a[0], &mut t, trap).await;
    until(&mut a[0].connection, &mut t, |t| t.warps.contains(&ZPQ_HUB)).await;
    assert!(pink(&t, keys::ZAKUM_MINE_UNKNOWN_FORCE), "{t:?}");

    // The leader hands in the Fire Ore and the documents: everyone gets the
    // documents' EXP.
    let t = ask_aura(&mut a[0], 1).await;
    assert_eq!(t.prompts, [(AURA, 1)]);
    assert!(said(&t, keys::ZAKUM_AURA_ASK_DOCUMENTS), "{t:?}");
    let mut t = told(&mut a[0], answer(1, 1, None)).await;
    assert_eq!(t.items, [(FIRE_ORE, -1), (ZPQ_DOCUMENT, -30)]);
    for (player, mut t) in a.iter_mut().zip([std::mem::take(&mut t), Trace::default()]) {
        until(&mut player.connection, &mut t, |t| !t.gains.is_empty()).await;
        assert_eq!(t.gains, [ZPQ_DOCUMENTS_EXP]);
    }

    // Unclaimed, the way out stays shut; Aura gives the prize once.
    let t = told(&mut a[1], way_out()).await;
    assert!(
        pink(&t, keys::ZAKUM_MINE_NO_PRIZE) && t.warps.is_empty(),
        "{t:?}"
    );
    for player in a.iter_mut() {
        let t = told(player, talk(ZPQ_HUB, AURA)).await;
        assert!(said(&t, keys::ZAKUM_AURA_WON), "{t:?}");
        let t = told(player, answer(0, 1, None)).await;
        assert_eq!(
            t.items,
            [
                (DEAD_MINE_SCROLL.0, DEAD_MINE_SCROLL.1.into()),
                (FIRE_ORE_PIECE, 1)
            ],
            "{t:?}"
        );
    }
    told(&mut a[1], talk(ZPQ_HUB, AURA)).await;
    let t = told(&mut a[1], answer(0, 1, None)).await;
    assert!(
        said(&t, keys::ZAKUM_AURA_PRIZE_TAKEN) && t.items.is_empty(),
        "{t:?}"
    );

    // Claimed, the way out leads back to Adobis, without the quest's items.
    let t = told(&mut a[1], way_out()).await;
    assert_eq!(t.portal_sounds, 1);
    assert_eq!(t.warps, [ZPQ.exit]);
    let mut t = told(&mut a[0], way_out()).await;
    assert_eq!(t.warps, [ZPQ.exit]);
    until(&mut a[0].connection, &mut t, |t| {
        t.removed.contains(&(4, 2))
    })
    .await;
}
