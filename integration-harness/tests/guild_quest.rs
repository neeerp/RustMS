//! The Guild Quest's line: a guild's master registers it with Shuang at the
//! Excavation Site's camp, and as no one else plays on the channel its turn
//! comes at once: its members hear of it (GUILD_OPERATION 0x4C), the master
//! goes into the waiting room, and others of the guild follow while it
//! gathers, the way on shut until then. Another guild registering hears it
//! is next. Once under way, too few staying ends it a few seconds after the
//! client's notice (0x4A), everyone goes to the Returning Path, where Nuris
//! takes them back to the camp, and the next guild's turn comes; alone, its
//! quest ends when the gathering is over. The harness lets two go
//! (`RUSTMS_EXPEDITION_MIN`) and runs its 3 minutes of gathering in 3
//! seconds (`RUSTMS_EVENT_SCALE`).
//!
//! Past the waiting room, Shuang briefs whoever reaches the outskirts, the
//! rocks there drop the protector earring, and the guide on each stage
//! takes a member back to the camp, the quest's items, worn ones too,
//! staying behind.
//!
//! In the hall of statues, the keeper tests the registrant alone: the
//! statues move in turn, the guild strikes them in that order, and the
//! keeper judges. A wrong guess starts the test over; the third part passed
//! opens the gate and earns the guild 15 points.

use game_data::server_content::keys;
use integration_harness::events::{cleared, go_to, said, talk, told, wrong};
use integration_harness::guild_quest::{
    arrived, guild_on, into_the_outskirts, take_the_way_on, touch_script_portal, CAMP, EARRING,
    EARRINGS_SLOT, OUTSKIRTS as FIRST_STAGE, ROCK, SHUANG, WAITING_ROOM,
};
use integration_harness::hunting::{hit_reactor, pickup, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use packet::io::read::PktRead;
use std::io::Cursor;

const RETURNING_PATH: i32 = 990_001_100;
const NURIS: i32 = 9_040_001;
const EXIT_GUIDE: i32 = 9_040_005;
const STATUE_HALL: i32 = 990_000_300;
const STATUE_KEEPER: i32 = 9_040_009;
const STATUES: std::ops::RangeInclusive<i32> = 9_208_000..=9_208_002;
const STATUE_GATE: i32 = 9_208_003;
const PINK: u8 = 5;
const POINTS_CHANGED: u8 = 0x48;
const TOO_FEW: u8 = 0x4A;
const WAITLIST: u8 = 0x4C;

/// New characters, logged in at the camp: the first founds a guild named
/// after them, the rest join it.
async fn guild(size: usize) -> Vec<WorldSession> {
    guild_on(0, size, 0).await
}

/// The waitlist numbers `t` heard (0x4C: channel, number).
fn waitlist(t: &Trace) -> Vec<(u8, i32)> {
    t.guild_ops
        .iter()
        .filter(|op| op[0] == WAITLIST)
        .map(|op| {
            let mut r = Cursor::new(&op[1..]);
            (r.read_byte().unwrap(), r.read_int().unwrap())
        })
        .collect()
}

#[tokio::test]
async fn guilds_take_their_turns_at_the_guild_quest() {
    let mut a = guild(2).await;
    let mut b = guild(1).await;
    let shuang = || talk(CAMP, SHUANG);

    // Only a master or Jr. Master registers the guild.
    let t = told(&mut a[1], shuang()).await;
    assert_eq!(t.prompts, [(SHUANG, 4)]);
    let t = told(&mut a[1], answer(4, 1, Some(0))).await;
    assert!(said(&t, keys::GUILD_QUEST_NOT_MASTER), "{t:?}");
    told(&mut a[1], answer(0, 1, None)).await;

    // The master does; no one plays, so the guild's turn comes at once and
    // the master goes in. Its members hear they are called.
    told(&mut a[0], shuang()).await;
    let t = told(&mut a[0], answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(SHUANG, 1)]);
    let mut master = told(&mut a[0], answer(1, 1, None)).await;
    assert!(said(&master, keys::GUILD_QUEST_REGISTERED), "{master:?}");
    until(&mut a[0].connection, &mut master, |t| {
        arrived(t, WAITING_ROOM) && waitlist(t) == [(0, 1)]
    })
    .await;

    // The member heard it too, and goes in while the guild gathers.
    let t = told(&mut a[1], shuang()).await;
    assert_eq!(waitlist(&t), [(0, 1)], "{t:?}");
    let mut member = told(&mut a[1], answer(4, 1, Some(1))).await;
    until(&mut a[1].connection, &mut member, |t| {
        arrived(t, WAITING_ROOM)
    })
    .await;
    // The way on is shut until the gathering is over.
    take_the_way_on(&mut a[1]).await;
    until(&mut a[1].connection, &mut member, |t| !t.blocked.is_empty()).await;
    assert!(!arrived(&member, FIRST_STAGE), "{member:?}");

    // Another guild registers, and is next.
    told(&mut b[0], shuang()).await;
    told(&mut b[0], answer(4, 1, Some(0))).await;
    let mut other = told(&mut b[0], answer(1, 1, None)).await;
    until(&mut b[0].connection, &mut other, |t| {
        waitlist(t) == [(0, 2)]
    })
    .await;
    told(&mut b[0], answer(0, 1, None)).await;
    // It may not go in yet.
    told(&mut b[0], shuang()).await;
    let t = told(&mut b[0], answer(4, 1, Some(1))).await;
    assert!(said(&t, keys::GUILD_QUEST_NOT_CALLED), "{t:?}");
    told(&mut b[0], answer(0, 1, None)).await;

    // The gathering is over: the quest's clock starts and the way opens.
    until(&mut a[1].connection, &mut member, |t| {
        t.countdowns.iter().any(|&seconds| seconds > 60)
    })
    .await;
    let mut member = Trace::default();
    take_the_way_on(&mut a[1]).await;
    until(&mut a[1].connection, &mut member, |t| {
        arrived(t, FIRST_STAGE)
    })
    .await;

    // The member leaves, which leaves too few: the master hears it, and is
    // sent to the Returning Path.
    warp(&mut a[1].connection, CAMP, "", &mut member).await;
    let mut master = Trace::default();
    until(&mut a[0].connection, &mut master, |t| {
        t.guild_ops.iter().any(|op| op == &[TOO_FEW])
    })
    .await;
    until(&mut a[0].connection, &mut master, |t| {
        arrived(t, RETURNING_PATH) && waitlist(t) == [(0, 0)]
    })
    .await;

    // The next guild is called, and its registrant, at the camp, goes in.
    let mut other = Trace::default();
    until(&mut b[0].connection, &mut other, |t| {
        arrived(t, WAITING_ROOM) && waitlist(t) == [(0, 1)]
    })
    .await;
    // Nuris takes the master back to the camp.
    let t = told(&mut a[0], talk(RETURNING_PATH, NURIS)).await;
    assert_eq!(t.prompts, [(NURIS, 1)]);
    let mut master = told(&mut a[0], answer(1, 1, None)).await;
    until(&mut a[0].connection, &mut master, |t| arrived(t, CAMP)).await;

    // Alone, the next guild's quest ends once its gathering is over.
    until(&mut b[0].connection, &mut other, |t| {
        t.guild_ops.iter().any(|op| op == &[TOO_FEW])
    })
    .await;
    until(&mut b[0].connection, &mut other, |t| {
        arrived(t, RETURNING_PATH) && waitlist(t) == [(0, 1), (0, 0)]
    })
    .await;
}

/// The master registers the guild and the rest of it joins; once the
/// gathering is over everyone goes on to the outskirts. Returns what each
/// saw there.
#[tokio::test]
async fn the_outskirts_rocks_give_earrings_and_the_guide_takes_them_back() {
    // The third already wears an earring, one of a kind.
    let mut team = guild_on(1, 3, 1).await;
    let there = into_the_outskirts(&mut team).await;
    // Shuang briefs each on arrival.
    for (player, t) in team.iter_mut().zip(&there) {
        assert_eq!(t.prompts, [(SHUANG, 0)], "{t:?}");
        assert!(said(t, keys::GUILD_QUEST_BRIEF), "{t:?}");
        told(player, answer(0, 1, None)).await;
    }

    // A rock breaks after its hits and drops an earring.
    let mut there = there;
    let mut seen = there.remove(1);
    let rock = *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == ROCK)
        .expect("a rock on the outskirts");
    let c = &mut team[1].connection;
    send(c, walk_to(rock.x, rock.y)).await;
    let broken = |t: &Trace| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == rock.object_id)
    };
    for hits in 1..=5 {
        send(c, hit_reactor(rock.object_id)).await;
        until(c, &mut seen, |t| {
            broken(t)
                || (t.field.reactor_hits.iter())
                    .filter(|&&(oid, _)| oid == rock.object_id)
                    .count()
                    >= hits
        })
        .await;
    }
    until(c, &mut seen, |t| {
        broken(t) && t.field.item_drop(EARRING).is_some()
    })
    .await;
    let earring = seen.field.item_drop(EARRING).unwrap();
    send(c, walk_to(earring.x, earring.y)).await;
    send(c, pickup(earring.object_id, earring.x, earring.y)).await;
    until(c, &mut seen, |t| {
        t.added.iter().any(|&(_, _, item_id)| item_id == EARRING)
    })
    .await;
    let &(kind, slot, _) = (seen.added.iter())
        .find(|&&(_, _, item_id)| item_id == EARRING)
        .unwrap();

    // The guide takes members back to the camp, and the earrings stay
    // behind: the one picked up and the one worn.
    let t = told(&mut team[1], talk(FIRST_STAGE, EXIT_GUIDE)).await;
    assert_eq!(t.prompts, [(EXIT_GUIDE, 1)]);
    let mut back = told(&mut team[1], answer(1, 1, None)).await;
    until(&mut team[1].connection, &mut back, |t| {
        arrived(t, CAMP) && t.removed.contains(&(kind, slot))
    })
    .await;
    told(&mut team[2], talk(FIRST_STAGE, EXIT_GUIDE)).await;
    let mut back = told(&mut team[2], answer(1, 1, None)).await;
    until(&mut team[2].connection, &mut back, |t| {
        arrived(t, CAMP) && t.removed.iter().any(|&(_, slot)| slot == EARRINGS_SLOT)
    })
    .await;
}

/// The statues' object ids in the hall `t` arrived in.
fn statues(t: &Trace) -> Vec<i32> {
    (t.field.reactors.iter())
        .filter(|r| STATUES.contains(&r.reactor_id))
        .map(|r| r.object_id)
        .collect()
}

/// The statues `t` saw move, in turn.
fn moved(t: &Trace, statues: &[i32]) -> Vec<i32> {
    (t.field.reactor_hits.iter())
        .filter(|(oid, _)| statues.contains(oid))
        .map(|&(oid, _)| oid)
        .collect()
}

/// Whether `t` saw `key` in pink.
fn pink(t: &Trace, key: &str, part: &str) -> bool {
    let text = net::content::text().get(key, &[("part", part)]);
    t.broadcasts.contains(&(PINK, text))
}

/// The registrant hears the keeper's intro, `key`, and goes on: `len`
/// statues move in turn. Returns them in that order.
async fn show_statues(
    master: &mut WorldSession,
    statues: &[i32],
    key: &str,
    len: usize,
) -> Vec<i32> {
    let t = told(master, talk(STATUE_HALL, STATUE_KEEPER)).await;
    assert_eq!(t.prompts, [(STATUE_KEEPER, 0)], "{t:?}");
    assert!(said(&t, key), "{t:?}");
    let mut t = told(master, answer(0, 1, None)).await;
    until(&mut master.connection, &mut t, |t| {
        moved(t, statues).len() >= len && pink(t, keys::GUILD_QUEST_STATUES_SHOWN, "")
    })
    .await;
    let combo = moved(&t, statues);
    assert_eq!(combo.len(), len, "{t:?}");
    assert!(pink(&t, keys::GUILD_QUEST_STATUES_SHOWING, ""), "{t:?}");
    combo
}

/// The registrant strikes `combo` in turn, then asks the keeper.
async fn strike(master: &mut WorldSession, hall: &Trace, combo: &[i32]) -> Trace {
    let c = &mut master.connection;
    let mut t = Trace::default();
    for (n, &oid) in combo.iter().enumerate() {
        let statue = hall.field.reactors.iter().find(|r| r.object_id == oid);
        let statue = statue.unwrap();
        send(c, walk_to(statue.x, statue.y)).await;
        send(c, hit_reactor(oid)).await;
        until(c, &mut t, |t| moved(t, combo).len() > n).await;
    }
    told(master, talk(STATUE_HALL, STATUE_KEEPER)).await
}

#[tokio::test]
async fn the_statue_keepers_test_opens_the_gate() {
    let mut team = guild_on(2, 2, 0).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);
    let (_, gate) = go_to(&mut team[1], STATUE_HALL, STATUE_GATE).await;
    let (mut hall, _) = go_to(&mut team[0], STATUE_HALL, STATUE_GATE).await;
    until(&mut team[0].connection, &mut hall, |t| {
        statues(t).len() >= 20
    })
    .await;
    let statues = statues(&hall);
    assert_eq!(statues.len(), 20, "{hall:?}");

    // Only the registrant is tested, and the gate is shut till then.
    let t = told(&mut team[1], talk(STATUE_HALL, STATUE_KEEPER)).await;
    assert!(said(&t, keys::GUILD_QUEST_STATUES_NOT_LEADER), "{t:?}");
    told(&mut team[1], answer(0, 1, None)).await;
    touch_script_portal(&mut team[1], STATUE_HALL, "next00").await;
    let c = &mut team[1].connection;
    let mut shut = Trace::default();
    until(c, &mut shut, |t| pink(t, keys::GUILD_QUEST_GATE_CLOSED, "")).await;
    assert!(shut.warps.is_empty(), "{shut:?}");

    // A wrong guess, here none, starts the test over.
    let master = &mut team[0];
    show_statues(master, &statues, keys::GUILD_QUEST_STATUES_INTRO, 4).await;
    let mut t = told(master, talk(STATUE_HALL, STATUE_KEEPER)).await;
    assert!(said(&t, keys::GUILD_QUEST_STATUES_WRONG), "{t:?}");
    until(&mut master.connection, &mut t, |t| {
        wrong(t) && pink(t, keys::GUILD_QUEST_STATUES_FAILED, "")
    })
    .await;
    told(master, answer(0, 1, None)).await;

    // Each part shows one statue more; the right order passes it.
    for (part, len) in [(1, 4), (2, 5)] {
        let intro = match part {
            1 => keys::GUILD_QUEST_STATUES_INTRO,
            _ => keys::GUILD_QUEST_STATUES_HARDER,
        };
        let combo = show_statues(master, &statues, intro, len).await;
        let mut t = strike(master, &hall, &combo).await;
        assert!(said(&t, keys::GUILD_QUEST_STATUES_RIGHT), "{t:?}");
        let part = part.to_string();
        until(&mut master.connection, &mut t, |t| {
            pink(t, keys::GUILD_QUEST_STATUES_PART_PASSED, &part)
        })
        .await;
        told(master, answer(0, 1, None)).await;
    }
    let combo = show_statues(master, &statues, keys::GUILD_QUEST_STATUES_HARDER, 6).await;
    let mut t = strike(master, &hall, &combo).await;
    assert!(said(&t, keys::GUILD_QUEST_STATUES_WON), "{t:?}");
    let c = &mut master.connection;
    until(c, &mut t, |t| {
        cleared(t)
            && t.field.reactor_hits.contains(&(gate.object_id, 1))
            && (t.guild_ops.iter()).any(|op| op[0] == POINTS_CHANGED)
    })
    .await;
    let op = (t.guild_ops.iter()).find(|op| op[0] == POINTS_CHANGED);
    let mut r = Cursor::new(&op.unwrap()[1..]);
    let (_guild, total) = (r.read_int().unwrap(), r.read_int().unwrap());
    assert_eq!(total, 15, "{t:?}");
    told(master, answer(0, 1, None)).await;

    // The gate lets the guild on.
    touch_script_portal(&mut team[1], STATUE_HALL, "next00").await;
    let c = &mut team[1].connection;
    let mut on = Trace::default();
    until(c, &mut on, |t| arrived(t, STATUE_HALL + 1)).await;
}
