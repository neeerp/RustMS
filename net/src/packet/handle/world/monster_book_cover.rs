use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::monster_book::build_set_cover;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// 0x39 `MONSTER_BOOK_COVER`: show a collected card (or none, 0) on the
/// Monster Book's cover. Anything else leaves the cover as it was.
pub struct MonsterBookCoverHandler;

impl WorldHandler for MonsterBookCoverHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let card = reader.read_int()?;
        if card != ctx.player.monster_book.cover() {
            ctx.player.monster_book_mut().set_cover(card);
        }
        Ok(HandlerResult::reply(build_set_cover(
            ctx.player.monster_book.cover(),
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldAction;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::MonsterBook;
    use packet::io::write::PktWrite;

    fn choose_cover(player: &mut game::Player, card: i32) -> i32 {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::MonsterBookCover as i16).unwrap();
        p.write_int(card).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        let actions = MonsterBookCoverHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions;
        let [WorldAction::Reply(reply)] = actions.as_slice() else {
            panic!("one reply: {actions:?}");
        };
        assert_eq!(reply.opcode(), SendOpcode::MonsterBookSetCover as i16);
        let mut reader = Cursor::new(&reply.bytes);
        reader.read_short().unwrap();
        reader.read_int().unwrap()
    }

    #[test]
    fn covers_with_a_collected_card_or_nothing() {
        let mut player = crate::codec::stats::sample_player();
        player.monster_book = MonsterBook::new(0, [(2380000, 1)]);
        assert_eq!(choose_cover(&mut player, 2380000), 2380000);
        assert!(player.is_dirty());
        assert_eq!(choose_cover(&mut player, 2380001), 2380000);
        assert_eq!(choose_cover(&mut player, 0), 0);
        assert_eq!(player.monster_book.cover(), 0);
    }
}
