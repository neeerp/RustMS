use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::loot::Loot;
use crate::packet::build::world::stat::{self, StatUpdate};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// The most and fewest mesos one drop may hold.
const MESO_DROP_MIN: i32 = 10;
const MESO_DROP_MAX: i32 = 50_000;

/// `MESO_DROP`: tick, amount. Throws a meso bag down where the player stands.
pub struct MesoDropHandler;

impl WorldHandler for MesoDropHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let amount = reader.read_int()?;

        let player = &mut *ctx.player;
        let field = crate::get_game_data()
            .ok()
            .and_then(|data| data.field(player.stats.map_id));
        if player.stats.hp <= 0
            || !(MESO_DROP_MIN..=MESO_DROP_MAX).contains(&amount)
            || amount > player.meso
            || !field.is_none_or(|field| field.may_drop())
        {
            debug!(amount, meso = player.meso, "Meso drop refused");
            // Unlock the client's item actions.
            return Ok(HandlerResult::empty()
                .with(WorldAction::Reply(stat::build_stat_update(true, &[])?)));
        }
        player.meso -= amount;
        player.mark_dirty();
        let meso = player.meso;
        Ok(HandlerResult::empty()
            .with(WorldAction::Reply(stat::build_stat_update(
                true,
                &[StatUpdate::Meso(meso)],
            )?))
            .with(WorldAction::Field(FieldRequest::Throw {
                loot: Loot::Mesos(amount),
                vanish: false,
            })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use packet::io::write::PktWrite;

    fn drop_mesos(player: &mut game::Player, amount: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::MesoDrop as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_int(amount).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        MesoDropHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    #[test]
    fn drops_mesos_the_player_has() {
        let mut player = crate::codec::stats::sample_player();
        player.meso = 100;
        let actions = drop_mesos(&mut player, 40);
        assert!(matches!(
            actions.as_slice(),
            [
                WorldAction::Reply(p),
                WorldAction::Field(FieldRequest::Throw { loot: Loot::Mesos(40), vanish: false }),
            ] if p.opcode() == SendOpcode::StatChange as i16
        ));
        assert_eq!(player.meso, 60);
        assert!(player.is_dirty());
    }

    #[test]
    fn refuses_amounts_out_of_range_or_not_held() {
        let mut player = crate::codec::stats::sample_player();
        player.meso = 100_000;
        for amount in [9, 50_001, -5] {
            let actions = drop_mesos(&mut player, amount);
            assert!(matches!(actions.as_slice(), [WorldAction::Reply(_)]));
        }
        player.meso = 30;
        assert!(matches!(
            drop_mesos(&mut player, 31).as_slice(),
            [WorldAction::Reply(_)]
        ));
        player.stats.hp = 0;
        assert!(matches!(
            drop_mesos(&mut player, 10).as_slice(),
            [WorldAction::Reply(_)]
        ));
        assert_eq!(player.meso, 30);
    }

    #[test]
    fn refuses_drops_where_the_map_forbids_them() {
        if crate::get_game_data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.meso = 100;
        player.stats.map_id = 910_000_001;
        assert!(matches!(
            drop_mesos(&mut player, 10).as_slice(),
            [WorldAction::Reply(_)]
        ));
        assert_eq!(player.meso, 100);
    }
}
