use std::collections::BTreeMap;

/// One key's binding. `kind` and `action` use the client's numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub kind: u8,
    pub action: i32,
}

/// Number of keys the v83 keymap packet describes.
pub const KEY_COUNT: i16 = 90;

/// Default bindings for new characters: (key, kind, action). Keys are scan
/// codes and actions HeavenClient's (`IO/KeyAction.h`); the layout is v83's.
/// v83 also binds O to party search and F to family, which the client has
/// no actions for.
const DEFAULT_BINDINGS: [(i16, u8, i32); 39] = [
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
    // The number row chats: all, party, buddy, guild, alliance and spouse
    // (v83 numbers keys by scan code, so 1 is key 2).
    (2, 4, 13),
    (3, 4, 14),
    (4, 4, 19),
    (5, 4, 25),
    (6, 4, 30),
    (7, 4, 60),
    // Esc and [ open the shortcut menu (the client numbers Esc 84).
    (84, 4, 15),
    (26, 4, 15),
    (27, 4, 16),
    (34, 4, 18),
    (25, 4, 20),
    (38, 4, 21),
    (41, 4, 24),
    (39, 4, 28),
    // B opens the Monster Book (HeavenClient's MONSTERBOOK 59).
    (48, 4, 59),
    // Ctrl attacks, Z picks up and X sits (HeavenClient's ATTACK 53, PICKUP 51
    // and SIT 52).
    (29, 5, 53),
    (44, 5, 51),
    (45, 5, 52),
    // Space talks to the nearest NPC (HeavenClient's INTERACT_HARVEST 55).
    (57, 5, 55),
];

/// A character's key bindings. Unbound keys are simply absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keymap {
    bindings: BTreeMap<i16, KeyBinding>,
}

impl Keymap {
    pub fn defaults() -> Self {
        let mut keymap = Self::default();
        for (key, kind, action) in DEFAULT_BINDINGS {
            keymap.bind(key, KeyBinding { kind, action });
        }
        keymap
    }

    pub fn get(&self, key: i16) -> Option<KeyBinding> {
        self.bindings.get(&key).copied()
    }

    /// Bind a key; kind 0 means "unbound" in the client's numbering.
    pub fn bind(&mut self, key: i16, binding: KeyBinding) {
        if binding.kind == 0 {
            self.bindings.remove(&key);
        } else {
            self.bindings.insert(key, binding);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (i16, KeyBinding)> + '_ {
        self.bindings.iter().map(|(&key, &binding)| (key, binding))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_kind_zero_unbinds() {
        let mut keymap = Keymap::defaults();
        assert!(keymap.get(59).is_some());
        keymap.bind(59, KeyBinding { kind: 0, action: 0 });
        assert!(keymap.get(59).is_none());
        assert_eq!(keymap.iter().count(), 38);
    }
}
