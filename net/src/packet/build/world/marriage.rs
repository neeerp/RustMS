//! Engagement (report b052): MARRIAGE_REQUEST asks the partner, and
//! MARRIAGE_RESULT answers the proposer; any result first closes the
//! proposer's "Waiting for her response..." box.

use crate::codec::item::{encode_item, SlotPrefix};
use crate::{codec::stats::write_padded_name, error::NetworkError, packet::op::SendOpcode};
use game::inventory::InventoryType;
use game::marriage::{MarriageRecord, Refusal};
use game::ring::{CoupleRing, RingKind, RingLooks};
use game::wedding::Venue;
use game::Item;
use packet::{io::write::PktWrite, Packet};

/// The couple is engaged now; the client keeps the record.
const ENGAGED: u8 = 11;
/// The couple is married now; the client keeps the record.
const MARRIED: u8 = 12;
/// The engagement is broken; the client drops the record.
const BROKEN: u8 = 13;
/// The marriage is over; the client drops the record.
const DIVORCED: u8 = 14;
/// A wedding invitation opened.
const INVITATION: u8 = 15;
/// A line of the server's own.
const NOTICE: u8 = 36;

fn result(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MarriageResult as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// GW_MarriageRecord, 0x30 bytes. Both items are the couple's wedding
/// ring, which the client draws the player's own ring effect from.
pub fn write_marriage_record(
    packet: &mut Packet,
    record: &MarriageRecord,
) -> Result<(), NetworkError> {
    packet.write_int(record.id)?;
    packet.write_int(record.groom_id)?;
    packet.write_int(record.bride_id)?;
    packet.write_short(record.status)?;
    packet.write_int(record.ring_id())?;
    packet.write_int(record.ring_id())?;
    write_padded_name(packet, &record.groom_name)?;
    write_padded_name(packet, &record.bride_name)?;
    Ok(())
}

/// The ring blocks of a look, as SPAWN_PLAYER and UPDATE_CHAR_LOOK end
/// it: the crush ring, the friendship ring, then the marriage ring
/// (reports b059 and b061). The client pairs the effect only with real ids
/// on both sides.
pub fn write_ring_looks(packet: &mut Packet, rings: &RingLooks) -> Result<(), NetworkError> {
    for look in [rings.crush, rings.friendship] {
        packet.write_byte(u8::from(look.is_some()))?;
        if let Some(look) = look {
            packet.write_long(look.ring_id)?;
            packet.write_long(look.partner_ring_id)?;
            packet.write_int(look.item_id)?;
        }
    }
    packet.write_byte(u8::from(rings.marriage.is_some()))?;
    if let Some(ring) = rings.marriage {
        packet.write_int(ring.self_id)?;
        packet.write_int(ring.partner_id)?;
        packet.write_int(ring.item_id)?;
    }
    Ok(())
}

/// The character data's crush and friendship ring lists: GW_CoupleRecord,
/// 0x21 bytes, then GW_FriendRecord, which adds the item (report b061).
pub fn write_couple_rings(packet: &mut Packet, rings: &[CoupleRing]) -> Result<(), NetworkError> {
    for kind in [RingKind::Crush, RingKind::Friendship] {
        let of_kind: Vec<_> = rings
            .iter()
            .filter(|ring| game::ring::kind(ring.item_id) == Some(kind))
            .collect();
        packet.write_short(of_kind.len() as i16)?;
        for ring in of_kind {
            packet.write_int(ring.partner_id)?;
            write_padded_name(packet, &ring.partner_name)?;
            packet.write_long(ring.id)?;
            packet.write_long(ring.partner_ring_id)?;
            if kind == RingKind::Friendship {
                packet.write_int(ring.item_id)?;
            }
        }
    }
    Ok(())
}

/// NOTIFY_MARRIED_PARTNER_MAP_TRANSFER: the partner `partner_id` is on
/// `map_id` (report b057). The mini-map marks the partner by the id, the
/// world map their map unless it is 0; `(0, 0)` forgets both.
pub fn build_partner_map(map_id: i32, partner_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NotifyMarriedPartnerMapTransfer as i16)?;
    packet.write_int(map_id)?;
    packet.write_int(partner_id)?;
    Ok(packet)
}

/// MARRIAGE_REQUEST mode 0: `name` (id `proposer_id`) proposes; the client
/// asks Yes or No and answers with RING_ACTION mode 2.
pub fn build_proposal(name: &str, proposer_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MarriageRequest as i16)?;
    packet.write_byte(0)?;
    packet.write_str_with_length(name)?;
    packet.write_int(proposer_id)?;
    Ok(packet)
}

/// MARRIAGE_RESULT with the client's line for `refusal`.
pub fn build_marriage_refusal(refusal: Refusal) -> Result<Packet, NetworkError> {
    result(refusal as u8)
}

/// MARRIAGE_RESULT's notices about a booked wedding (report b066).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeddingNotice {
    /// 0x109e: the reservation was made.
    Reserved = 16,
    /// The reservation has been cancelled: the wedding fell through.
    Cancelled = 31,
    /// 0x109b: no breaking off once a wedding is booked.
    NoBreakingOff = 32,
    /// 0x109d: the invitation opened is not valid.
    InvalidInvitation = 34,
}

/// MARRIAGE_RESULT with the client's line for `notice`.
pub fn build_wedding_notice(notice: WeddingNotice) -> Result<Packet, NetworkError> {
    result(notice as u8)
}

/// MARRIAGE_RESULT 15: the invitation window for the wedding of `groom`
/// and `bride` at `venue`.
pub fn build_invitation(groom: &str, bride: &str, venue: Venue) -> Result<Packet, NetworkError> {
    let mut packet = result(INVITATION)?;
    packet.write_str_with_length(groom)?;
    packet.write_str_with_length(bride)?;
    packet.write_short(venue.window() as i16)?;
    Ok(packet)
}

/// MARRIAGE_RESULT 11: "You are now engaged.", with the couple's record.
pub fn build_engaged(record: &MarriageRecord) -> Result<Packet, NetworkError> {
    let mut packet = result(ENGAGED)?;
    write_marriage_record(&mut packet, record)?;
    Ok(packet)
}

/// MARRIAGE_RESULT 12: "You are now married!" (0x108c), with the
/// couple's record (report b071).
pub fn build_married(record: &MarriageRecord) -> Result<Packet, NetworkError> {
    let mut packet = result(MARRIED)?;
    write_marriage_record(&mut packet, record)?;
    Ok(packet)
}

/// MARRIAGE_RESULT 13, "Your engagement has been broken.", or for a married
/// couple 14, "You are no longer married."
pub fn build_couple_ended(record: &MarriageRecord) -> Result<Packet, NetworkError> {
    result(if record.married() { DIVORCED } else { BROKEN })
}

/// WEDDING_PROGRESS: the priest's line `step` to everyone at the altar,
/// for the couple `groom` and `bride`, who answer it with WEDDING_TALK
/// (report b075). Step 0 also starts the venue's music.
pub fn build_wedding_progress(step: u8, groom: i32, bride: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::WeddingProgress as i16)?;
    packet.write_byte(step)?;
    packet.write_int(groom)?;
    packet.write_int(bride)?;
    Ok(packet)
}

/// WEDDING_CEREMONY_END: the client closes the priest's line and shows the
/// hearts between the couple until the next ceremony. It reads nothing.
pub fn build_wedding_hearts() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::WeddingCeremonyEnd as i16)?;
    Ok(packet)
}

/// NOTIFY_MARRIAGE: "<Guild> `name` is now married. Please congratulate
/// them." in the chat log, or "<Family>" to a relative (report b079). The
/// client's text has no '>' after the group, so it goes with the name.
pub fn build_married_notice(family: bool, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NotifyMarriage as i16)?;
    packet.write_byte(u8::from(family))?;
    packet.write_str_with_length(&format!("> {name}"))?;
    Ok(packet)
}

/// MARRIAGE_RESULT 36: `text` in the client's OK box.
pub fn build_marriage_notice(text: &str) -> Result<Packet, NetworkError> {
    let mut packet = result(NOTICE)?;
    packet.write_byte(1)?;
    packet.write_str_with_length(text)?;
    Ok(packet)
}

/// MARRIAGE_REQUEST mode 9: the couple's wishlist input window, which the
/// client answers with RING_ACTION mode 9 (report b084). The client
/// ignores it over another dialog.
pub fn build_wishlist_input() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MarriageRequest as i16)?;
    packet.write_byte(9)?;
    Ok(packet)
}

/// WEDDING_GIFT_RESULT's modes (report b084). The client throws on an
/// answer it didn't ask for, so each request gets exactly one.
mod gift {
    /// Open the give window on a partner's wishlist.
    pub const REGISTRY: u8 = 9;
    /// Open the receive window on the player's gifts.
    pub const GIFTS: u8 = 10;
    /// The gift went: "You have sent a gift.", and the window closes.
    pub const SENT: u8 = 11;
    /// 0x107f, one present for each wishlist. The client stays waiting, so
    /// the window can only be closed.
    pub const ALREADY_GIVEN: u8 = 12;
    /// 0x10b0, "Failed to send the gift."
    pub const FAILED: u8 = 14;
    /// The gift was taken: the window shows the gifts left.
    pub const TAKEN: u8 = 15;
    /// The gift wasn't taken; the client says nothing.
    pub const NOT_TAKEN: u8 = 16;
}

fn gift_result(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::WeddingGiftResult as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

fn write_wishes(packet: &mut Packet, wishes: &[String]) -> Result<(), NetworkError> {
    let wishes = &wishes[..wishes.len().min(usize::from(u8::MAX))];
    packet.write_byte(wishes.len() as u8)?;
    for wish in wishes {
        packet.write_str_with_length(wish)?;
    }
    Ok(())
}

/// The bag an item is listed under in the gift windows.
pub fn gift_bag(item: &Item) -> Option<InventoryType> {
    InventoryType::for_item(item.item_id).filter(|&bag| bag != InventoryType::Equipped)
}

/// The gift windows' lists of `bags`: a mask of the bags whose lists
/// follow (bit `1 << (bag + 1)`), then for each, in order, a count and
/// its items without a slot. The client keeps its lists of other bags. A
/// list holds at most 255, as the client's take request numbers them
/// with a byte.
fn write_gifts<'a>(
    packet: &mut Packet,
    bags: &[InventoryType],
    gifts: impl Iterator<Item = &'a Item> + Clone,
) -> Result<(), NetworkError> {
    let mut bags = bags.to_vec();
    bags.sort_by_key(|&bag| bag as i8);
    bags.dedup();
    let mask = bags
        .iter()
        .fold(0i64, |mask, &bag| mask | 1 << (bag as i64 + 1));
    packet.write_long(mask)?;
    for bag in bags {
        let listed: Vec<&Item> = gifts
            .clone()
            .filter(|item| gift_bag(item) == Some(bag))
            .take(usize::from(u8::MAX))
            .collect();
        packet.write_byte(listed.len() as u8)?;
        for item in listed {
            encode_item(packet, SlotPrefix::None, item)?;
        }
    }
    Ok(())
}

/// WEDDING_GIFT_RESULT mode 9: the give window, on a partner's `wishes`.
pub fn build_registry(wishes: &[String]) -> Result<Packet, NetworkError> {
    let mut packet = gift_result(gift::REGISTRY)?;
    write_wishes(&mut packet, wishes)?;
    Ok(packet)
}

/// WEDDING_GIFT_RESULT mode 10: the receive window, on every bag's list
/// of the player's `gifts`, oldest first.
pub fn build_gifts<'a>(
    gifts: impl Iterator<Item = &'a Item> + Clone,
) -> Result<Packet, NetworkError> {
    let mut packet = gift_result(gift::GIFTS)?;
    write_gifts(&mut packet, &InventoryType::BAGS, gifts)?;
    Ok(packet)
}

/// WEDDING_GIFT_RESULT mode 11: `gift` went from the wishlist `wishes`;
/// the window lists it as given before it closes.
pub fn build_gift_sent(wishes: &[String], gift: &Item) -> Result<Packet, NetworkError> {
    let mut packet = gift_result(gift::SENT)?;
    write_wishes(&mut packet, wishes)?;
    let bags: Vec<InventoryType> = gift_bag(gift).into_iter().collect();
    write_gifts(&mut packet, &bags, std::iter::once(gift))?;
    Ok(packet)
}

/// WEDDING_GIFT_RESULT mode 12: the player has given this partner a gift.
pub fn build_gift_already_given() -> Result<Packet, NetworkError> {
    gift_result(gift::ALREADY_GIVEN)
}

/// WEDDING_GIFT_RESULT mode 14: the gift didn't go.
pub fn build_gift_failed() -> Result<Packet, NetworkError> {
    gift_result(gift::FAILED)
}

/// WEDDING_GIFT_RESULT mode 15: a gift was taken from `bag`, whose list
/// is now `gifts`' items of it.
pub fn build_gift_taken<'a>(
    bag: InventoryType,
    gifts: impl Iterator<Item = &'a Item> + Clone,
) -> Result<Packet, NetworkError> {
    let mut packet = gift_result(gift::TAKEN)?;
    write_gifts(&mut packet, &[bag], gifts)?;
    Ok(packet)
}

/// WEDDING_GIFT_RESULT mode 16: the gift wasn't taken.
pub fn build_gift_not_taken() -> Result<Packet, NetworkError> {
    gift_result(gift::NOT_TAKEN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::marriage::ENGAGED as STATUS;

    #[test]
    fn the_record_is_0x30_bytes() {
        let record = MarriageRecord {
            id: 7,
            groom_id: 1,
            bride_id: 2,
            status: STATUS,
            item_id: 2240000,
            groom_name: "Groom".into(),
            bride_name: "Bride".into(),
        };
        let packet = build_engaged(&record).unwrap();
        assert_eq!(packet.bytes.len(), 3 + 0x30);
        assert_eq!(&packet.bytes[..3], &[0x49, 0x00, 11]);
        // Both rings are the Moonstone box's wedding ring.
        assert_eq!(
            &packet.bytes[3 + 14..3 + 22],
            &[0xe3, 0xfa, 0x10, 0, 0xe3, 0xfa, 0x10, 0]
        );
        assert_eq!(&packet.bytes[3 + 22..3 + 27], b"Groom");
        assert_eq!(&packet.bytes[3 + 35..3 + 40], b"Bride");
    }

    #[test]
    fn the_priest_names_his_line_and_the_couple() {
        let packet = build_wedding_progress(1, 7, 8).unwrap();
        assert_eq!(packet.bytes, [0x40, 0x01, 1, 7, 0, 0, 0, 8, 0, 0, 0]);
        assert_eq!(build_wedding_hearts().unwrap().bytes, [0x41, 0x01]);
    }

    #[test]
    fn the_married_notice_names_its_group_and_the_partner() {
        let packet = build_married_notice(false, "Bob").unwrap();
        assert_eq!(
            packet.bytes,
            [0x6a, 0x00, 0, 5, 0, b'>', b' ', b'B', b'o', b'b']
        );
        let packet = build_married_notice(true, "Bob").unwrap();
        assert_eq!(packet.bytes[2], 1);
    }

    #[test]
    fn a_wedding_notice_is_its_mode_alone() {
        let packet = build_wedding_notice(WeddingNotice::Reserved).unwrap();
        assert_eq!(packet.bytes, [0x49, 0x00, 16]);
        let packet = build_wedding_notice(WeddingNotice::NoBreakingOff).unwrap();
        assert_eq!(packet.bytes, [0x49, 0x00, 32]);
        let packet = build_wedding_notice(WeddingNotice::InvalidInvitation).unwrap();
        assert_eq!(packet.bytes, [0x49, 0x00, 34]);
        let packet = build_wedding_notice(WeddingNotice::Cancelled).unwrap();
        assert_eq!(packet.bytes, [0x49, 0x00, 31]);
    }

    #[test]
    fn the_married_result_carries_the_record_with_its_status() {
        let record = MarriageRecord {
            id: 7,
            groom_id: 1,
            bride_id: 2,
            status: game::marriage::MARRIED,
            item_id: 2240001,
            groom_name: "Groom".into(),
            bride_name: "Bride".into(),
        };
        let packet = build_married(&record).unwrap();
        assert_eq!(packet.bytes.len(), 3 + 0x30);
        assert_eq!(&packet.bytes[..3], &[0x49, 0x00, 12]);
        assert_eq!(&packet.bytes[3 + 12..3 + 14], &[3, 0]);
        assert_eq!(&packet.bytes[3 + 14..3 + 18], &1112806i32.to_le_bytes());
    }

    #[test]
    fn an_invitation_names_the_couple_and_its_window() {
        let packet = build_invitation("Al", "Bo", Venue::Cathedral).unwrap();
        assert_eq!(
            packet.bytes,
            [0x49, 0x00, 15, 2, 0, b'A', b'l', 2, 0, b'B', b'o', 2, 0]
        );
        let packet = build_invitation("Al", "Bo", Venue::Chapel).unwrap();
        assert_eq!(packet.bytes[packet.bytes.len() - 2..], [1, 0]);
    }

    #[test]
    fn a_marriage_ring_follows_two_empty_ring_blocks() {
        let mut packet = Packet::new_empty();
        write_ring_looks(&mut packet, &RingLooks::default()).unwrap();
        assert_eq!(packet.bytes, vec![0, 0, 0]);
        let rings = RingLooks {
            marriage: Some(game::marriage::MarriageRing {
                self_id: 2,
                partner_id: 1,
                item_id: 1112803,
            }),
            ..RingLooks::default()
        };
        let mut packet = Packet::new_empty();
        write_ring_looks(&mut packet, &rings).unwrap();
        assert_eq!(
            packet.bytes,
            vec![0, 0, 1, 2, 0, 0, 0, 1, 0, 0, 0, 0xe3, 0xfa, 0x10, 0]
        );
    }

    #[test]
    fn a_crush_ring_block_holds_both_serials_and_the_item() {
        let rings = RingLooks {
            crush: Some(game::ring::RingLook {
                ring_id: 5,
                partner_ring_id: 6,
                item_id: 1112001,
            }),
            ..RingLooks::default()
        };
        let mut packet = Packet::new_empty();
        write_ring_looks(&mut packet, &rings).unwrap();
        let mut expected = vec![1, 5, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0];
        expected.extend(1112001i32.to_le_bytes());
        expected.extend([0, 0]);
        assert_eq!(packet.bytes, expected);
    }

    #[test]
    fn couple_rings_are_listed_by_kind() {
        let ring = |id, item_id| CoupleRing {
            id,
            partner_ring_id: id + 1,
            partner_id: 9,
            partner_name: "Pal".into(),
            item_id,
        };
        let mut packet = Packet::new_empty();
        write_couple_rings(&mut packet, &[ring(3, 1112800), ring(1, 1112001)]).unwrap();
        assert_eq!(packet.bytes.len(), 2 + 0x21 + 2 + 0x25);
        assert_eq!(&packet.bytes[..6], &[1, 0, 9, 0, 0, 0]);
        assert_eq!(&packet.bytes[6..9], b"Pal");
        assert_eq!(packet.bytes[2 + 0x11], 1);
        assert_eq!(packet.bytes[2 + 0x19], 2);
        assert_eq!(&packet.bytes[2 + 0x21..2 + 0x21 + 2], &[1, 0]);
        assert_eq!(
            packet.bytes[packet.bytes.len() - 4..],
            1112800i32.to_le_bytes()
        );
    }

    #[test]
    fn the_partner_map_is_two_ints() {
        let packet = build_partner_map(100000000, 4).unwrap();
        assert_eq!(
            packet.bytes,
            vec![0x4B, 0x00, 0x00, 0xe1, 0xf5, 0x05, 4, 0, 0, 0]
        );
    }

    #[test]
    fn a_refusal_is_its_mode_alone() {
        let packet = build_marriage_refusal(Refusal::Declined).unwrap();
        assert_eq!(packet.bytes, vec![0x49, 0x00, 30]);
    }

    fn listed(bags: &[(u8, &[&Item])]) -> Vec<u8> {
        let mut packet = Packet::new_empty();
        for (count, items) in bags {
            packet.write_byte(*count).unwrap();
            for item in *items {
                encode_item(&mut packet, SlotPrefix::None, item).unwrap();
            }
        }
        packet.bytes
    }

    #[test]
    fn the_gift_window_lists_every_bag_in_order() {
        let gifts = [
            Item::stack(4000000, 3),
            Item::stack(2000000, 1),
            Item::stack(4000001, 5),
        ];
        let packet = build_gifts(gifts.iter()).unwrap();
        assert_eq!(&packet.bytes[..3], &[0x4A, 0x00, 10]);
        assert_eq!(packet.bytes[3..11], 0x7Ci64.to_le_bytes());
        let expected = listed(&[
            (0, &[]),
            (1, &[&gifts[1]]),
            (0, &[]),
            (2, &[&gifts[0], &gifts[2]]),
            (0, &[]),
        ]);
        assert_eq!(packet.bytes[11..], expected[..]);
    }

    #[test]
    fn a_take_relists_its_bag_alone() {
        let gifts = [Item::stack(2000000, 1), Item::stack(4000001, 5)];
        let packet = build_gift_taken(InventoryType::Etc, gifts.iter()).unwrap();
        assert_eq!(&packet.bytes[..3], &[0x4A, 0x00, 15]);
        assert_eq!(packet.bytes[3..11], 0x20i64.to_le_bytes());
        assert_eq!(packet.bytes[11..], listed(&[(1, &[&gifts[1]])])[..]);
    }

    #[test]
    fn a_sent_gift_comes_with_the_wishlist() {
        let gift = Item::stack(2000000, 4);
        let packet = build_gift_sent(&["Cake".into()], &gift).unwrap();
        assert_eq!(
            &packet.bytes[..9],
            &[0x4A, 0x00, 11, 1, 4, 0, b'C', b'a', b'k']
        );
        assert_eq!(packet.bytes[10..18], 0x08i64.to_le_bytes());
        assert_eq!(packet.bytes[18..], listed(&[(1, &[&gift])])[..]);
        assert_eq!(build_registry(&[]).unwrap().bytes, vec![0x4A, 0x00, 9, 0]);
        assert_eq!(build_wishlist_input().unwrap().bytes, vec![0x48, 0x00, 9]);
    }
}
