//! The Amorian Challenge's first stage: its side rooms let in only men on
//! the right and women on the left; once the stage is bare the Glimmer Man
//! brings out the fierry, whose Wing Hammer breaks the Magik Mirror; the
//! leader hands the Glimmer Man its shard, and Amos then clears the stage and opens
//! one of its three ways up, drawn at random, to its own copy of stage 2,
//! the others staying shut.

use game_data::server_content::keys;
use integration_harness::events::{
    cleared, couples_in, enter_amoria, go_to, party, said, talk, throw_down, told,
};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::hunting::{attack, pickup, walk_to};
use integration_harness::packets::build_change_map;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    AMORIA_ENTRY, AMORIA_LOBBY, AMORIA_STAGE2, AMORIA_WAYS, FIERRY, MAGIK_MIRROR_SHARD,
};
use net::npc_scripts::{AMOS_STAGE, GLIMMER_MAN};

const MAGIK_MIRROR: i32 = 6_702_000;
const WING_HAMMER: i32 = 4_031_596;
const STAGE_MOBS: [i32; 3] = [9_400_533, 9_400_534, 9_400_535];
const PINK: u8 = 5;
const ETC: u8 = 4;

/// The leader beats every mob on the stage as it comes out, till a spawn
/// pass brings out no more.
async fn bare(leader: &mut WorldSession, seen: &mut Trace) {
    let c = &mut leader.connection;
    for _ in 0..20 {
        let mobs: Vec<i32> = (STAGE_MOBS.iter())
            .flat_map(|&mob_id| seen.field.alive(mob_id))
            .collect();
        for &oid in &mobs {
            send(c, attack(oid, 30_000)).await;
        }
        until(c, seen, |t| {
            mobs.iter().all(|oid| t.field.killed.contains(oid))
        })
        .await;
        // A spawn pass is every 7 seconds.
        read_for(c, seen, 8_000).await;
        if STAGE_MOBS.iter().all(|&m| seen.field.alive(m).is_empty()) {
            return;
        }
    }
    panic!("the stage never went bare: {seen:?}");
}

/// `player` hears `key` from `npc` on the stage, and closes it.
async fn hears(player: &mut WorldSession, npc: i32, key: &str) -> Trace {
    let t = told(player, talk(AMORIA_ENTRY, npc)).await;
    assert!(said(&t, key), "{t:?}");
    told(player, answer(0, 1, None)).await;
    t
}

/// `player` takes the stage's way `name` and is turned back.
async fn shut(player: &mut WorldSession, name: &str) {
    let mut t = Trace::default();
    send(&mut player.connection, build_change_map(-1, name).unwrap()).await;
    until(&mut player.connection, &mut t, |t| !t.blocked.is_empty()).await;
    assert!(t.warps.is_empty(), "{name} {t:?}");
}

// One test: a channel holds one run at a time.
#[tokio::test]
async fn the_glimmer_man_s_shard_lets_amos_open_one_way_up() {
    let mut team = couples_in(AMORIA_LOBBY, 40, 3, &[]).await;
    party(&mut team).await;
    enter_amoria(&mut team).await;

    // The side rooms: a man is turned back from the women's room, and
    // goes into the men's.
    let man = &mut team[2];
    touch_script_portal(man, AMORIA_ENTRY, "female00").await;
    let text = net::content::text().get(keys::AMORIA_PQ_GENDER_WOMEN_ONLY, &[]);
    let mut t = Trace::default();
    until(&mut man.connection, &mut t, |t| {
        t.broadcasts.contains(&(PINK, text.clone()))
    })
    .await;
    assert!(t.warps.is_empty(), "{t:?}");
    touch_script_portal(man, AMORIA_ENTRY, "male00").await;
    until(&mut man.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| (a.0, a.1) == (AMORIA_ENTRY, 3))
    })
    .await;

    // A woman is turned back from the men's, and goes into the women's.
    let woman = &mut team[3];
    touch_script_portal(woman, AMORIA_ENTRY, "male00").await;
    let text = net::content::text().get(keys::AMORIA_PQ_GENDER_MEN_ONLY, &[]);
    let mut t = Trace::default();
    until(&mut woman.connection, &mut t, |t| {
        t.broadcasts.contains(&(PINK, text.clone()))
    })
    .await;
    assert!(t.warps.is_empty(), "{t:?}");
    touch_script_portal(woman, AMORIA_ENTRY, "female00").await;
    until(&mut woman.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| (a.0, a.1) == (AMORIA_ENTRY, 4))
    })
    .await;

    // Amos tells the leader what the stage asks, and the others to fetch
    // the leader; the Glimmer Man tells them all the task.
    hears(&mut team[1], AMOS_STAGE, keys::AMORIA_PQ_AMOS_NOT_LEADER).await;
    hears(
        &mut team[1],
        GLIMMER_MAN,
        keys::AMORIA_PQ_GLIMMER_MEMBER_TASK,
    )
    .await;
    hears(&mut team[0], AMOS_STAGE, keys::AMORIA_PQ_STAGE1_INTRO).await;
    hears(&mut team[0], GLIMMER_MAN, keys::AMORIA_PQ_GLIMMER_TASK).await;

    // Every way up is shut.
    for way in AMORIA_WAYS {
        shut(&mut team[0], way).await;
    }

    // With the stage bare, the Glimmer Man brings out the fierry, once.
    let (mut seen, mirror) = go_to(&mut team[0], AMORIA_ENTRY, MAGIK_MIRROR).await;
    bare(&mut team[0], &mut seen).await;
    let mut t = told(&mut team[0], talk(AMORIA_ENTRY, GLIMMER_MAN)).await;
    assert!(said(&t, keys::AMORIA_PQ_GLIMMER_FIERRY_OUT), "{t:?}");
    let c = &mut team[0].connection;
    until(c, &mut t, |t| t.field.alive(FIERRY).len() == 1).await;
    let fierry = t.field.alive(FIERRY)[0];
    told(&mut team[0], answer(0, 1, None)).await;
    hears(
        &mut team[0],
        GLIMMER_MAN,
        keys::AMORIA_PQ_GLIMMER_BEAT_FIERRY,
    )
    .await;
    hears(&mut team[0], AMOS_STAGE, keys::AMORIA_PQ_STAGE1_SEE_GLIMMER).await;

    // Its Wing Hammer, thrown on the mirror, breaks it into the shard.
    let c = &mut team[0].connection;
    let mut seen = Trace::default();
    send(c, attack(fierry, 30_000)).await;
    until(c, &mut seen, |t| t.field.item_drop(WING_HAMMER).is_some()).await;
    let hammer = seen.field.item_drop(WING_HAMMER).unwrap();
    send(c, pickup(hammer.object_id, hammer.x, hammer.y)).await;
    until(c, &mut seen, |t| t.added.iter().any(|a| a.2 == WING_HAMMER)).await;
    let slot = (seen.added.iter()).find(|a| a.2 == WING_HAMMER).unwrap().1;
    // The mirror sits a little under its floor: stand on the floor.
    send(c, walk_to(mirror.x, mirror.y - 10)).await;
    send(c, throw_down(slot, 1)).await;
    until(c, &mut seen, |t| {
        t.field.item_drop(MAGIK_MIRROR_SHARD).is_some()
    })
    .await;
    let shard = seen.field.item_drop(MAGIK_MIRROR_SHARD).unwrap();
    send(c, pickup(shard.object_id, shard.x, shard.y)).await;
    until(c, &mut seen, |t| {
        t.added.iter().any(|a| a.2 == MAGIK_MIRROR_SHARD)
    })
    .await;
    let slot = (seen.added.iter()).find(|a| a.2 == MAGIK_MIRROR_SHARD);
    let slot = slot.unwrap().1;

    // Amos won't clear the stage before the leader hands the shard to the
    // Glimmer Man, who takes it.
    hears(&mut team[0], AMOS_STAGE, keys::AMORIA_PQ_STAGE1_SEE_GLIMMER).await;
    let t = hears(
        &mut team[0],
        GLIMMER_MAN,
        keys::AMORIA_PQ_GLIMMER_SHARD_TAKEN,
    )
    .await;
    assert!(t.removed.contains(&(ETC, slot)), "{t:?}");

    // Amos then clears it without a word, showing the gate of the way
    // drawn.
    send(&mut team[0].connection, talk(AMORIA_ENTRY, AMOS_STAGE)).await;
    let gates = ["gate0", "gate1", "gate2"];
    let mut t = Trace::default();
    until(&mut team[0].connection, &mut t, |t| {
        cleared(t) && (t.field_effects.iter()).any(|e| gates.contains(&e.1.as_str()))
    })
    .await;
    assert!(t.prompts.is_empty(), "{t:?}");
    let gate = (t.field_effects.iter())
        .find(|e| gates.contains(&e.1.as_str()))
        .unwrap()
        .clone();
    let way: usize = gate.1[4..].parse().unwrap();

    // The way drawn leads to its copy of stage 2; the others stay shut.
    for (_, name) in (AMORIA_WAYS.into_iter().enumerate()).filter(|&(n, _)| n != way) {
        shut(&mut team[1], name).await;
    }
    let mut t = Trace::default();
    let c = &mut team[1].connection;
    send(c, build_change_map(-1, AMORIA_WAYS[way]).unwrap()).await;
    until(c, &mut t, |t| t.warps.contains(&AMORIA_STAGE2[way])).await;

    // One coming onto the stage after sees the gate open, and Amos says
    // so.
    let mut seen = Trace::default();
    let c = &mut team[2].connection;
    assert_eq!(warp(c, AMORIA_ENTRY, "", &mut seen).await, AMORIA_ENTRY);
    until(c, &mut seen, |t| t.field_effects.contains(&gate)).await;
    hears(&mut team[0], AMOS_STAGE, keys::AMORIA_PQ_AMOS_CLEARED).await;
}
