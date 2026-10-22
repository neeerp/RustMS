//! Equipment scrolls (USE_UPGRADE_SCROLL): a 100% one-handed sword scroll
//! on the starting sword, with a White Scroll and without; a helmet scroll
//! that doesn't fit; the scroll's glow for the map; the upgrade surviving a
//! relogin; and Legendary Spirit's dialog scrolling a sword in the Equip tab.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    equip_upgrades, grant_item, grant_skill, grant_stack, set_weapon_attack, stow_equip,
};
use integration_harness::packets::{build_all_chat, decode_item_stack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use tokio::time::Duration;

const USE: i16 = 2;
const WEAPON: i16 = -11;
const SWORD: i32 = 1_302_000;
/// Scroll for One-Handed Sword for ATT 100%: weapon attack +1.
const SWORD_SCROLL: i32 = 2_043_000;
/// Scroll for Helmet for DEF 100%: not for swords.
const HELMET_SCROLL: i32 = 2_040_000;
const WHITE_SCROLL: i32 = 2_340_000;
const BASE_WATK: i16 = 17;

fn use_scroll(scroll_slot: i16, white: bool) -> Packet {
    scroll_request(scroll_slot, WEAPON, white, false)
}

fn scroll_request(scroll_slot: i16, equip_slot: i16, white: bool, legendary: bool) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseUpgradeScroll as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(scroll_slot).unwrap();
    p.write_short(equip_slot).unwrap();
    p.write_short(if white { 2 } else { 1 }).unwrap();
    p.write_byte(u8::from(legendary)).unwrap();
    p
}

/// Send `packet`, then a chat line, and return what the server sent before
/// the line came back.
async fn answers(c: &mut MapleTestConnection, marker: &str, packet: Packet) -> Vec<Packet> {
    send(c, packet).await;
    send(c, build_all_chat(marker, 0).unwrap()).await;
    let mut trace = Trace::default();
    let mut packets = Vec::new();
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() == SendOpcode::ChatText as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            if r.read_str_with_length().unwrap() == marker {
                return packets;
            }
        }
        packets.push(p);
    }
}

#[derive(Debug, PartialEq)]
enum Change {
    /// (type, slot, quantity)
    Quantity(u8, i16, i16),
    /// (type, slot)
    Remove(u8, i16),
    /// The sword added back at `slot`: (slot, upgrade slots, level, watk).
    Sword(i16, u8, u8, i16),
}

/// The changes of the one INVENTORY_OPERATION among `packets`, which must
/// answer the request, and its trailing look refresh byte.
fn inventory_operation(packets: &[Packet]) -> (Vec<Change>, Option<u8>) {
    let ops: Vec<_> = packets
        .iter()
        .filter(|p| p.opcode() == SendOpcode::InventoryOperation as i16)
        .collect();
    assert_eq!(ops.len(), 1, "one inventory operation expected");
    let mut r = Cursor::new(ops[0].bytes.as_slice());
    r.read_short().unwrap();
    assert_eq!(r.read_byte().unwrap(), 1, "it answers the request");
    let mut changes = Vec::new();
    for _ in 0..r.read_byte().unwrap() {
        let mode = r.read_byte().unwrap();
        let kind = r.read_byte().unwrap();
        let slot = r.read_short().unwrap();
        changes.push(match mode {
            0 => {
                // Type, id, cash flag and expiration, then the upgrade slots,
                // level, and STR, DEX, INT, LUK, HP, MP, weapon attack.
                let mut equip = r.clone();
                assert_eq!(decode_item_stack(&mut r).unwrap(), (SWORD, 1));
                equip.read_bytes(1 + 4 + 1 + 8).unwrap();
                let slots = equip.read_byte().unwrap();
                let level = equip.read_byte().unwrap();
                equip.read_bytes(6 * 2).unwrap();
                Change::Sword(slot, slots, level, equip.read_short().unwrap())
            }
            1 => Change::Quantity(kind, slot, r.read_short().unwrap()),
            3 => Change::Remove(kind, slot),
            other => panic!("unexpected change mode {other}"),
        });
    }
    (changes, r.read_byte().ok())
}

/// The bytes after the character id of `character_id`'s SHOW_SCROLL_EFFECT
/// among `packets` (other tests' characters may scroll on the same map).
fn scroll_effect(packets: &[Packet], character_id: i32) -> Option<Vec<u8>> {
    packets
        .iter()
        .find(|p| {
            p.opcode() == SendOpcode::ShowScrollEffect as i16
                && p.bytes[2..6] == character_id.to_le_bytes()
        })
        .map(|p| p.bytes[6..].to_vec())
}

/// What an observer sees of `character_id`'s next scroll: the effect bytes,
/// and then the look refresh.
async fn observed_scroll(c: &mut MapleTestConnection, character_id: i32) -> Vec<u8> {
    let mut trace = Trace::default();
    let effect = loop {
        let p = read(c, &mut trace).await;
        if let Some(effect) = scroll_effect(std::slice::from_ref(&p), character_id) {
            break effect;
        }
    };
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() == SendOpcode::UpdateCharLook as i16
            && p.bytes[2..6] == character_id.to_le_bytes()
        {
            return effect;
        }
    }
}

#[tokio::test]
async fn scrolls_upgrade_the_worn_sword_for_everyone_to_see() {
    let config = load_harness_config_or_fail().await;
    let observer_config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_weapon_attack(id, BASE_WATK).await.expect("set watk");
    grant_stack(id, USE, 1, SWORD_SCROLL, 2)
        .await
        .expect("grant sword scrolls");
    grant_item(id, USE, 2, HELMET_SCROLL)
        .await
        .expect("grant helmet scroll");
    grant_item(id, USE, 3, WHITE_SCROLL)
        .await
        .expect("grant white scroll");
    let mut session = relogin(&config).await;
    let mut observer = login_to_world_session(&observer_config).await.unwrap();
    assert_eq!(session.map_id, observer.map_id);
    assert_eq!(session.equipped[&WEAPON.abs()], SWORD);

    // With a White Scroll: both scrolls are taken and the sword replaced.
    let got = answers(&mut session.connection, "white", use_scroll(1, true)).await;
    assert_eq!(
        inventory_operation(&got),
        (
            vec![
                Change::Quantity(USE as u8, 1, 1),
                Change::Remove(USE as u8, 3),
                Change::Remove(1, WEAPON),
                Change::Sword(WEAPON, 6, 1, BASE_WATK + 1),
            ],
            Some(2)
        )
    );
    // Success, not cursed, no Legendary Spirit, a White Scroll.
    assert_eq!(scroll_effect(&got, id), Some(vec![1, 0, 0, 1]));
    assert_eq!(
        observed_scroll(&mut observer.connection, id).await,
        [1, 0, 0, 1]
    );

    // A helmet scroll doesn't fit a sword: the request is only unlocked.
    let got = answers(&mut session.connection, "helmet", use_scroll(2, false)).await;
    assert!(
        got.iter()
            .any(|p| p.opcode() == SendOpcode::StatChange as i16 && p.bytes.len() == 7),
        "unlocked"
    );
    assert!(got.iter().all(|p| {
        p.opcode() != SendOpcode::InventoryOperation as i16
            && p.opcode() != SendOpcode::ShowScrollEffect as i16
    }));
    drop(session);

    // The upgrade was saved: the next scroll builds on it.
    let mut session = relogin(&config).await;
    let got = answers(&mut session.connection, "plain", use_scroll(1, false)).await;
    assert_eq!(
        inventory_operation(&got),
        (
            vec![
                Change::Remove(USE as u8, 1),
                Change::Remove(1, WEAPON),
                Change::Sword(WEAPON, 5, 2, BASE_WATK + 2),
            ],
            Some(2)
        )
    );
    assert_eq!(scroll_effect(&got, id), Some(vec![1, 0, 0, 0]));
    // The refused helmet scroll showed the observer nothing.
    assert_eq!(
        observed_scroll(&mut observer.connection, id).await,
        [1, 0, 0, 0]
    );
}

#[tokio::test]
async fn legendary_spirit_scrolls_a_sword_in_the_equip_tab() {
    const LEGENDARY_SPIRIT: i32 = 1003;
    const TAB_SLOT: i16 = 3;
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    stow_equip(id, TAB_SLOT, SWORD, 7).await.expect("a sword");
    grant_stack(id, USE, 1, SWORD_SCROLL, 2)
        .await
        .expect("grant sword scrolls");
    let mut session = relogin(&config).await;

    // Without the skill, the dialog is told no.
    let request = || scroll_request(1, TAB_SLOT, false, true);
    let got = answers(&mut session.connection, "unskilled", request()).await;
    assert_eq!(scroll_effect(&got, id), Some(vec![0xFF, 0, 1, 0]));
    assert!(got
        .iter()
        .all(|p| p.opcode() != SendOpcode::InventoryOperation as i16));
    drop(session);

    grant_skill(id, LEGENDARY_SPIRIT, 1)
        .await
        .expect("grant Legendary Spirit");
    let mut session = relogin(&config).await;
    let got = answers(&mut session.connection, "legendary", request()).await;
    assert_eq!(
        inventory_operation(&got),
        (
            vec![
                Change::Quantity(USE as u8, 1, 1),
                Change::Remove(1, TAB_SLOT),
                Change::Sword(TAB_SLOT, 6, 1, 1),
            ],
            None
        )
    );
    assert_eq!(scroll_effect(&got, id), Some(vec![1, 0, 1, 0]));
    // The worn sword is not the dialog's.
    let got = answers(
        &mut session.connection,
        "worn",
        scroll_request(1, WEAPON, false, true),
    )
    .await;
    assert_eq!(scroll_effect(&got, id), Some(vec![0xFF, 0, 1, 0]));
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        equip_upgrades(id, TAB_SLOT).await.expect("read the sword"),
        Some((6, 1, 0))
    );
}
