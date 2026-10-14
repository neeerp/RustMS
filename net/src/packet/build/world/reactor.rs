use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// A reactor (a breakable box and the like) as players see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReactorView {
    pub object_id: i32,
    pub reactor_id: i32,
    pub state: u8,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
}

/// SPAWN_REACTOR: show a reactor.
pub fn build_spawn_reactor(reactor: &ReactorView) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnReactor as i16)?;
    p.write_int(reactor.object_id)?;
    p.write_int(reactor.reactor_id)?;
    p.write_byte(reactor.state)?;
    p.write_short(reactor.x)?;
    p.write_short(reactor.y)?;
    p.write_byte(u8::from(reactor.flip))?;
    p.write_str_with_length("")?; // name
    Ok(p)
}

/// HIT_REACTOR: a hit moved the reactor to its current state.
pub fn build_hit_reactor(reactor: &ReactorView, stance: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::HitReactor as i16)?;
    p.write_int(reactor.object_id)?;
    p.write_byte(reactor.state)?;
    p.write_short(reactor.x)?;
    p.write_short(reactor.y)?;
    p.write_short(stance)?;
    p.write_byte(0)?;
    p.write_byte(5)?; // frame delay
    Ok(p)
}

/// REMOVE_REACTOR: the reactor broke (the client plays its last hit
/// animation) or leaves the map.
pub fn build_remove_reactor(reactor: &ReactorView) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveReactor as i16)?;
    p.write_int(reactor.object_id)?;
    p.write_byte(reactor.state)?;
    p.write_short(reactor.x)?;
    p.write_short(reactor.y)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: ReactorView = ReactorView {
        object_id: 1_200_000_000,
        reactor_id: 2001,
        state: 1,
        x: 610,
        y: 254,
        flip: false,
    };

    #[test]
    fn reactor_packets_match_the_client_handlers() {
        let spawn = build_spawn_reactor(&BOX).unwrap();
        assert_eq!(
            spawn.bytes,
            [
                &(SendOpcode::SpawnReactor as i16).to_le_bytes()[..],
                &1_200_000_000i32.to_le_bytes(),
                &2001i32.to_le_bytes(),
                &[1],
                &610i16.to_le_bytes(),
                &254i16.to_le_bytes(),
                &[0, 0, 0],
            ]
            .concat()
        );
        let hit = build_hit_reactor(&BOX, 0).unwrap();
        assert_eq!(hit.bytes.len(), 2 + 4 + 1 + 4 + 4);
        let remove = build_remove_reactor(&BOX).unwrap();
        assert_eq!(remove.bytes.len(), 2 + 4 + 1 + 4);
    }
}
