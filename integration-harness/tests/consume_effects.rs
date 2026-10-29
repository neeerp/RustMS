//! An item buff's aura (SHOW_CONSUME_EFFECT, recv 166): the player and
//! the map are told when Horntail Squad Victory starts, those who arrive
//! later and the player on a new map are told again, and cancelling the
//! buff takes it away for everyone. A skill's buff that takes over all the
//! item buff's stats takes the aura away too. Pharaoh's Blessing draws
//! its aura through its buff (temporary stat 73) instead.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, grant_stack, set_gm_level};
use integration_harness::hunting::{cancel_item_effect, use_item, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;

const USE: i16 = 2;
const SQUAD_VICTORY: i32 = 2_022_108;
const ELLINIA: i32 = 101_000_000;
const HENESYS: i32 = 100_000_000;
/// The GM's Bless: attack, magic and both defenses among its stats, all
/// four of Squad Victory's.
const GM_BLESS: i32 = 9_101_003;
const PHARAOHS_BLESSING: i32 = 2_022_585;

/// Read until a packet with `opcode`, with no SHOW_CONSUME_EFFECT for `id`
/// meanwhile (the other tests' drinkers share the map), and return its
/// bytes.
async fn until(c: &mut MapleTestConnection, t: &mut Trace, id: i32, opcode: SendOpcode) -> Vec<u8> {
    let opcode = opcode as i16;
    loop {
        let p = read(c, t).await;
        if p.opcode() == SendOpcode::ShowConsumeEffect as i16 {
            assert_ne!(int(&p.bytes, 2), id, "no 166 for the blessed");
        }
        if p.opcode() == opcode {
            return p.bytes.clone();
        }
    }
}

fn long(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn int(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// Read until the next SHOW_CONSUME_EFFECT, and return its player and item.
async fn aura(c: &mut MapleTestConnection, t: &mut Trace) -> (i32, i32) {
    loop {
        let p = read(c, t).await;
        if p.opcode() == SendOpcode::ShowConsumeEffect as i16 {
            let int = |at: usize| i32::from_le_bytes(p.bytes[at..at + 4].try_into().unwrap());
            return (int(2), int(6));
        }
    }
}

#[tokio::test]
async fn an_item_buff_shows_its_aura_to_the_map() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("drinker login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, SQUAD_VICTORY, 1)
        .await
        .expect("grant the potion");
    let mut drinker = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (d, mut dt) = (&mut drinker.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(warp(d, ELLINIA, "", &mut dt).await, ELLINIA);

    // The drinker's own client is told as well as the map's.
    send(d, use_item(1, SQUAD_VICTORY)).await;
    assert_eq!(aura(d, &mut dt).await, (id, SQUAD_VICTORY));
    assert_eq!(aura(w, &mut wt).await, (id, SQUAD_VICTORY));

    // SPAWN_PLAYER doesn't carry it: someone arriving is told after it,
    // and so is the drinker on their next map.
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(aura(w, &mut wt).await, (id, SQUAD_VICTORY));
    assert_eq!(warp(d, HENESYS, "", &mut dt).await, HENESYS);
    assert_eq!(aura(d, &mut dt).await, (id, SQUAD_VICTORY));
    assert_eq!(warp(d, ELLINIA, "", &mut dt).await, ELLINIA);
    assert_eq!(aura(d, &mut dt).await, (id, SQUAD_VICTORY));
    assert_eq!(aura(w, &mut wt).await, (id, SQUAD_VICTORY));

    // Cancelling the buff takes it away.
    send(d, cancel_item_effect(-SQUAD_VICTORY)).await;
    assert_eq!(aura(d, &mut dt).await, (id, 0));
    assert_eq!(aura(w, &mut wt).await, (id, 0));
}

#[tokio::test]
async fn a_skill_taking_over_the_item_buffs_stats_takes_its_aura() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("drinker login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");
    grant_skill(id, GM_BLESS, 1).await.expect("teach Bless");
    grant_stack(id, USE, 1, SQUAD_VICTORY, 1)
        .await
        .expect("grant the potion");
    let mut drinker = relogin(&config).await;
    let (d, mut dt) = (&mut drinker.connection, Trace::default());
    assert_eq!(warp(d, ELLINIA, "", &mut dt).await, ELLINIA);

    send(d, use_item(1, SQUAD_VICTORY)).await;
    assert_eq!(aura(d, &mut dt).await, (id, SQUAD_VICTORY));
    send(d, use_skill(GM_BLESS, 1)).await;
    assert_eq!(aura(d, &mut dt).await, (id, 0));
}

#[tokio::test]
async fn pharaohs_blessing_shows_its_aura_through_its_buff() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("drinker login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, PHARAOHS_BLESSING, 1)
        .await
        .expect("grant the blessing");
    let mut drinker = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (d, mut dt) = (&mut drinker.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(warp(d, ELLINIA, "", &mut dt).await, ELLINIA);

    // The drinker gets Booster (11) and the aura (73), the map the aura
    // with the item as its reason; nobody gets a SHOW_CONSUME_EFFECT.
    send(d, use_item(1, PHARAOHS_BLESSING)).await;
    let given = until(d, &mut dt, id, SendOpcode::GiveBuff).await;
    assert_eq!((long(&given, 2), long(&given, 10)), (1 << 41, 1 << 43));
    assert_eq!(
        (int(&given, 20), int(&given, 30)),
        (-PHARAOHS_BLESSING, -PHARAOHS_BLESSING)
    );
    let shown = until(w, &mut wt, id, SendOpcode::GiveForeignBuff).await;
    assert_eq!(int(&shown, 2), id);
    assert_eq!((long(&shown, 6), long(&shown, 14)), (1 << 41, 0));
    assert_eq!(int(&shown, 22), -PHARAOHS_BLESSING);

    // Those arriving see it in SPAWN_PLAYER, and cancelling takes it away.
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    send(d, cancel_item_effect(-PHARAOHS_BLESSING)).await;
    let cancelled = until(w, &mut wt, id, SendOpcode::CancelForeignBuff).await;
    assert_eq!(int(&cancelled, 2), id);
    assert_eq!(long(&cancelled, 6), 1 << 41);
}
