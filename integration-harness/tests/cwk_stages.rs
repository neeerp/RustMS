//! Crimsonwood Keep's stages and their ways on. The first's way on stays
//! shut until a member finds the stage's hidden way, behind the pillar by
//! the Lion Shield; that takes the finder back to the stage's start and
//! clears it for all, with its EXP and mesos. The way on then leads to the
//! second stage, on its own clock, whose way on stays shut until a skill of
//! each job has lit that job's sigil. The third's way on waits on its
//! sigils, which set its menhirs going, and on five trips up its way up;
//! the fourth wakes a sigil of each job, which clear it once lit. The
//! fifth's hub has a room for each job, whose keeper gives back its weapon
//! once the room is won, and an altar for each weapon; with all five
//! restored, it clears. The sixth's host, for the leader alone, brings out
//! an escort and then the four Twisted Masters, and once they are beaten
//! wins the Keep for all, opening the way to the rest room and on to the
//! treasure room, whose clock sees the team out. The harness lets two set
//! out (`RUSTMS_EXPEDITION_MIN`) and runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the first stage's clock lasts 2 seconds, the
//! next three's 10, the fifth's 20, the sixth's 60 and the treasure room's
//! 1. Both are made strong enough to beat the masters in time.

use game_data::server_content::keys;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_item, grant_skill, set_job, set_str, set_weapon_attack};
use integration_harness::events::{cleared, said, talk, throw_down, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::hunting::{attack, hit_reactor, skill_at_reactor, walk_to, MapReactor};
use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{
    cwk_menhirs, CWK, CWK_ENTRY, CWK_ESCORT, CWK_EYE, CWK_HIDDEN_WAY, CWK_HOST, CWK_JACK,
    CWK_LATER_SIGILS, CWK_LOBBY, CWK_MASTERS, CWK_MASTER_GUARDIAN, CWK_MENHIR_LINES, CWK_ROOMS,
    CWK_SIGILS, CWK_SIGIL_LINES,
};
use net::npc_scripts::CWK_RECRUITER;
use tokio::time::Duration;

const KEYSTONE: i32 = 3_992_041;
const SETUP: i16 = 3;
const PINK: u8 = 5;
/// The stages' way on, `next00`.
const WAY_ON: &str = "next00";
const STAGE2: i32 = CWK_ENTRY + 100;
const STAGE3: i32 = CWK_ENTRY + 200;
const STAGE4: i32 = CWK_ENTRY + 300;
const HUB: i32 = CWK_ENTRY + 400;
const STAGE6: i32 = CWK_ENTRY + 500;
const REST: i32 = CWK_ENTRY + 600;
const TREASURE: i32 = CWK_ENTRY + 700;
/// Under the most a character with 32000 STR and a 1999-attack weapon may
/// hit for.
const HIT: i32 = 5_000_000;
const ETC: i16 = 4;
/// The fifth stage's altars, in job order, and a spot on the ground in
/// each one's area.
const ALTARS: [(i32, i16, i16); 5] = [
    (6_109_005, 1093, -586),
    (6_109_006, 1280, -415),
    (6_109_007, 825, -939),
    (6_109_008, 1343, -887),
    (6_109_009, 1296, -559),
];
/// The third stage's way up, and its way on.
const WAY_UP: &str = "next00";
const STAGE3_WAY_ON: &str = "next01";
/// The fourth stage's fakes.
const FAKES: [i32; 3] = [6_108_000, 6_108_001, 6_108_005];
/// A skill of each job that lights its sigil: a White Knight's, a
/// Ranger's, a Fire/Poison Wizard's, a Hermit's and a Marauder's.
const SIGIL_SKILLS: [i32; 5] = [1_211_002, 3_211_006, 2_111_002, 4_111_005, 5_111_005];
const HERMIT: i16 = 411;
const RANGER: i16 = 321;

/// A new character of level 90 of `job` with the Keystone, `skills` and
/// the Etc items `etc`, logged in at the Inner Sanctum's entrance.
async fn adventurer(job: i16, skills: &[i32], etc: &[i32]) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, 90, 50).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    grant_item(id, SETUP, 1, KEYSTONE).await.unwrap();
    for &skill in skills {
        grant_skill(id, skill, 1).await.unwrap();
    }
    for (slot, &item) in (1..).zip(etc) {
        grant_item(id, ETC, slot, item).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, CWK_LOBBY, "", &mut seen).await, CWK_LOBBY);
    session
}

/// `player` hears `key` in pink.
async fn hears(player: &mut WorldSession, t: &mut Trace, key: &str) {
    let line = (PINK, net::content::text().get(key, &[]));
    until(&mut player.connection, t, |t| t.broadcasts.contains(&line)).await;
}

/// The item line `key`.
fn line(key: &str) -> (String, i32) {
    (net::content::text().get(key, &[]), 0)
}

/// The sigil of job `job`, from 0, as `t` saw it.
fn sigil(t: &Trace, job: i32) -> Option<MapReactor> {
    let mut reactors = t.field.reactors.iter();
    reactors.find(|r| r.reactor_id == CWK_SIGILS + job).copied()
}

/// The line "`n` more to go" up the third stage's way up.
fn to_go(n: u8) -> (String, i32) {
    let n = n.to_string();
    (
        net::content::text().get(keys::CWK_STEPS_TO_GO, &[("n", &n)]),
        0,
    )
}

/// Whether `t` has seen all five sigils.
fn all_sigils(t: &Trace) -> bool {
    (0..5).all(|job| sigil(t, job).is_some())
}

// One test: a channel holds one expedition at a time.
#[tokio::test]
async fn the_keep_s_first_stages_clear_by_the_hidden_way_and_the_sigils() {
    // The leader, a thief, has the other jobs' weapons already.
    let others: Vec<_> = (CWK_ROOMS.iter())
        .map(|room| room.weapon)
        .filter(|&weapon| weapon != CWK_ROOMS[3].weapon)
        .collect();
    let mut leader = adventurer(HERMIT, &SIGIL_SKILLS, &others).await;
    let mut member = adventurer(RANGER, &SIGIL_SKILLS[1..2], &[]).await;
    let recruiter = || talk(CWK_LOBBY, CWK_RECRUITER);

    // The leader gathers an expedition, the member joins, and they set
    // out.
    told(&mut leader, recruiter()).await;
    told(&mut leader, answer(1, 1, None)).await;
    told(&mut leader, answer(0, 1, None)).await;
    told(&mut member, recruiter()).await;
    told(&mut member, answer(1, 1, None)).await;
    told(&mut member, answer(0, 1, None)).await;
    told(&mut leader, recruiter()).await;
    let mut lt = told(&mut leader, answer(4, 1, Some(1))).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&CWK_ENTRY)
    })
    .await;
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&CWK_ENTRY)
    })
    .await;

    // The way on is shut.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, CWK_ENTRY, WAY_ON).await;
    hears(&mut leader, &mut lt, keys::CWK_WAY_BLOCKED).await;
    assert!(lt.warps.is_empty(), "{lt:?}");

    // The member finds the hidden way and is taken back to the start.
    let mut mt = Trace::default();
    send(
        &mut member.connection,
        walk_to(CWK_HIDDEN_WAY.0, CWK_HIDDEN_WAY.1),
    )
    .await;
    until(&mut member.connection, &mut mt, |t| {
        t.arrivals.iter().any(|a| (a.0, a.1) == (CWK_ENTRY, 0))
    })
    .await;

    // Both hear of it, clear the stage and get its EXP and mesos.
    let (exp, meso) = (CWK.stage_exp(1), CWK.stage_meso(1));
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        hears(player, t, keys::CWK_STAGE1_FOUND).await;
        until(&mut player.connection, t, |t| {
            cleared(t) && t.gains.contains(&exp) && t.meso_gains.contains(&meso)
        })
        .await;
    }

    // Now the way on leads to the second stage, whose clock starts over
    // (the first's may show first).
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, CWK_ENTRY, WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&STAGE2) && t.countdowns.iter().any(|&clock| clock > 2) && all_sigils(t)
    })
    .await;
    assert!(lt.countdowns.iter().all(|&clock| clock <= 10), "{lt:?}");
    let sigils: Vec<_> = (0..5).filter_map(|job| sigil(&lt, job)).collect();
    let mut mt = Trace::default();
    touch_script_portal(&mut member, CWK_ENTRY, WAY_ON).await;
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&STAGE2) && all_sigils(t)
    })
    .await;

    // Whose own way on is shut until its sigils are lit.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, STAGE2, WAY_ON).await;
    hears(&mut leader, &mut lt, keys::CWK_WAY_NOT_ACTIVATED).await;
    assert!(lt.warps.is_empty(), "{lt:?}");

    // By the warriors' sigil, the bowman's own skill and a warrior's they
    // don't have do nothing; by the bowmen's, their skill lights it, and
    // everyone hears.
    let (warriors, bowmen) = (sigils[0], sigils[1]);
    let c = &mut member.connection;
    send(c, walk_to(warriors.x, warriors.y)).await;
    send(c, skill_at_reactor(warriors.object_id, SIGIL_SKILLS[1])).await;
    send(c, skill_at_reactor(warriors.object_id, SIGIL_SKILLS[0])).await;
    send(c, walk_to(bowmen.x, bowmen.y)).await;
    send(c, skill_at_reactor(bowmen.object_id, SIGIL_SKILLS[1])).await;
    until(c, &mut mt, |t| {
        t.field.reactor_hits.contains(&(bowmen.object_id, 1))
            && t.item_lines.contains(&line(CWK_SIGIL_LINES[1]))
    })
    .await;
    assert!(
        !(mt.field.reactor_hits.iter()).any(|&(oid, _)| oid == warriors.object_id),
        "{mt:?}"
    );

    // From too far off, the leader's skill does nothing; by each other
    // sigil, it lights it, the warriors' last.
    let c = &mut leader.connection;
    send(c, walk_to(bowmen.x, bowmen.y)).await;
    send(c, skill_at_reactor(warriors.object_id, SIGIL_SKILLS[0])).await;
    for job in [2, 3, 4, 0] {
        let sigil = sigils[job];
        if job == 0 {
            let mut hits = lt.field.reactor_hits.iter();
            assert!(!hits.any(|&(oid, _)| oid == sigil.object_id), "{lt:?}");
        }
        send(c, walk_to(sigil.x, sigil.y)).await;
        send(c, skill_at_reactor(sigil.object_id, SIGIL_SKILLS[job])).await;
        until(c, &mut lt, |t| {
            t.field.reactor_hits.contains(&(sigil.object_id, 1))
        })
        .await;
    }

    // With the fifth, everyone hears the way on is open, and the stage
    // clears with its EXP and mesos.
    let (exp, meso) = (CWK.stage_exp(2), CWK.stage_meso(2));
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            CWK_SIGIL_LINES
                .iter()
                .all(|&key| t.item_lines.contains(&line(key)))
                && t.item_lines.contains(&line(keys::CWK_SIGILS_DONE))
                && cleared(t)
                && t.gains.contains(&exp)
                && t.meso_gains.contains(&meso)
        })
        .await;
    }
    let done = line(keys::CWK_SIGILS_DONE);
    assert_eq!(lt.item_lines.iter().filter(|&l| *l == done).count(), 1);

    // And the way on leads to the third stage.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, STAGE2, WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&STAGE3) && all_sigils(t)
    })
    .await;
    let sigils: Vec<_> = (0..5).filter_map(|job| sigil(&lt, job)).collect();
    let mut mt = Trace::default();
    touch_script_portal(&mut member, STAGE2, WAY_ON).await;
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&STAGE3)
    })
    .await;

    // Whose way on is shut.
    touch_script_portal(&mut leader, STAGE3, STAGE3_WAY_ON).await;
    hears(&mut leader, &mut lt, keys::CWK_WAY_SHUT).await;
    assert_eq!(lt.warps, [STAGE3], "{lt:?}");

    // Each trip up the way up takes the leader back down, and everyone
    // hears how many more are to go; past five, that not all the sigils
    // are lit.
    for n in (0..5).rev() {
        touch_script_portal(&mut leader, STAGE3, WAY_UP).await;
        until(&mut leader.connection, &mut lt, |t| {
            t.item_lines.contains(&to_go(n))
        })
        .await;
    }
    touch_script_portal(&mut leader, STAGE3, WAY_UP).await;
    let not_all = line(keys::CWK_STEPS_NOT_ALL);
    until(&mut leader.connection, &mut lt, |t| {
        t.item_lines.contains(&not_all)
    })
    .await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.iter().filter(|&&map| map == STAGE3).count() == 7
    })
    .await;
    until(&mut member.connection, &mut mt, |t| {
        (0..5).all(|n| t.item_lines.contains(&to_go(n))) && t.item_lines.contains(&not_all)
    })
    .await;

    // Each sigil lit sets its menhirs going; with the fifth, the stage
    // clears with its EXP and mesos.
    let c = &mut leader.connection;
    for (job, sigil) in sigils.iter().enumerate() {
        send(c, walk_to(sigil.x, sigil.y)).await;
        send(c, skill_at_reactor(sigil.object_id, SIGIL_SKILLS[job])).await;
    }
    let (exp, meso) = (CWK.stage_exp(3), CWK.stage_meso(3));
    let menhirs: Vec<_> = (0..5)
        .flat_map(cwk_menhirs)
        .map(|&name| (name.to_string(), 1))
        .collect();
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            CWK_MENHIR_LINES
                .iter()
                .all(|&key| t.item_lines.contains(&line(key)))
                && menhirs
                    .iter()
                    .all(|menhir| t.field.obstacles.contains(menhir))
                && t.item_lines.contains(&line(keys::CWK_SIGILS_DONE))
                && cleared(t)
                && t.gains.contains(&exp)
                && t.meso_gains.contains(&meso)
        })
        .await;
    }

    // Now a trip up tells just the one who took it the way on is open:
    // the member hears it once, for their own trip after the leader's.
    touch_script_portal(&mut leader, STAGE3, WAY_UP).await;
    hears_item(&mut leader, &mut lt, keys::CWK_STEPS_OPEN).await;
    touch_script_portal(&mut member, STAGE3, WAY_UP).await;
    hears_item(&mut member, &mut mt, keys::CWK_STEPS_OPEN).await;
    let open = line(keys::CWK_STEPS_OPEN);
    let heard = mt.item_lines.iter().filter(|&l| *l == open).count();
    assert_eq!(heard, 1, "{mt:?}");

    // The way on leads to the fourth stage.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, STAGE3, STAGE3_WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&STAGE4) && woken(t).len() == 8
    })
    .await;

    // Where a sigil of each job and a fake have woken, at eight places.
    let woken = woken(&lt);
    let mut jobs: Vec<_> = (woken.iter())
        .map(|r| (r.reactor_id - CWK_LATER_SIGILS).clamp(-1, 5))
        .collect();
    jobs.sort();
    jobs.dedup();
    assert_eq!(jobs, [-1, 0, 1, 2, 3, 4], "{woken:?}");
    for (n, r) in woken.iter().enumerate() {
        let near = |o: &&MapReactor| (o.x - r.x).abs() < 10 && (o.y - r.y).abs() < 10;
        assert_eq!(woken[n + 1..].iter().filter(near).count(), 0, "{woken:?}");
    }

    // The woken sigils light as the second stage's do, and clear the
    // stage.
    let c = &mut leader.connection;
    for job in 0..5 {
        let mut sigils = woken.iter();
        let sigil = sigils
            .find(|r| r.reactor_id == CWK_LATER_SIGILS + job)
            .unwrap();
        send(c, walk_to(sigil.x, sigil.y)).await;
        send(
            c,
            skill_at_reactor(sigil.object_id, SIGIL_SKILLS[job as usize]),
        )
        .await;
    }
    let (exp, meso) = (CWK.stage_exp(4), CWK.stage_meso(4));
    until(c, &mut lt, |t| {
        CWK_SIGIL_LINES
            .iter()
            .all(|&key| t.item_lines.contains(&line(key)))
            && t.item_lines.contains(&line(keys::CWK_SIGILS_DONE))
            && cleared(t)
            && t.gains.contains(&exp)
            && t.meso_gains.contains(&meso)
    })
    .await;

    // The way on leads to the fifth stage's hub, with its altars.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, STAGE4, WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&HUB) && ALTARS.iter().all(|a| altar(t, a.0).is_some())
    })
    .await;
    let altars: Vec<_> = ALTARS.iter().map(|a| altar(&lt, a.0).unwrap()).collect();
    let mut mt = Trace::default();
    touch_script_portal(&mut member, STAGE3, STAGE3_WAY_ON).await;
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&STAGE4)
    })
    .await;
    touch_script_portal(&mut member, STAGE4, WAY_ON).await;
    until(&mut member.connection, &mut mt, |t| t.warps.contains(&HUB)).await;

    // Its keepers there tell of their rooms; its way on is shut.
    let t = told(&mut leader, talk(HUB, CWK_ROOMS[0].keeper)).await;
    assert!(said(&t, CWK_ROOMS[0].lore[0]), "{t:?}");
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, HUB, WAY_ON).await;
    hears(&mut leader, &mut lt, keys::CWK_WAY_SHUT).await;

    // The warriors' room is only for warriors; the thieves' takes the
    // leader in.
    touch_script_portal(&mut leader, HUB, "in01").await;
    hears(&mut leader, &mut lt, keys::CWK_ROOM_ONLY_WARRIOR).await;
    assert!(lt.warps.is_empty(), "{lt:?}");
    let thieves = &CWK_ROOMS[3];
    touch_script_portal(&mut leader, HUB, "in04").await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&thieves.map) && eyes(t).len() == 5
    })
    .await;
    let eyes = eyes(&lt);

    // Whose keeper wants its eyes broken first.
    let t = told(&mut leader, talk(thieves.map, thieves.keeper)).await;
    assert!(said(&t, keys::CWK_ROOM_EYES), "{t:?}");
    let c = &mut leader.connection;
    for eye in &eyes {
        send(c, walk_to(eye.x, eye.y)).await;
        send(c, hit_reactor(eye.object_id)).await;
    }
    until(c, &mut lt, |t| {
        (eyes.iter()).all(|eye| t.field.reactor_hits.contains(&(eye.object_id, 1)))
    })
    .await;

    // Then gives back the thieves' weapon, once.
    let t = told(&mut leader, talk(thieves.map, thieves.keeper)).await;
    assert!(said(&t, keys::CWK_ROOM_GIVEN), "{t:?}");
    let given = t
        .added
        .iter()
        .find(|a| (a.0, a.2) == (ETC as u8, thieves.weapon));
    let (_, slot, _) = *given.unwrap_or_else(|| panic!("{t:?}"));
    let t = told(&mut leader, talk(thieves.map, thieves.keeper)).await;
    assert!(said(&t, keys::CWK_ROOM_TAKEN), "{t:?}");

    // The bowmen's room takes the member in, to its five Master
    // Guardians, whom its keeper wants gone first.
    let bowmen = &CWK_ROOMS[1];
    touch_script_portal(&mut member, HUB, "in02").await;
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&bowmen.map) && t.field.alive(CWK_MASTER_GUARDIAN).len() == 5
    })
    .await;
    let t = told(&mut member, talk(bowmen.map, bowmen.keeper)).await;
    assert!(said(&t, keys::CWK_ROOM_MASTERS), "{t:?}");

    // Back on the hub, the leader lays each weapon on its altar; each is
    // restored, and with the fifth the stage clears.
    let mut lt = Trace::default();
    send(
        &mut leader.connection,
        build_change_map(-1, "out00").unwrap(),
    )
    .await;
    until(&mut leader.connection, &mut lt, |t| t.warps.contains(&HUB)).await;
    let c = &mut leader.connection;
    for (room, &(_, x, y)) in CWK_ROOMS.iter().zip(&ALTARS) {
        let from = match others.iter().position(|&weapon| weapon == room.weapon) {
            Some(n) => n as i16 + 1,
            None => slot,
        };
        send(c, walk_to(x, y)).await;
        send(c, throw_down(from, 1)).await;
    }
    let restored = line(keys::CWK_WEAPON_RESTORED);
    let (exp, meso) = (CWK.stage_exp(5), CWK.stage_meso(5));
    until(c, &mut lt, |t| {
        t.item_lines.iter().filter(|&l| *l == restored).count() == 5
            && t.item_lines.contains(&line(keys::CWK_WEAPONS_DONE))
            && cleared(t)
            && t.gains.contains(&exp)
            && t.meso_gains.contains(&meso)
    })
    .await;
    for altar in &altars {
        assert!(
            lt.field.reactor_hits.contains(&(altar.object_id, 1)),
            "{lt:?}"
        );
    }

    // Jack tells of the stage.
    let t = told(&mut leader, talk(HUB, CWK_JACK)).await;
    assert!(said(&t, keys::CWK_JACK_STAGE5_1), "{t:?}");

    // And the way on leads to the sixth stage, for both.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, HUB, WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&STAGE6)
    })
    .await;
    let mut mt = Trace::default();
    send(
        &mut member.connection,
        build_change_map(-1, "out00").unwrap(),
    )
    .await;
    until(&mut member.connection, &mut mt, |t| t.warps.contains(&HUB)).await;
    touch_script_portal(&mut member, HUB, WAY_ON).await;
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&STAGE6)
    })
    .await;

    // Its host talks only to the leader, whom it welcomes before bringing
    // out the escort.
    let t = told(&mut member, talk(STAGE6, CWK_HOST)).await;
    assert!(said(&t, keys::CWK_HOST_LEADER), "{t:?}");
    let t = told(&mut leader, talk(STAGE6, CWK_HOST)).await;
    assert!(said(&t, keys::CWK_HOST_WELCOME_1), "{t:?}");
    told(&mut leader, answer(0, 1, None)).await;
    // What the host brings out may reach the leader before the talk's
    // answer is done, so the leader goes on from that.
    lt = told(&mut leader, answer(0, 1, None)).await;
    assert!(said(&lt, keys::CWK_HOST_WELCOME_3), "{lt:?}");
    let escort = |t: &Trace| {
        (CWK_ESCORT.iter()).all(|&(mob_id, count)| t.field.alive(mob_id).len() == count)
    };
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        hears_item(player, t, keys::CWK_ESCORT_OUT).await;
        until(&mut player.connection, t, escort).await;
    }
    let t = told(&mut leader, talk(STAGE6, CWK_HOST)).await;
    assert!(said(&t, keys::CWK_HOST_ESCORTING), "{t:?}");

    // With the escort gone, it brings out the Twisted Masters.
    beat_all(&mut leader.connection, &mut lt).await;
    let t = told(&mut leader, talk(STAGE6, CWK_HOST)).await;
    assert!(said(&t, keys::CWK_HOST_ESCORT_BEATEN), "{t:?}");
    lt = told(&mut leader, answer(0, 1, None)).await;
    assert!(said(&lt, keys::CWK_HOST_MASTERS), "{lt:?}");
    let masters =
        |t: &Trace| (CWK_MASTERS.iter()).all(|&(mob_id, _)| t.field.alive(mob_id).len() == 1);
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        hears_item(player, t, keys::CWK_MASTERS_OUT).await;
        until(&mut player.connection, t, masters).await;
    }
    let t = told(&mut leader, talk(STAGE6, CWK_HOST)).await;
    assert!(said(&t, keys::CWK_HOST_MASTERS_ESCORTING), "{t:?}");

    // With them gone too, the way on opens, and the Keep is won, with the
    // stage's EXP and mesos for both.
    beat_all(&mut leader.connection, &mut lt).await;
    lt = told(&mut leader, talk(STAGE6, CWK_HOST)).await;
    assert!(said(&lt, keys::CWK_HOST_BEATEN), "{lt:?}");
    let (exp, meso) = (CWK.stage_exp(6), CWK.stage_meso(6));
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        hears(player, t, keys::CWK_BOSS_OPEN).await;
        until(&mut player.connection, t, |t| {
            cleared(t) && t.gains.contains(&exp) && t.meso_gains.contains(&meso)
        })
        .await;
    }
    for player in [&mut leader, &mut member] {
        let t = told(player, talk(STAGE6, CWK_HOST)).await;
        assert!(said(&t, keys::CWK_HOST_WELL_PLAYED), "{t:?}");
    }

    // Its way on leads to the rest room, where Jack tells of the armory,
    // and on to the treasure room, on a clock of a minute, which sees
    // them out when it runs out.
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        touch_script_portal(player, STAGE6, WAY_ON).await;
        until(&mut player.connection, t, |t| t.warps.contains(&REST)).await;
    }
    let t = told(&mut member, talk(REST, CWK_JACK)).await;
    assert!(said(&t, keys::CWK_JACK_REST), "{t:?}");
    // The clock is the team's, so it sees out the member too, wherever
    // they are by then.
    touch_script_portal(&mut member, REST, WAY_ON).await;
    lt = Trace::default();
    touch_script_portal(&mut leader, REST, WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&TREASURE) && t.countdowns.iter().any(|&clock| clock <= 1)
    })
    .await;
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        until(&mut player.connection, t, |t| t.warps.contains(&CWK_LOBBY)).await;
    }
}

/// Beat every mob `seen` sees alive, each hit once answered.
async fn beat_all(c: &mut MapleTestConnection, seen: &mut Trace) {
    let alive = |seen: &Trace| {
        let mobs = seen.field.mobs.iter();
        let mut alive = mobs.filter(|(oid, _)| !seen.field.killed.contains(oid));
        alive.next().map(|&(oid, _)| oid)
    };
    while let Some(object_id) = alive(seen) {
        for _ in 0..60 {
            if seen.field.killed.contains(&object_id) {
                break;
            }
            let bars = seen.field.hp_bars.len();
            send(c, attack(object_id, HIT)).await;
            until(c, seen, |t| {
                t.field.hp_bars.len() > bars || t.field.killed.contains(&object_id)
            })
            .await;
        }
        assert!(seen.field.killed.contains(&object_id), "{seen:?}");
    }
}

/// The fifth stage's altar `id`, as `t` saw it.
fn altar(t: &Trace, id: i32) -> Option<MapReactor> {
    let mut reactors = t.field.reactors.iter();
    reactors.find(|r| r.reactor_id == id).copied()
}

/// The thieves' room's eyes that `t` saw.
fn eyes(t: &Trace) -> Vec<MapReactor> {
    let reactors = t.field.reactors.iter();
    let eyes = reactors.filter(|r| r.reactor_id == CWK_EYE);
    eyes.copied().collect()
}

/// `player` hears `key` in the chat log, as of an item.
async fn hears_item(player: &mut WorldSession, t: &mut Trace, key: &str) {
    until(&mut player.connection, t, |t| {
        t.item_lines.contains(&line(key))
    })
    .await;
}

/// The fourth stage's sigils and fakes that `t` saw woken.
fn woken(t: &Trace) -> Vec<MapReactor> {
    let later = CWK_LATER_SIGILS..CWK_LATER_SIGILS + 5;
    let reactors = t.field.reactors.iter();
    let woken = reactors.filter(|r| {
        r.state == 1 && (later.contains(&r.reactor_id) || FAKES.contains(&r.reactor_id))
    });
    woken.copied().collect()
}
