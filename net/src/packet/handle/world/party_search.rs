use crate::handler::{HandlerResult, PartyRequest, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// 0xDE, `PARTY_SEARCH_START`: the minimum and maximum level, the party size
/// wanted, and the job checkboxes (FUN_00a3e264). Nothing answers it; the
/// world directory checks it and invites whoever it finds.
pub struct PartySearchStartHandler;

impl WorldHandler for PartySearchStartHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = PartyRequest::SearchStart {
            min_level: reader.read_int()?,
            max_level: reader.read_int()?,
            members: reader.read_int()?,
            jobs: reader.read_int()? as u32,
        };
        Ok(HandlerResult::empty().with(WorldAction::Party(request)))
    }
}

/// 0xDF, with nothing after the opcode (FUN_00a3e2d7). v83 sends it to pause
/// or stop a search, and after any map change in the same channel, which
/// ends one.
pub struct PartySearchStopHandler;

impl WorldHandler for PartySearchStopHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty().with(WorldAction::Party(PartyRequest::SearchStop)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn reads_a_search() {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::PartySearchStart as i16)
            .unwrap();
        for value in [20, 40, 6, 0x1ff_ffff] {
            packet.write_int(value).unwrap();
        }
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let action = PartySearchStartHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
            .pop();
        let Some(WorldAction::Party(request)) = action else {
            panic!("not a party request");
        };
        assert_eq!(
            request,
            PartyRequest::SearchStart {
                min_level: 20,
                max_level: 40,
                members: 6,
                jobs: 0x1ff_ffff,
            }
        );
    }
}
