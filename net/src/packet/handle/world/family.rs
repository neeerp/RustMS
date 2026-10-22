//! The family's requests (report b039, section 2). The client sends all but
//! the first two only once FAMILY_INFO has come.

use crate::handler::{
    FamilyRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

fn request(request: FamilyRequest) -> WorldResult {
    Ok(HandlerResult::empty().with(WorldAction::Family(request)))
}

/// `OPEN_FAMILY_PEDIGREE`: the Family Tree of the character named, from the
/// Family window (the player's own) or a character's menu.
pub struct OpenFamilyPedigreeHandler;

impl WorldHandler for OpenFamilyPedigreeHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        request(FamilyRequest::Chart { name })
    }
}

/// `OPEN_FAMILY`: the Family window opened, and wants its contents.
pub struct OpenFamilyHandler;

impl WorldHandler for OpenFamilyHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        request(FamilyRequest::Open)
    }
}

/// `ADD_FAMILY`: Add Junior, with the name typed.
pub struct AddFamilyHandler;

impl WorldHandler for AddFamilyHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        request(FamilyRequest::Add { name })
    }
}

/// `ACCEPT_FAMILY`: the asker's id and name, and whether the player said
/// yes. The client says no by itself when it isn't showing the card.
pub struct AcceptFamilyHandler;

impl WorldHandler for AcceptFamilyHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let inviter = reader.read_int()?;
        let _name = reader.read_str_with_length()?;
        let accept = reader.read_byte()? != 0;
        request(FamilyRequest::Answer { inviter, accept })
    }
}

/// `SEPARATE_FAMILY_BY_SENIOR`: the player parts from a junior, by id
/// (report b048).
pub struct SeparateFamilyBySeniorHandler;

impl WorldHandler for SeparateFamilyBySeniorHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let junior = reader.read_int()?;
        request(FamilyRequest::Separate {
            junior: Some(junior),
        })
    }
}

/// `SEPARATE_FAMILY_BY_JUNIOR`: the player parts from their senior.
pub struct SeparateFamilyByJuniorHandler;

impl WorldHandler for SeparateFamilyByJuniorHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        request(FamilyRequest::Separate { junior: None })
    }
}

/// `CHANGE_FAMILY_MESSAGE`: the leader's new precepts.
pub struct ChangeFamilyMessageHandler;

impl WorldHandler for ChangeFamilyMessageHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let precepts = reader.read_str_with_length()?;
        request(FamilyRequest::Precepts { precepts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn handle(handler: &dyn WorldHandler, packet: Packet) -> FamilyRequest {
        let mut packet = Packet::new(&packet.bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        match handler.handle(&mut packet, &mut ctx).unwrap().actions.pop() {
            Some(WorldAction::Family(request)) => request,
            other => panic!("unexpected {other:?}"),
        }
    }

    fn packet(op: RecvOpcode) -> Packet {
        let mut packet = Packet::new_empty();
        packet.write_short(op as i16).unwrap();
        packet
    }

    #[test]
    fn reads_each_request() {
        let mut chart = packet(RecvOpcode::OpenFamilyPedigree);
        chart.write_str_with_length("Alice").unwrap();
        assert_eq!(
            handle(&OpenFamilyPedigreeHandler, chart),
            FamilyRequest::Chart {
                name: "Alice".into()
            }
        );
        assert_eq!(
            handle(&OpenFamilyHandler, packet(RecvOpcode::OpenFamily)),
            FamilyRequest::Open
        );
        let mut add = packet(RecvOpcode::AddFamily);
        add.write_str_with_length("Bobby").unwrap();
        assert_eq!(
            handle(&AddFamilyHandler, add),
            FamilyRequest::Add {
                name: "Bobby".into()
            }
        );
        let mut accept = packet(RecvOpcode::AcceptFamily);
        accept.write_int(7).unwrap();
        accept.write_str_with_length("Carol").unwrap();
        accept.write_byte(1).unwrap();
        assert_eq!(
            handle(&AcceptFamilyHandler, accept),
            FamilyRequest::Answer {
                inviter: 7,
                accept: true
            }
        );
        let mut separate = packet(RecvOpcode::SeparateFamilyBySenior);
        separate.write_int(9).unwrap();
        assert_eq!(
            handle(&SeparateFamilyBySeniorHandler, separate),
            FamilyRequest::Separate { junior: Some(9) }
        );
        assert_eq!(
            handle(
                &SeparateFamilyByJuniorHandler,
                packet(RecvOpcode::SeparateFamilyByJunior)
            ),
            FamilyRequest::Separate { junior: None }
        );
        let mut precepts = packet(RecvOpcode::ChangeFamilyMessage);
        precepts.write_str_with_length("Be kind").unwrap();
        assert_eq!(
            handle(&ChangeFamilyMessageHandler, precepts),
            FamilyRequest::Precepts {
                precepts: "Be kind".into()
            }
        );
    }
}
