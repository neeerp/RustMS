use std::collections::BTreeMap;

/// One key's binding. `kind` and `action` use the client's numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub kind: u8,
    pub action: i32,
}

/// Number of keys the v83 keymap packet describes.
pub const KEY_COUNT: i16 = 90;

/// Default bindings for new characters: (key, kind, action).
const DEFAULT_BINDINGS: [(i16, u8, i32); 25] = [
    (59, 6, 100),
    (60, 6, 101),
    (61, 6, 102),
    (62, 6, 103),
    (63, 6, 104),
    (64, 6, 105),
    (65, 6, 106),
    (56, 5, 54),
    (87, 5, 54),
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
    (21, 4, 12),
    (4, 4, 14),
    (84, 4, 15),
    // Ctrl attacks and Z picks up (HeavenClient's ATTACK 53 and PICKUP 51).
    (29, 5, 53),
    (44, 5, 51),
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
        assert_eq!(keymap.iter().count(), 24);
    }
}
