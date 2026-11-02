//! An Aran's walk across Snow Island, from the Ice Cave to Rien: Lilin
//! wakes the hero, then leads the way into the Cold Forest as the guide.
//! The penguins' quests open its maps one at a time: Puka's potion (21010),
//! Puir's sword (21011), Purun's Murus (21012) and Putzki's chair (21013),
//! whose parts come out of the boxes in the fourth map. In the last map
//! Lilin goes ahead toward town.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{complete_quests, replace_item, set_hp, set_job, set_str};
use integration_harness::events::{maps, said, talk, told};
use integration_harness::hunting::{attack, hit_reactor, pickup, use_item, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START, START};
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ICE_CAVE: i32 = 140090000;
const COLD_FOREST: [i32; 5] = [140090100, 140090200, 140090300, 140090400, 140090500];
const LILIN: i32 = 1202000;
const PUKA: i32 = 1202001;
const PUEN: i32 = 1202002;
const PUIR: i32 = 1202003;
const PURUN: i32 = 1202004;
const PUTZKI: i32 = 1202005;
const RETURN_OF_THE_HERO: i16 = 21010;
const MISSING_WEAPON: i16 = 21011;
const ABILITIES_LOST: i16 = 21012;
const GIFT_FOR_THE_HERO: i16 = 21013;
const RIEN_RED_POTION: i32 = 2000022;
const SWORD: i32 = 1302000;
const MURU: i32 = 9300383;
const BOX: i32 = 1402000;
const CHAIR_PARTS: [i32; 2] = [4032309, 4032310];
const BAMBOO_CHAIR: i32 = 3010062;
const USE: u8 = 2;
const PINK: u8 = 5;

/// Going through the scripted portal `portal` of the current map.
async fn enter(c: &mut MapleTestConnection, m: &str, map: i32, portal: &str) -> Trace {
    let field = maps().field(map).unwrap();
    let at = field.portals().find(|p| p.name == portal).unwrap();
    step(c, m, walk_to(at.x as i16, at.y as i16)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    step(c, m, p).await
}

/// Walk to NPC `npc` on `map`, send `first`, and page through what it says,
/// saying yes to any question: everything seen, a step at a time.
async fn through(s: &mut WorldSession, map: i32, npc: i32, first: Packet) -> Vec<Trace> {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    told(s, walk_to(at.x, at.y)).await;
    let mut seen = vec![told(s, first).await];
    while let Some(&(_, kind)) = seen.last().unwrap().prompts.last() {
        let reply = if kind == 1 || kind == 12 {
            answer(kind, 1, None)
        } else {
            answer(0, 1, None)
        };
        seen.push(told(s, reply).await);
        assert!(seen.len() < 20, "{seen:?}");
    }
    seen
}

fn state(seen: &[Trace], quest: i16) -> Option<u8> {
    seen.iter()
        .rev()
        .find_map(|t| t.states.get(&quest).copied())
}

#[tokio::test]
async fn lilin_and_the_penguins_see_an_aran_from_the_ice_cave_to_rien() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2000, 1, 50).await.expect("an Aran beginner");
    set_hp(id, 50, 50).await.unwrap();
    // Strong enough to beat a Muru in a hit.
    replace_item(id, -1, -11, SWORD).await.unwrap();
    set_str(id, 500).await.unwrap();
    // Off the Black Road.
    complete_quests(id, 21000, 21001).await.unwrap();

    let mut s = relogin(&config).await;
    let m = s.character_name.clone();
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut s.connection, ICE_CAVE, "", &mut t).await,
        ICE_CAVE
    );
    barrier(&mut s.connection, &m, &mut t).await;
    assert!(
        t.scene_parts.iter().any(|p| p.ends_with("ClickLilin")),
        "{t:?}"
    );

    // Lilin wakes the hero, then leads the way to the forest as the guide.
    let seen = through(&mut s, ICE_CAVE, LILIN, talk(ICE_CAVE, LILIN)).await;
    assert!(said(&seen[0], keys::ICE_CAVE_LILIN_WAKES[0]), "{seen:?}");
    assert!(seen.iter().all(|t| t.warps.is_empty()), "{seen:?}");
    let seen = through(&mut s, ICE_CAVE, LILIN, talk(ICE_CAVE, LILIN)).await;
    assert!(said(&seen[0], keys::ICE_CAVE_LILIN_LEADS[0]), "{seen:?}");
    let last = seen.last().unwrap();
    assert_eq!(last.warps, [COLD_FOREST[0]], "{seen:?}");
    assert!(last.guides.contains(&true), "{seen:?}");

    // The way on stays shut until Puka's quest is done.
    let t = enter(&mut s.connection, &m, COLD_FOREST[0], "west00").await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert!(t.broadcasts.iter().any(|b| b.0 == PINK), "{t:?}");

    // Puka hurts the hero and hands over a potion to drink.
    let start = build_quest_action(SCRIPT_START, RETURN_OF_THE_HERO, PUKA);
    let seen = through(&mut s, COLD_FOREST[0], PUKA, start).await;
    assert_eq!(state(&seen, RETURN_OF_THE_HERO), Some(1), "{seen:?}");
    let (_, slot, _) = *seen
        .iter()
        .flat_map(|t| &t.added)
        .find(|a| a.0 == USE && a.2 == RIEN_RED_POTION)
        .unwrap_or_else(|| panic!("{seen:?}"));
    // Still holding it, Puka has nothing to say yet.
    let end = || build_quest_action(SCRIPT_END, RETURN_OF_THE_HERO, PUKA);
    let t = told(&mut s, end()).await;
    assert!(t.prompts.is_empty(), "{t:?}");
    told(&mut s, use_item(slot, RIEN_RED_POTION)).await;
    let seen = through(&mut s, COLD_FOREST[0], PUKA, end()).await;
    assert!(said(&seen[0], keys::PUKA_THANKS[0]), "{seen:?}");
    assert_eq!(state(&seen, RETURN_OF_THE_HERO), Some(2), "{seen:?}");
    assert!(seen.iter().any(|t| t.gains.contains(&16)), "{seen:?}");
    let t = enter(&mut s.connection, &m, COLD_FOREST[0], "west00").await;
    assert_eq!(t.warps, [COLD_FOREST[1]], "{t:?}");

    // Puen asks after the sword, and Puir hands it over.
    let start = build_quest_action(START, MISSING_WEAPON, PUEN);
    let seen = through(&mut s, COLD_FOREST[1], PUEN, start).await;
    assert_eq!(state(&seen, MISSING_WEAPON), Some(1), "{seen:?}");
    let end = build_quest_action(SCRIPT_END, MISSING_WEAPON, PUIR);
    let seen = through(&mut s, COLD_FOREST[1], PUIR, end).await;
    assert_eq!(state(&seen, MISSING_WEAPON), Some(2), "{seen:?}");
    assert!(
        seen.iter().any(|t| t.items.contains(&(SWORD, 1))),
        "{seen:?}"
    );
    let mut t = enter(&mut s.connection, &m, COLD_FOREST[1], "west00").await;
    assert_eq!(t.warps, [COLD_FOREST[2]], "{t:?}");

    // Purun wants three Murus beaten.
    let start = build_quest_action(SCRIPT_START, ABILITIES_LOST, PURUN);
    let seen = through(&mut s, COLD_FOREST[2], PURUN, start).await;
    assert_eq!(state(&seen, ABILITIES_LOST), Some(1), "{seen:?}");
    let c = &mut s.connection;
    until(c, &mut t, |t| t.field.alive(MURU).len() >= 3).await;
    // Each where it came out, in the map's order.
    let spawns = &maps().field(COLD_FOREST[2]).unwrap().mob_spawns;
    for (oid, at) in t.field.alive(MURU).into_iter().zip(spawns).take(3) {
        send(c, walk_to(at.x, at.y)).await;
        send(c, attack(oid, 8)).await;
        until(c, &mut t, |t| t.field.killed.contains(&oid)).await;
    }
    barrier(c, &m, &mut t).await;
    let end = build_quest_action(SCRIPT_END, ABILITIES_LOST, PURUN);
    let seen = through(&mut s, COLD_FOREST[2], PURUN, end).await;
    assert_eq!(state(&seen, ABILITIES_LOST), Some(2), "{seen:?}");
    let mut t = enter(&mut s.connection, &m, COLD_FOREST[2], "west00").await;
    assert_eq!(t.warps, [COLD_FOREST[3]], "{t:?}");

    // Putzki wants a chair, whose parts come out of the boxes.
    let start = build_quest_action(SCRIPT_START, GIFT_FOR_THE_HERO, PUTZKI);
    let seen = through(&mut s, COLD_FOREST[3], PUTZKI, start).await;
    assert_eq!(state(&seen, GIFT_FOR_THE_HERO), Some(1), "{seen:?}");
    let c = &mut s.connection;
    until(c, &mut t, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == BOX)
    })
    .await;
    let r = *t
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == BOX)
        .unwrap();
    send(c, walk_to(r.x, r.y)).await;
    let part = |t: &Trace, item: i32| {
        (t.field.drops.iter())
            .find(|d| d.value == item && !t.field.removed.iter().any(|x| x.0 == d.object_id))
            .copied()
    };
    for hit in 1.. {
        send(c, hit_reactor(r.object_id)).await;
        until(c, &mut t, |t| {
            t.field.reactor_hits.contains(&(r.object_id, hit))
                || CHAIR_PARTS.iter().all(|&i| part(t, i).is_some())
        })
        .await;
        if CHAIR_PARTS.iter().all(|&i| part(&t, i).is_some()) {
            break;
        }
        assert!(hit < 10, "{t:?}");
    }
    for item in CHAIR_PARTS {
        let drop = part(&t, item).unwrap();
        send(c, pickup(drop.object_id, drop.x, drop.y)).await;
        until(c, &mut t, |t| {
            t.field.removed.iter().any(|x| x.0 == drop.object_id)
        })
        .await;
    }
    let end = build_quest_action(SCRIPT_END, GIFT_FOR_THE_HERO, PUTZKI);
    let seen = through(&mut s, COLD_FOREST[3], PUTZKI, end).await;
    assert_eq!(state(&seen, GIFT_FOR_THE_HERO), Some(2), "{seen:?}");
    let items: Vec<_> = seen.iter().flat_map(|t| t.items.clone()).collect();
    for item in CHAIR_PARTS {
        assert!(items.contains(&(item, -1)), "{seen:?}");
    }
    assert!(items.contains(&(BAMBOO_CHAIR, 1)), "{seen:?}");
    let t = enter(&mut s.connection, &m, COLD_FOREST[3], "west00").await;
    assert_eq!(t.warps, [COLD_FOREST[4]], "{t:?}");

    // Near town, Lilin goes ahead, then is gone.
    let t = enter(&mut s.connection, &m, COLD_FOREST[4], "tutor00").await;
    let ahead = net::content::text().get(keys::LILIN_GOES_AHEAD, &[]);
    assert!(t.guide_talk.contains(&ahead), "{t:?}");
    let t = enter(&mut s.connection, &m, COLD_FOREST[4], "tutor01").await;
    assert!(t.guides.contains(&false), "{t:?}");
}
