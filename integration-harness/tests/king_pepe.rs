//! Stop the Wedding (2330) and King Pepe's room: while the quest still wants
//! a Yeti brother, the East Castle Tower's `TD_MC_enterboss1` takes the
//! player alone into their own copy of the room for twenty minutes, where
//! one of the three brothers rides out with King Pepe. One player at a time
//! may be in it on a channel. Its way out (`out_pepeking`) leads back up
//! the tower, with the Wedding Hall key once all three are beaten. The
//! harness world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so
//! the twenty minutes are twenty seconds.

use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, step, until, until_within, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::events::{
    EAST_CASTLE_TOWER, KING_PEPE_ROOM, STOP_THE_WEDDING, WEDDING_HALL_KEY, YETI_BROTHERS,
};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const WEDDING_HALL_ENTRANCE: i32 = 106_021_401;
const ETC: u8 = 4;

/// A level 35 fighter, armed for the brothers, on Stop the Wedding with
/// `progress` (three kill counts), standing on `map_id` at `(x, y)`.
async fn on_quest(
    config: &HarnessConfig,
    progress: &str,
    map_id: i32,
    (x, y): (i16, i16),
) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 35, 50).await.expect("a level 35 warrior");
    replace_item(id, -1, -11, 1302000).await.expect("a sword");
    set_weapon_attack(id, 500).await.expect("its attack");
    set_str(id, 999).await.expect("strength");
    set_quest(id, STOP_THE_WEDDING, 1, progress)
        .await
        .expect("2330 taken");
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, map_id, "", &mut trace).await, map_id);
    step(c, &config.character_name, walk_to(x, y)).await;
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

const TOWER_DOOR: (i16, i16) = (268, -699);
const WAY_OUT: (i16, i16) = (-487, -66);

#[tokio::test]
async fn a_yeti_brother_waits_in_king_pepes_room() {
    let (config, other_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let none = "000000000";
    let mut session = on_quest(&config, none, EAST_CASTLE_TOWER, TOWER_DOOR).await;
    let mut other = on_quest(&other_config, none, EAST_CASTLE_TOWER, TOWER_DOOR).await;
    let c = &mut session.connection;

    // In, alone, with one of the brothers.
    send(c, touch("TD_MC_enterboss1")).await;
    let mut trace = Trace::default();
    read_for(c, &mut trace, 1_500).await;
    assert_eq!(trace.warps, [KING_PEPE_ROOM]);
    assert_eq!(trace.countdowns.len(), 1, "the twenty minutes run");
    until(c, &mut trace, |t| {
        YETI_BROTHERS
            .iter()
            .map(|&id| t.field.alive(id).len())
            .sum::<usize>()
            == 1
    })
    .await;
    let (index, brother) = YETI_BROTHERS
        .iter()
        .enumerate()
        .find_map(|(i, &id)| Some((i, *trace.field.alive(id).first()?)))
        .unwrap();

    // Someone else on the quest must wait while the room is taken.
    let o = &mut other.connection;
    send(o, touch("TD_MC_enterboss1")).await;
    let mut t = Trace::default();
    read_for(o, &mut t, 1_500).await;
    assert!(t.warps.is_empty());
    assert_eq!(t.broadcasts.len(), 1, "told in pink: {:?}", t.broadcasts);

    // The brother falls, and counts for 2330.
    for _ in 0..40 {
        send(c, attack(brother, 30_000)).await;
        read_for(c, &mut trace, 100).await;
        if trace.field.killed.contains(&brother) {
            break;
        }
    }
    until(c, &mut trace, |t| t.kills.contains_key(&STOP_THE_WEDDING)).await;
    let mut kills = ["000"; 3];
    kills[index] = "001";
    assert_eq!(trace.kills[&STOP_THE_WEDDING], kills.concat());

    // Out and up the tower, with no key for one brother.
    step(c, &config.character_name, walk_to(WAY_OUT.0, WAY_OUT.1)).await;
    send(c, touch("out01")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [EAST_CASTLE_TOWER]).await;
    assert!(trace.added.is_empty());
    drop(session);

    // The other goes in at last (the room is held two seconds after its
    // player leaves), and the time running out sends them back.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let o = &mut other.connection;
    send(o, touch("TD_MC_enterboss1")).await;
    let mut t = Trace::default();
    until(o, &mut t, |t| t.warps == [KING_PEPE_ROOM]).await;
    let mut t = Trace::default();
    until_within(o, &mut t, Duration::from_secs(30), |t| {
        t.warps == [EAST_CASTLE_TOWER]
    })
    .await;
}

#[tokio::test]
async fn king_pepe_drops_the_wedding_hall_key_once_all_three_are_beaten() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    // Straight onto the room's own map, out of any event.
    let mut session = on_quest(&config, "001001001", KING_PEPE_ROOM, WAY_OUT).await;
    let c = &mut session.connection;
    send(c, touch("out01")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [EAST_CASTLE_TOWER]).await;
    assert_eq!(trace.added.len(), 1);
    let (inventory, _, item) = trace.added[0];
    assert_eq!((inventory, item), (ETC, WEDDING_HALL_KEY));

    // With all three beaten, the tower's door leads on to the Wedding Hall.
    step(c, &m, walk_to(TOWER_DOOR.0, TOWER_DOOR.1)).await;
    send(c, touch("TD_MC_enterboss1")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [WEDDING_HALL_ENTRANCE]).await;
}
