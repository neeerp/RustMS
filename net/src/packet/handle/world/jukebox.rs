//! The jukebox (510xxxx), used through USE_CASH_ITEM with the song's length
//! in ms, which nothing reads (v83 FUN_00a0a63f, type 0x14). Everyone on the
//! map hears the song, under a line with the player's name (report 201).

use crate::handler::{FieldRequest, HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::packet::build::world::effect::build_play_jukebox;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::build_stat_update;
use game::InventoryType;

/// Play the song of the jukebox `item_id` in Cash `slot`.
pub fn jukebox(ctx: &mut WorldContext, slot: i16, item_id: i32) -> WorldResult {
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    // The song is the map's, so a hidden GM's plays for everyone too.
    let song = build_play_jukebox(item_id, ctx.player.name())?;
    Ok(
        HandlerResult::reply(build_inventory_operation(true, &changes)?)
            .with(WorldAction::Reply(song.clone()))
            .with(WorldAction::Field(FieldRequest::BroadcastAll(song))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldHandler;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{Item, Player};
    use packet::{io::write::PktWrite, Packet};

    const SONG: i32 = 5100000;

    fn use_jukebox(player: &mut Player, slot: i16) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(SONG).unwrap();
        p.write_int(67030).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    #[test]
    fn a_jukebox_plays_its_song_for_the_map() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 3, Item::stack(SONG, 2));
        let actions = use_jukebox(&mut player, 3);
        let [WorldAction::Reply(used), WorldAction::Reply(own), WorldAction::Field(FieldRequest::BroadcastAll(song))] =
            &actions[..]
        else {
            panic!("expected a reply and a song: {actions:?}");
        };
        assert_eq!(used.bytes[..2], [0x1D, 0], "an inventory operation");
        assert_eq!(own.bytes, song.bytes);
        assert_eq!(
            song.bytes,
            build_play_jukebox(SONG, "Tester").unwrap().bytes
        );
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Cash, 3)
                .unwrap()
                .quantity,
            1
        );
    }

    #[test]
    fn no_jukebox_plays_nothing() {
        let mut player = crate::codec::stats::sample_player();
        let actions = use_jukebox(&mut player, 3);
        let [WorldAction::Reply(packet)] = &actions[..] else {
            panic!("expected a lone reply: {actions:?}");
        };
        assert_eq!(packet.bytes[..2], [0x1F, 0], "a stat change to unlock");
    }
}
