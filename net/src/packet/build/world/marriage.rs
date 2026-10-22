//! Engagement (report b052): MARRIAGE_REQUEST asks the partner, and
//! MARRIAGE_RESULT answers the proposer; any result first closes the
//! proposer's "Waiting for her response..." box.

use crate::{codec::stats::write_padded_name, error::NetworkError, packet::op::SendOpcode};
use game::marriage::{MarriageRecord, Refusal};
use game::ring::{CoupleRing, RingKind, RingLooks};
use game::wedding::Venue;
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

/// MARRIAGE_RESULT 36: `text` in the client's OK box.
pub fn build_marriage_notice(text: &str) -> Result<Packet, NetworkError> {
    let mut packet = result(NOTICE)?;
    packet.write_byte(1)?;
    packet.write_str_with_length(text)?;
    Ok(packet)
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
}
