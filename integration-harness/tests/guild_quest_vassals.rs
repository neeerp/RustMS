//! The Guild Quest's fountain: four vassals each wish for one of four
//! kinds of offering, which the harness pins to 0, 1, 2 and 3 in turn
//! (`RUSTMS_GQ_VASSALS`). The registrant hears of them, sets one before
//! each and asks the keeper, who hints at a wrong offering, bringing out a
//! knight of each kind; the right one opens the water gate, clears the
//! stage, takes the offering and earns the guild 25 points.

use game_data::server_content::keys;
use integration_harness::events::{cleared, go_to, said, talk, throw_down, told, wrong};
use integration_harness::guild_quest::{
    arrived, guild_with, into_the_outskirts, touch_script_portal,
};
use integration_harness::hunting::{pickup, walk_to, MapDrop};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use integration_harness::WorldSession;
use packet::io::read::PktRead;
use std::io::Cursor;

const FOUNTAIN: i32 = 990_000_500;
const VASSAL_KEEPER: i32 = 9_040_006;
const WATER_GATE: i32 = 9_208_013;
/// The four kinds, held in the Etc slots from 1.
const OFFERINGS: [i32; 4] = [4_001_027, 4_001_028, 4_001_029, 4_001_030];
const KNIGHTS: [i32; 2] = [9_300_036, 9_300_037];
/// Where on the floor each vassal's offering goes.
const VASSALS_X: [i16; 4] = [-185, -40, 137, 270];
const FLOOR_Y: i16 = 150;
const POINTS_CHANGED: u8 = 0x48;

/// The registrant asks the vassals' keeper and closes the talk: what they
/// saw.
async fn ask(master: &mut WorldSession) -> Trace {
    let c = &mut master.connection;
    let mut t = Trace::default();
    send(c, talk(FOUNTAIN, VASSAL_KEEPER)).await;
    until(c, &mut t, |t| !t.prompts.is_empty()).await;
    assert_eq!(t.prompts, [(VASSAL_KEEPER, 0)], "{t:?}");
    send(c, answer(0, 1, None)).await;
    t
}

/// The registrant throws down the `item` in Etc `slot` before the vassal
/// `vassal`: the drop.
async fn offer(master: &mut WorldSession, slot: i16, item: i32, vassal: usize) -> MapDrop {
    let c = &mut master.connection;
    send(c, walk_to(VASSALS_X[vassal], FLOOR_Y)).await;
    send(c, throw_down(slot, 1)).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| t.field.item_drop(item).is_some()).await;
    t.field.item_drop(item).unwrap()
}

/// The registrant picks `drop` back up.
async fn take_back(master: &mut WorldSession, drop: MapDrop) {
    let c = &mut master.connection;
    send(c, walk_to(drop.x, drop.y)).await;
    send(c, pickup(drop.object_id, drop.x, drop.y)).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.field.removals(drop.object_id).is_empty()).await;
}

#[tokio::test]
async fn the_vassals_take_the_right_offering_and_open_the_water_gate() {
    let items = OFFERINGS.map(|item| (item, 1));
    let mut team = guild_with(0, 2, 2, &items).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);
    let (mut fountain, gate) = go_to(&mut team[1], FOUNTAIN, WATER_GATE).await;
    go_to(&mut team[0], FOUNTAIN, WATER_GATE).await;

    // Only the registrant is heard, and the gate is shut till then.
    let t = told(&mut team[1], talk(FOUNTAIN, VASSAL_KEEPER)).await;
    assert!(said(&t, keys::GUILD_QUEST_VASSALS_NOT_LEADER), "{t:?}");
    told(&mut team[1], answer(0, 1, None)).await;
    touch_script_portal(&mut team[1], FOUNTAIN, "next00").await;
    let mut shut = Trace::default();
    let text = net::content::text().get(keys::GUILD_QUEST_GATE_CLOSED, &[]);
    until(&mut team[1].connection, &mut shut, |t| {
        t.broadcasts.iter().any(|(_, said)| *said == text)
    })
    .await;
    assert!(shut.warps.is_empty(), "{shut:?}");

    // The registrant hears of the vassals, and nothing set is no try.
    let master = &mut team[0];
    let t = ask(master).await;
    assert!(said(&t, keys::GUILD_QUEST_VASSALS_INTRO), "{t:?}");
    let t = ask(master).await;
    assert!(said(&t, keys::GUILD_QUEST_VASSALS_NO_OFFERING), "{t:?}");

    // The first two swapped: two pleased, two wrong; a knight of each
    // kind comes.
    let mut drops = vec![
        offer(master, 2, OFFERINGS[1], 0).await,
        offer(master, 1, OFFERINGS[0], 1).await,
        offer(master, 3, OFFERINGS[2], 2).await,
        offer(master, 4, OFFERINGS[3], 3).await,
    ];
    let mut t = ask(master).await;
    let hint = net::content::text().get(
        keys::GUILD_QUEST_VASSALS_HINT,
        &[
            ("attempt", "1"),
            ("pleased", "2"),
            ("wrong", "2"),
            ("unknown", "0"),
        ],
    );
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&hint)),
        "{t:?}"
    );
    assert!(wrong(&t), "{t:?}");
    until(&mut master.connection, &mut t, |t| {
        KNIGHTS
            .iter()
            .all(|&knight| !t.field.alive(knight).is_empty())
    })
    .await;

    // Set right (the two picked up fill the first free slots in turn), the
    // gate opens, the stage clears, the offering goes and the guild earns
    // its points.
    take_back(master, drops.remove(0)).await;
    take_back(master, drops.remove(0)).await;
    drops.push(offer(master, 2, OFFERINGS[0], 0).await);
    drops.push(offer(master, 1, OFFERINGS[1], 1).await);
    let mut t = ask(master).await;
    assert!(said(&t, keys::GUILD_QUEST_VASSALS_RIGHT), "{t:?}");
    until(&mut master.connection, &mut t, |t| {
        cleared(t)
            && (t.guild_ops.iter()).any(|op| op[0] == POINTS_CHANGED)
            && drops
                .iter()
                .all(|d| !t.field.removals(d.object_id).is_empty())
    })
    .await;
    let op = (t.guild_ops.iter()).find(|op| op[0] == POINTS_CHANGED);
    let mut r = Cursor::new(&op.unwrap()[1..]);
    let (_guild, total) = (r.read_int().unwrap(), r.read_int().unwrap());
    assert_eq!(total, 25, "{t:?}");
    let t = ask(master).await;
    assert!(said(&t, keys::GUILD_QUEST_VASSALS_CLEARED), "{t:?}");

    // The water gate lets the guild on.
    let c = &mut team[1].connection;
    until(c, &mut fountain, |t| {
        t.field.reactor_hits.contains(&(gate.object_id, 1)) && cleared(t)
    })
    .await;
    touch_script_portal(&mut team[1], FOUNTAIN, "next00").await;
    let mut on = Trace::default();
    until(&mut team[1].connection, &mut on, |t| {
        arrived(t, FOUNTAIN + 100)
    })
    .await;
}
