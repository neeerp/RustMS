//! Hired merchants (report 165, plan c007; `game::hired_merchant`): mini
//! room type 5, the personal shop's room with a merchant in the owner's
//! seat. Listing and buying follow the personal shop's rules
//! (`personal_shop`); these are the merchant's own.

use crate::items;

/// The mini room type of a hired merchant.
pub const ROOM_TYPE: u8 = 5;
/// The merchant and three visitors.
pub const ROOM_USERS: u8 = 4;
/// How many rows a merchant lists at most.
pub const MAX_ITEMS: u8 = 16;
/// Characters of this level and below can't hire one (FUN_00a27b7d).
pub const MAX_LEVEL_REFUSED: u8 = 15;
/// The longest title the client lets one type (0xa1dee7).
pub const MAX_TITLE: usize = 50;

/// Whether `item_id` is a hired merchant permit (503xxxx) that exists.
/// The client takes any 503 item for one, but there is no 5030007.
pub fn is_permit(item_id: i32) -> bool {
    item_id / 10_000 == 503 && items::info(item_id).is_some()
}

/// Whether `item_id` is a Store Remote Controller (547xxxx) that exists:
/// its owner manages their merchant from anywhere on its channel.
pub fn is_remote_controller(item_id: i32) -> bool {
    item_id / 10_000 == 547 && items::info(item_id).is_some()
}

/// The merchant and balloon a permit gives (`Item/Cash/0503.img/050300NN`).
pub fn skin(item_id: i32) -> u8 {
    (item_id % 100) as u8
}

/// The counts in a merchant's balloon. The client draws it "Able" while
/// the first is under the second less one (0x48c788), so an open merchant
/// counts its visitors out of four and shows full at three; one in
/// maintenance shows 0 of 1, which is never able.
pub fn balloon_counts(open: bool, visitors: u8) -> (u8, u8) {
    if open {
        (visitors, ROOM_USERS)
    } else {
        (0, 1)
    }
}

/// What a hired merchant's owner or visitor asks: a permit's use
/// (ENTRUSTED_SHOP_REQUEST) or PLAYER_INTERACTION in a type 5 room, whose
/// modes are the personal shop's plus 0xb. Opening (0xb) and the blacklist
/// sent just before (0x1e) are the same for both kinds of room, so those
/// come as `PersonalShopRequest`; the runtime knows which room it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HiredMerchantRequest {
    /// A permit was used: may the player hire a merchant here?
    Check,
    Create {
        title: String,
        permit_slot: i16,
        permit_id: i32,
    },
    PutItem {
        kind: i16,
        slot: i16,
        bundles: i16,
        per_bundle: i16,
        price: i32,
    },
    Buy {
        index: u8,
        bundles: i16,
    },
    TakeBack {
        index: i16,
    },
    /// The owner is done with maintenance; the merchant opens again.
    EndMaintenance,
    /// Strike the sold-out rows and take the mesos.
    Organize,
    /// Close the merchant, taking back what it holds.
    Close,
    TakeMeso,
    /// Who visited, and the blacklist, for the owner's dialogs.
    Visitors,
    Blacklist,
    BlacklistAdd(String),
    BlacklistRemove(String),
    /// The Store Remote Controller in cash `slot` was used (op 0x3B): where
    /// is the player's merchant?
    Remote {
        slot: i16,
    },
    /// The owner said yes to managing their merchant, room `sn`, from
    /// afar, with the controller in cash `slot` whose serial is `cash_sn`
    /// (mode 0xe around a visit).
    RemoteVisit {
        sn: i32,
        slot: i16,
        cash_sn: i64,
    },
}

/// The answers to a permit's use (ENTRUSTED_SHOP_CHECK_RESULT).
pub mod check_result {
    /// The client asks for a title and opens the room.
    pub const OK: u8 = 7;
    /// "Your store is currently open in Channel %s, Free Market %d. ..."
    pub const OPEN_ELSEWHERE: u8 = 8;
    /// "Another character is currently using the item. ..."
    pub const IN_USE: u8 = 0x0a;
    /// "You are currently unable to open the store."
    pub const UNAVAILABLE: u8 = 0x0b;
    /// "Please retrieve your items from Fredrick."
    pub const AT_FREDRICK: u8 = 0x0f;
    /// The Remote Controller's: the merchant is open on another channel,
    /// or, with channel `NO_CHANNEL`, nowhere.
    pub const REMOTE_ELSEWHERE: u8 = 0x10;
    /// The Remote Controller's: the merchant is open on this channel.
    pub const REMOTE_HERE: u8 = 0x11;
    /// The channel of `REMOTE_ELSEWHERE` when no merchant is open
    /// ("Unable to use this due to the Remote Shop not being open.").
    pub const NO_CHANNEL: u8 = 0xff;
}

/// What closing a merchant in its room brought back (mode 0x2a).
pub mod close_result {
    /// "Both the items and mesos are successfully retrieved."
    pub const RETRIEVED: u8 = 0;
    /// Too many mesos to carry; Fredrick keeps it all.
    pub const TOO_MUCH_MESO: u8 = 1;
    /// A one-of-a-kind item the owner has; only the mesos came back.
    pub const ONE_OF_A_KIND: u8 = 2;
    /// No room for the items; only the mesos came back.
    pub const INVENTORY_FULL: u8 = 3;
}

/// Why a merchant's room closed for the one in it (LEAVE, type 5).
pub mod leave_reason {
    /// "The owner of the store is currently undergoing store
    /// maintenance. ..."
    pub const MAINTENANCE: u8 = 0x11;
    /// "It is past the working hours, so the store will close."
    pub const CLOSED: u8 = 0x12;
    /// "The use of the Remote Control stopped due to moving to another
    /// map." (SP 0xd8d, report b088 §5).
    pub const REMOTE_MOVED: u8 = 0x13;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permits_are_the_503_items_that_exist() {
        if items::data().is_err() {
            return;
        }
        assert!(is_permit(5030000) && is_permit(5030012));
        assert!(!is_permit(5030007), "no such item");
        assert!(!is_permit(5140000) && !is_permit(2000000));
        assert_eq!((skin(5030000), skin(5030012)), (0, 12));
        assert!(is_remote_controller(5470000));
        assert!(!is_remote_controller(5030000) && !is_remote_controller(5479999));
    }

    #[test]
    fn the_balloon_shows_full_at_three_and_in_maintenance() {
        let able = |(cur, max): (u8, u8)| cur < max - 1;
        assert!((0..3).all(|visitors| able(balloon_counts(true, visitors))));
        assert!(!able(balloon_counts(true, 3)));
        assert!(!able(balloon_counts(false, 0)));
    }
}
