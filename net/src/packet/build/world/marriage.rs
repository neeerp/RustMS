//! Engagement (report b052): MARRIAGE_REQUEST asks the partner, and
//! MARRIAGE_RESULT answers the proposer; any result first closes the
//! proposer's "Waiting for her response..." box.

use crate::{codec::stats::write_padded_name, error::NetworkError, packet::op::SendOpcode};
use game::marriage::{MarriageRecord, MarriageRing, Refusal};
use packet::{io::write::PktWrite, Packet};

/// The couple is engaged now; the client keeps the record.
const ENGAGED: u8 = 11;
/// The engagement is broken; the client drops the record.
const BROKEN: u8 = 13;
/// The marriage is over; the client drops the record.
const DIVORCED: u8 = 14;
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
/// it: crush and friendship rings (none), then the marriage ring
/// (report b059). The client pairs the effect only with real ids on both
/// sides.
pub fn write_ring_looks(
    packet: &mut Packet,
    marriage: Option<&MarriageRing>,
) -> Result<(), NetworkError> {
    packet.write_byte(0)?; // crush ring
    packet.write_byte(0)?; // friendship ring
    packet.write_byte(u8::from(marriage.is_some()))?;
    if let Some(ring) = marriage {
        packet.write_int(ring.self_id)?;
        packet.write_int(ring.partner_id)?;
        packet.write_int(ring.item_id)?;
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

/// MARRIAGE_RESULT 11: "You are now engaged.", with the couple's record.
pub fn build_engaged(record: &MarriageRecord) -> Result<Packet, NetworkError> {
    let mut packet = result(ENGAGED)?;
    write_marriage_record(&mut packet, record)?;
    Ok(packet)
}

/// MARRIAGE_RESULT 13, "Your engagement has been broken.", or for a married
/// couple 14, "You are no longer married."
pub fn build_couple_ended(record: &MarriageRecord) -> Result<Packet, NetworkError> {
    result(if record.married() { DIVORCED } else { BROKEN })
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
    fn a_marriage_ring_follows_two_empty_ring_blocks() {
        let mut packet = Packet::new_empty();
        write_ring_looks(&mut packet, None).unwrap();
        assert_eq!(packet.bytes, vec![0, 0, 0]);
        let ring = MarriageRing {
            self_id: 2,
            partner_id: 1,
            item_id: 1112803,
        };
        let mut packet = Packet::new_empty();
        write_ring_looks(&mut packet, Some(&ring)).unwrap();
        assert_eq!(
            packet.bytes,
            vec![0, 0, 1, 2, 0, 0, 0, 1, 0, 0, 0, 0xe3, 0xfa, 0x10, 0]
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
