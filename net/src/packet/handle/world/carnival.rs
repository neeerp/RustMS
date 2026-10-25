use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `MONSTER_CARNIVAL` (v83ref b163): the Carnival window spends CP on
/// `index` of tab `tab` (0 mobs, 1 skills, 2 guardians). The client waits
/// for a SUMMON or a MESSAGE before it sends another.
pub struct MonsterCarnivalHandler;

impl WorldHandler for MonsterCarnivalHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let tab = reader.read_byte()?;
        let index = reader.read_byte()?;
        Ok(HandlerResult::empty().with(WorldAction::CarnivalSpend { tab, index }))
    }
}
