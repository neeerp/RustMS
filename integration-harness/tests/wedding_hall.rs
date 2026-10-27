//! The Mushroom Castle's Wedding Hall: the key from King Pepe starts
//! Where's Violetta? (2332) through the quest alert, and the hall's door
//! (`TD_MC_enterboss2`) then takes the player alone into their own copy of
//! the hall for ten minutes, one at a time on a channel. Finding Violetta
//! there (`findvioleta`) lets 2332 end with her; The Story of Betrayal
//! (2333) brings out the Prime Minister. Once he is beaten, the door leads
//! straight into the hall, where Violetta shows her face (2334) and keeps
//! the Royal Seal for one who left it (2342), and gives the key whose door
//! completes Eliminating the Rest (2335). The harness world runs events
//! 60 times as fast (`RUSTMS_EVENT_SCALE`), so the ten minutes are ten
//! seconds.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_quest, set_str, set_weapon_attack,
};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{read_for, send, step, until, until_within, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::events::{
    PRIME_MINISTER, STORY_OF_BETRAYAL, WEDDING_HALL, WEDDING_HALL_DOOR, WEDDING_HALL_KEY,
    WHERES_VIOLETTA,
};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const VIOLETTA: i32 = 1_300_002;
const VIOLETTA_AT: (i16, i16) = (72, 87);
/// The hall's door: the portal `right00`, whose script is `TD_MC_enterboss2`.
const DOOR: &str = "right00";
const DOOR_AT: (i16, i16) = (255, 140);
const SEAL_OF_THE_EMPIRE: i16 = 2331;
const ROYAL_SEAL: i32 = 4_001_318;
const ETC: u8 = 4;
const ELIMINATING_THE_REST: i16 = 2335;
const SECURITY_ROOM_KEY: i32 = 4_032_405;
/// Skyscraper 3's door into the Security Room: `in00`, whose script is
/// `go_secretroom`.
const SKYSCRAPER_3: i32 = 106_021_000;
const SECURITY_DOOR: &str = "in00";
const SECURITY_DOOR_AT: (i16, i16) = (927, -97);
const SECURITY_ROOM: i32 = 106_021_001;

/// A level 35 fighter, armed for the Prime Minister, on The Seal of the
/// Empire, with quests `quests` (id, status) and the key, at the hall's
/// door.
async fn at_the_door(config: &HarnessConfig, quests: &[(i16, i16)]) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 35, 50).await.expect("a level 35 warrior");
    replace_item(id, -1, -11, 1302000).await.expect("a sword");
    set_weapon_attack(id, 500).await.expect("its attack");
    set_str(id, 999).await.expect("strength");
    set_quest(id, SEAL_OF_THE_EMPIRE, 1, "")
        .await
        .expect("2331 taken");
    for &(quest, status) in quests {
        set_quest(id, quest, status, "").await.expect("a quest");
    }
    grant_stack(id, ETC as i16, 1, WEDDING_HALL_KEY, 1)
        .await
        .expect("the key");
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, WEDDING_HALL_DOOR, "", &mut trace).await,
        WEDDING_HALL_DOOR
    );
    step(c, &config.character_name, walk_to(DOOR_AT.0, DOOR_AT.1)).await;
    session
}

/// Going through a scripted portal.
fn touch(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Run Violetta's script for `quest` (`action` 4 or 5), reading its pages
/// through and saying yes to any question; what came of it, with any mob
/// it brought out.
async fn with_violetta(c: &mut MapleTestConnection, m: &str, action: u8, quest: i16) -> Trace {
    let mut t = step(c, m, build_quest_action(action, quest, VIOLETTA)).await;
    let mut all = Trace::default();
    loop {
        all.progress.append(&mut t.progress);
        all.gains.append(&mut t.gains);
        all.added.append(&mut t.added);
        all.field.mobs.append(&mut t.field.mobs);
        let Some(&(npc, kind)) = t.prompts.first() else {
            return all;
        };
        assert_eq!(npc, VIOLETTA);
        all.prompts.append(&mut t.prompts);
        t = step(c, m, answer(kind, 1, None)).await;
    }
}

#[tokio::test]
async fn violetta_is_found_and_the_prime_minister_beaten() {
    let (config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let m = config.character_name.clone();
    let mut session = at_the_door(&config, &[]).await;
    let mut other = at_the_door(&other_config, &[(WHERES_VIOLETTA, 1)]).await;
    let c = &mut session.connection;

    // Locked before the key's quest, which the quest alert then starts.
    let t = step(c, &m, touch(DOOR)).await;
    assert!(t.warps.is_empty());
    assert_eq!(t.broadcasts.len(), 1, "told in pink: {:?}", t.broadcasts);
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, WHERES_VIOLETTA, VIOLETTA),
    )
    .await;
    assert!(t.prompts.is_empty(), "started without a word");
    assert_eq!(t.progress, [(WHERES_VIOLETTA, 1)]);

    // In, alone, and Violetta is found.
    send(c, touch(DOOR)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [WEDDING_HALL] && t.kills.contains_key(&WHERES_VIOLETTA)
    })
    .await;
    assert_eq!(trace.countdowns.len(), 1, "the ten minutes run");
    assert_eq!(trace.kills[&WHERES_VIOLETTA], "001");

    // Someone else must wait while the hall is taken.
    let o = &mut other.connection;
    send(o, touch(DOOR)).await;
    let mut t = Trace::default();
    read_for(o, &mut t, 800).await;
    assert!(t.warps.is_empty());
    assert_eq!(t.broadcasts.len(), 1, "told in pink: {:?}", t.broadcasts);

    step(c, &m, walk_to(VIOLETTA_AT.0, VIOLETTA_AT.1)).await;
    let t = with_violetta(c, &m, SCRIPT_END, WHERES_VIOLETTA).await;
    assert_eq!(t.progress, [(WHERES_VIOLETTA, 2)]);
    assert_eq!(t.gains, [4_400]);

    // She names the Prime Minister, and he comes out.
    let mut trace = with_violetta(c, &m, SCRIPT_START, STORY_OF_BETRAYAL).await;
    assert_eq!(trace.prompts.len(), 2, "a question, then his name");
    assert_eq!(trace.progress, [(STORY_OF_BETRAYAL, 1)]);
    until(c, &mut trace, |t| t.field.alive(PRIME_MINISTER).len() == 1).await;

    // The time runs out before he falls; the next run finds him waiting.
    until_within(c, &mut trace, Duration::from_secs(20), |t| {
        t.warps == [WEDDING_HALL_DOOR]
    })
    .await;
    step(c, &m, walk_to(DOOR_AT.0, DOOR_AT.1)).await;
    // The hall is held two seconds after a run ends.
    tokio::time::sleep(Duration::from_secs(3)).await;
    send(c, touch(DOOR)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [WEDDING_HALL] && t.field.alive(PRIME_MINISTER).len() == 1
    })
    .await;
    let prime_minister = trace.field.alive(PRIME_MINISTER)[0];
    for _ in 0..40 {
        send(c, attack(prime_minister, 30_000)).await;
        read_for(c, &mut trace, 100).await;
        if trace.field.killed.contains(&prime_minister) {
            break;
        }
    }
    until(c, &mut trace, |t| t.kills.contains_key(&STORY_OF_BETRAYAL)).await;
    assert_eq!(trace.kills[&STORY_OF_BETRAYAL], "001");
    step(c, &m, walk_to(VIOLETTA_AT.0, VIOLETTA_AT.1)).await;
    let t = with_violetta(c, &m, SCRIPT_END, STORY_OF_BETRAYAL).await;
    assert_eq!(t.progress, [(STORY_OF_BETRAYAL, 2)]);
    assert_eq!(t.gains, [15_000]);

    // The time runs out, and the door now leads straight into the hall,
    // where Violetta shows her face and has the seal left behind.
    let mut trace = Trace::default();
    until_within(c, &mut trace, Duration::from_secs(20), |t| {
        t.warps == [WEDDING_HALL_DOOR]
    })
    .await;
    step(c, &m, walk_to(DOOR_AT.0, DOOR_AT.1)).await;
    send(c, touch(DOOR)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [WEDDING_HALL]).await;
    assert!(trace.countdowns.is_empty(), "the hall itself, no copy");
    step(c, &m, walk_to(VIOLETTA_AT.0, VIOLETTA_AT.1)).await;
    let t = with_violetta(c, &m, SCRIPT_START, 2334).await;
    assert_eq!(t.progress, [(2334, 2)]);
    assert_eq!(t.gains, [1_000]);
    let t = with_violetta(c, &m, SCRIPT_START, 2342).await;
    assert_eq!(t.progress, [(2342, 2)]);
    assert_eq!(t.added.len(), 1);
    let (inventory, _, item) = t.added[0];
    assert_eq!((inventory, item), (ETC, ROYAL_SEAL));

    // Violetta gives the Security Room's key with Eliminating the Rest
    // (2335), and its door near Skyscraper 3 completes the quest.
    let t = with_violetta(c, &m, SCRIPT_START, ELIMINATING_THE_REST).await;
    assert_eq!(t.progress, [(ELIMINATING_THE_REST, 1)]);
    let (inventory, _, item) = t.added[0];
    assert_eq!((inventory, item), (ETC, SECURITY_ROOM_KEY));
    let mut trace = Trace::default();
    assert_eq!(warp(c, SKYSCRAPER_3, "", &mut trace).await, SKYSCRAPER_3);
    step(c, &m, walk_to(SECURITY_DOOR_AT.0, SECURITY_DOOR_AT.1)).await;
    send(c, touch(SECURITY_DOOR)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [SECURITY_ROOM]).await;
    read_for(c, &mut trace, 300).await;
    assert_eq!(trace.progress, [(ELIMINATING_THE_REST, 2)]);
    assert_eq!(trace.gains, [5_000]);
    assert_eq!(trace.items, [(SECURITY_ROOM_KEY, -1)]);
}
