use crate::handler::{
    BuddyRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `BUDDYLIST_MODIFY`: add a buddy, accept a request, or remove a buddy
/// (which also declines a request).
pub struct BuddyListModifyHandler;

impl WorldHandler for BuddyListModifyHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            1 => BuddyRequest::Add {
                name: reader.read_str_with_length()?,
                group: reader.read_str_with_length()?,
            },
            2 => BuddyRequest::Accept {
                id: reader.read_int()?,
            },
            3 => BuddyRequest::Remove {
                id: reader.read_int()?,
            },
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Buddy(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn request(bytes: &[u8]) -> Option<BuddyRequest> {
        let mut packet = Packet::new(bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = BuddyListModifyHandler.handle(&mut packet, &mut ctx);
        match result.unwrap().actions.pop() {
            Some(WorldAction::Buddy(request)) => Some(request),
            None => None,
            other => panic!("unexpected {other:?}"),
        }
    }

    fn modify(mode: u8) -> Packet {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::BuddyListModify as i16)
            .unwrap();
        packet.write_byte(mode).unwrap();
        packet
    }

    #[test]
    fn reads_add_accept_and_remove() {
        let mut add = modify(1);
        add.write_str_with_length("Bobby").unwrap();
        add.write_str_with_length("Friends").unwrap();
        assert_eq!(
            request(&add.bytes),
            Some(BuddyRequest::Add {
                name: "Bobby".into(),
                group: "Friends".into()
            })
        );

        let mut accept = modify(2);
        accept.write_int(7).unwrap();
        assert_eq!(request(&accept.bytes), Some(BuddyRequest::Accept { id: 7 }));

        let mut remove = modify(3);
        remove.write_int(7).unwrap();
        assert_eq!(request(&remove.bytes), Some(BuddyRequest::Remove { id: 7 }));

        assert_eq!(request(&modify(9).bytes), None);
    }
}
