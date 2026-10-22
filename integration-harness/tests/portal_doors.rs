//! Portal scripts of a004's section 3: halls anyone may walk into
//! (`enterAchter`, `enterMagiclibrar`, `balogTemple`), doors that keep
//! players back with a pink line until they hold a pass or have done a
//! quest (`ariant_castle`, `enterWitch`, `secretDoor`, `curseforest`),
//! Rien's way out (`rienTutor8`), and the Temple of
//! Time's lanes, which send a player who isn't ready back to where the lane
//! began (`timeQuest`).

use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Go to `map` and try its portal `portal`; returns what came back.
async fn try_portal(player: &mut WorldSession, map: i32, portal: &str) -> Trace {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    step(&mut player.connection, &name, enter(portal)).await
}

/// Where the portal led, after its sound.
fn arrived(trace: &Trace) -> (i32, u8) {
    assert_eq!(trace.portal_sounds, 1, "{trace:?}");
    let [(map, portal, _)] = trace.arrivals[..] else {
        panic!("{:?}", trace.arrivals);
    };
    (map, portal)
}

fn pink(trace: &Trace) -> bool {
    matches!(trace.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty())
}

#[tokio::test]
async fn halls_open_and_gated_doors_stay_shut_to_a_new_character() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();

    for (map, portal, to) in [
        (100000200, "in02", (100000201, 9)),
        (101000000, "jobin00", (101000003, 8)),
        (105090200, "under00", (105100000, 2)),
        // Rien holds back only an Aran in training.
        (140000000, "west00", (140010000, 2)),
    ] {
        assert_eq!(arrived(&try_portal(&mut player, map, portal).await), to);
    }

    for (map, portal) in [
        (260000300, "in00"),
        (240040510, "in00"),
        (261010000, "secret00"),
        // Off Arwen's quests, at any hour.
        (100040105, "secret00"),
    ] {
        let trace = try_portal(&mut player, map, portal).await;
        assert!(trace.warps.is_empty(), "{map}: {:?}", trace.warps);
        assert!(pink(&trace), "{map}: {:?}", trace.broadcasts);
    }

    // Memory Lane 1's quest isn't done: back to the lane's start.
    let trace = try_portal(&mut player, 270010100, "in00").await;
    assert!(pink(&trace), "{:?}", trace.broadcasts);
    assert_eq!(arrived(&trace), (270010000, 3));
}
