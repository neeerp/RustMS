//! Quick slot keys (report 240): the eight keys the status bar's quick slot
//! panel shows, left to right and top to bottom. Each is a key map index
//! (a scan code). The client edits them in Key Config and sends all eight.

/// v83's layout when none is set (table `0xbdafac`): Shift, Insert, Home,
/// Page Up over Ctrl, Delete, End, Page Down.
pub const DEFAULT: [u8; 8] = [42, 82, 71, 73, 29, 83, 79, 81];

/// The keys a quick slot may take: those with a label under
/// `UIWindow.img/KeyConfig/quickslotConfig/key`, which the editor checks.
/// Esc, Backspace, Tab, Enter, Caps Lock, the arrows and some numpad keys
/// have none.
pub const ALLOWED: [u8; 68] = [
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 29, 30,
    31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 56, 57,
    59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 71, 73, 79, 81, 82, 83, 87, 88,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuickSlots(pub [u8; 8]);

impl Default for QuickSlots {
    fn default() -> Self {
        Self(DEFAULT)
    }
}

impl QuickSlots {
    /// The keys from the client's QUICK_SLOT, if the editor could have made
    /// them: each allowed and no two alike.
    pub fn from_client(keys: [i32; 8]) -> Option<Self> {
        let mut slots = [0u8; 8];
        for (i, key) in keys.into_iter().enumerate() {
            let key = u8::try_from(key).ok().filter(|k| ALLOWED.contains(k))?;
            if slots[..i].contains(&key) {
                return None;
            }
            slots[i] = key;
        }
        Some(Self(slots))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_layout_is_allowed() {
        let keys = DEFAULT.map(i32::from);
        assert_eq!(QuickSlots::from_client(keys), Some(QuickSlots::default()));
    }

    #[test]
    fn a_layout_of_allowed_keys_is_kept_in_order() {
        let keys = [2, 3, 4, 5, 16, 17, 18, 88];
        assert_eq!(
            QuickSlots::from_client(keys),
            Some(QuickSlots([2, 3, 4, 5, 16, 17, 18, 88]))
        );
    }

    #[test]
    fn keys_the_editor_refuses_are_refused() {
        // Esc, Enter, Right Shift, an arrow, past the key map and negative.
        for bad in [1, 28, 54, 75, 89, 256, -1] {
            let keys = [bad, 82, 71, 73, 29, 83, 79, 81];
            assert_eq!(QuickSlots::from_client(keys), None, "key {bad}");
        }
    }

    #[test]
    fn a_key_used_twice_is_refused() {
        assert_eq!(
            QuickSlots::from_client([42, 82, 71, 73, 29, 83, 79, 42]),
            None
        );
    }
}
