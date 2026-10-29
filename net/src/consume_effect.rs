//! The aura some item buffs draw over the player (SHOW_CONSUME_EFFECT, recv
//! 166, report c029 A): the squad victory potions and the ARPQ shield.
//! Pharaoh's Blessing and Power Buff draw theirs through their buff
//! instead (`BuffStat::ItemAura`, report b339).
//! The v83 client keeps the item id (user+0x1f64) and draws
//! `Effect/ItemEff.img/<id>` until told 0, the owner's own client included;
//! nothing in SPAWN_PLAYER carries it, so those who arrive later are told
//! apart. The server shows it while the item's buff lasts.

use game::Player;
use packet::Packet;

use crate::error::NetworkError;
use crate::packet::build::world::field::build_show_consume_effect;

/// The items whose buff has an aura: the flat `ItemEff.img` entries of the
/// timed USE buffs (Horntail Squad Victory, Breath of Nine Spirit, the ARPQ
/// shield, Pink Bean Squad Victory) whose buff doesn't draw it itself.
pub const ITEMS: [i32; 4] = [2022108, 2022109, 2022269, 2022449];

/// The item whose aura `player`'s buffs show, or 0. Item buffs are kept
/// under `-item_id`.
pub fn due(player: &Player) -> i32 {
    player
        .buffs
        .active()
        .iter()
        .map(|buff| -buff.skill_id)
        .find(|item_id| ITEMS.contains(item_id))
        .unwrap_or(0)
}

/// Bring the aura the others were shown up to date with the buffs: the
/// SHOW_CONSUME_EFFECT for the player and their map if it changed.
pub fn refresh(player: &mut Player) -> Result<Option<Packet>, NetworkError> {
    let due = due(player);
    if due == player.consume_effect {
        return Ok(None);
    }
    player.consume_effect = due;
    build_show_consume_effect(player.id(), due).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::SendOpcode;
    use game::{Buff, BuffStat};
    use std::io::Cursor;
    use std::time::{Duration, Instant};

    use packet::io::read::PktRead;

    const SQUAD: i32 = 2022108;

    fn shown(packet: Packet) -> (i16, i32, i32) {
        let mut r = Cursor::new(&packet.bytes);
        (
            r.read_short().unwrap(),
            r.read_int().unwrap(),
            r.read_int().unwrap(),
        )
    }

    #[test]
    fn the_aura_shows_while_the_item_buff_lasts() {
        let mut player = crate::codec::stats::sample_player();
        let now = Instant::now();
        let hour = Duration::from_secs(3600);
        assert!(refresh(&mut player).unwrap().is_none());
        player
            .buffs
            .give(Buff::new(-2000000, BuffStat::Pad, 5, now, hour));
        assert!(refresh(&mut player).unwrap().is_none(), "no aura");
        player
            .buffs
            .give(Buff::new(-SQUAD, BuffStat::Mad, 5, now, hour));
        let op = SendOpcode::ShowConsumeEffect as i16;
        let id = player.id();
        assert_eq!(
            shown(refresh(&mut player).unwrap().unwrap()),
            (op, id, SQUAD)
        );
        assert!(refresh(&mut player).unwrap().is_none(), "told once");
        player.buffs.cancel(-SQUAD);
        assert_eq!(shown(refresh(&mut player).unwrap().unwrap()), (op, id, 0));
        assert_eq!(player.consume_effect, 0);
    }
}
