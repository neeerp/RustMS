use crate::{error::NetworkError, packet::op::SendOpcode};
use game::keymap::KEY_COUNT;
use game::{AutoPotions, Keymap};
use packet::{io::write::PktWrite, Packet};

/// `KEYMAP` (0x14F). A v83 client drops it before the first SET_FIELD,
/// while its stage is still the login one (CLogin::OnPacket FUN_005f80ff).
pub fn build_keymap(keymap: &Keymap) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::KeyMap as i16)?;
    packet.write_byte(0)?;

    for key in 0..KEY_COUNT {
        let binding = keymap.get(key);
        packet.write_byte(binding.map_or(0, |b| b.kind))?;
        packet.write_int(binding.map_or(0, |b| b.action))?;
    }

    Ok(packet)
}

/// The potions the first pet gives from the HP and MP pouches' slots,
/// `AUTO_HP_POT` (0x150) and `AUTO_MP_POT` (0x151): the item id, or 0 for
/// none (v83 FUN_0058de2d, FUN_0058de53). Like the key map, they reach
/// only a field's stage, so they follow SET_FIELD.
pub fn build_auto_potions(potions: AutoPotions) -> Result<[Packet; 2], NetworkError> {
    let build = |op: SendOpcode, item_id: i32| -> Result<Packet, NetworkError> {
        let mut packet = Packet::new_empty();
        packet.write_short(op as i16)?;
        packet.write_int(item_id)?;
        Ok(packet)
    };
    Ok([
        build(SendOpcode::AutoHpPot, potions.hp)?,
        build(SendOpcode::AutoMpPot, potions.mp)?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::KeyBinding;

    fn expected_table(bound: &[(i16, u8, i32)]) -> Vec<(u8, i32)> {
        let mut table = vec![(0u8, 0i32); 90];
        for &(key, kind, action) in bound {
            table[key as usize] = (kind, action);
        }
        table
    }

    fn assert_keymap_packet(keymap: &Keymap, table: &[(u8, i32)]) {
        let packet = build_keymap(keymap).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::KeyMap as i16).byte(0);
        for &(kind, action) in table {
            expect.byte(kind).int(action);
        }
        expect.end();
    }

    #[test]
    fn auto_potions_are_the_hp_then_the_mp_item() {
        let [hp, mp] = build_auto_potions(AutoPotions { hp: 2000000, mp: 0 }).unwrap();
        Expect::new(&hp.bytes)
            .short(SendOpcode::AutoHpPot as i16)
            .int(2000000)
            .end();
        Expect::new(&mp.bytes)
            .short(SendOpcode::AutoMpPot as i16)
            .int(0)
            .end();
    }

    #[test]
    fn empty_keymap_is_ninety_zero_entries() {
        assert_keymap_packet(&Keymap::default(), &expected_table(&[]));
    }

    #[test]
    fn bindings_land_at_their_key_index() {
        let mut keymap = Keymap::default();
        keymap.bind(0, KeyBinding { kind: 1, action: 7 });
        keymap.bind(18, KeyBinding { kind: 4, action: 1 });
        keymap.bind(
            89,
            KeyBinding {
                kind: 5,
                action: 54,
            },
        );
        assert_keymap_packet(
            &keymap,
            &expected_table(&[(0, 1, 7), (18, 4, 1), (89, 5, 54)]),
        );
    }

    #[test]
    fn default_keymap_matches_the_default_bindings() {
        assert_keymap_packet(
            &Keymap::defaults(),
            &expected_table(&[
                (59, 6, 100),
                (60, 6, 101),
                (61, 6, 102),
                (62, 6, 103),
                (63, 6, 104),
                (64, 6, 105),
                (65, 6, 106),
                (56, 5, 54),
                (18, 4, 1),
                (23, 4, 2),
                (31, 4, 3),
                (37, 4, 4),
                (19, 4, 5),
                (17, 4, 6),
                (46, 4, 7),
                (50, 4, 8),
                (16, 4, 9),
                (43, 4, 10),
                (40, 4, 11),
                (35, 4, 12),
                (2, 4, 13),
                (3, 4, 14),
                (4, 4, 19),
                (5, 4, 25),
                (6, 4, 30),
                (7, 4, 60),
                (84, 4, 15),
                (26, 4, 15),
                (27, 4, 16),
                (34, 4, 18),
                (25, 4, 20),
                (38, 4, 21),
                (41, 4, 24),
                (39, 4, 28),
                (48, 4, 59),
                (29, 5, 53),
                (44, 5, 51),
                (45, 5, 52),
                (57, 5, 55),
            ]),
        );
    }
}
