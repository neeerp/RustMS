use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Any server-driven NPC prompt: a text page, a yes/no question, or a menu.
pub fn build_prompt(prompt: &crate::dialogue::Prompt) -> Result<Packet, NetworkError> {
    use crate::dialogue::Prompt;
    let (npc_id, message_type, text) = match prompt {
        Prompt::Text(page) => return build_npc_dialogue(page),
        Prompt::YesNo { npc_id, text } => (*npc_id, 1, text),
        Prompt::Selection { npc_id, text } => (*npc_id, 4, text),
    };
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::NpcDialogue as i16)?;
    p.write_byte(4)?;
    p.write_int(npc_id)?;
    p.write_byte(message_type)?;
    p.write_byte(0)?;
    p.write_str_with_length(text)?;
    Ok(p)
}

/// v83 text dialogue: message type 0, followed by separate Back/Next bytes.
pub fn build_npc_dialogue(page: &crate::dialogue::DialoguePage) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NpcDialogue as i16)?;
    packet.write_byte(4)?; // NPC speaker category
    packet.write_int(page.npc_id)?;
    packet.write_byte(0)?; // text dialogue
    packet.write_byte(0)?; // NPC portrait
    packet.write_str_with_length(&page.text)?;
    packet.write_byte(u8::from(page.has_previous))?;
    packet.write_byte(u8::from(page.has_next))?;
    Ok(packet)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignNpc {
    pub object_id: i32,
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    pub foothold: i16,
    pub rx0: i16,
    pub rx1: i16,
}

pub fn build_spawn_npc(npc: &ForeignNpc) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpawnNpc as i16)?;
    write_npc(&mut packet, npc)?;
    Ok(packet)
}

/// The client now picks the NPC's idle actions: mode 1, then the NPC as
/// SPAWN_NPC writes it, for a client that hasn't seen it yet.
pub fn build_control_npc(npc: &ForeignNpc) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpawnNpcRequestController as i16)?;
    packet.write_byte(1)?;
    write_npc(&mut packet, npc)?;
    Ok(packet)
}

/// NPC_ACTION: everyone, the controller too, plays the action and line it
/// chose. `action` is what followed the object id in its request.
pub fn build_npc_action(object_id: i32, action: &[u8]) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NpcAction as i16)?;
    packet.write_int(object_id)?;
    packet.write_bytes(action)?;
    Ok(packet)
}

fn write_npc(packet: &mut Packet, npc: &ForeignNpc) -> Result<(), NetworkError> {
    packet.write_int(npc.object_id)?;
    packet.write_int(npc.npc_id)?;
    packet.write_short(npc.x)?;
    packet.write_short(npc.y)?;
    packet.write_byte(if npc.flip { 1 } else { 0 })?;
    packet.write_short(npc.foothold)?;
    packet.write_short(npc.rx0)?;
    packet.write_short(npc.rx1)?;
    packet.write_byte(1)?; // show on the minimap
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::dialogue::DialoguePage;

    fn foreign_npc(flip: bool) -> ForeignNpc {
        ForeignNpc {
            object_id: 1_000_000_001,
            npc_id: 2100,
            x: 1200,
            y: -175,
            flip,
            foothold: 32,
            rx0: 1150,
            rx1: 1250,
        }
    }

    #[test]
    fn spawn_npc_matches_reference() {
        let packet = build_spawn_npc(&foreign_npc(true)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnNpc as i16);
        expect.int(1_000_000_001).int(2100);
        expect.short(1200).short(-175);
        // The reference writes `f == 1 ? 0 : 1`; `flip` is already `!f`
        // (see game-data), so it goes out as-is.
        expect.byte(1);
        expect.short(32).short(1150).short(1250);
        expect.byte(1); // shown on the minimap
        expect.end();
    }

    #[test]
    fn spawn_npc_unflipped_writes_zero() {
        let packet = build_spawn_npc(&foreign_npc(false)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnNpc as i16);
        expect.int(1_000_000_001).int(2100);
        expect.short(1200).short(-175).byte(0);
        expect.short(32).short(1150).short(1250).byte(1);
        expect.end();
    }

    #[test]
    fn control_npc_is_mode_one_and_the_spawn() {
        let packet = build_control_npc(&foreign_npc(true)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnNpcRequestController as i16);
        expect.byte(1).int(1_000_000_001).int(2100);
        expect.short(1200).short(-175).byte(1);
        expect.short(32).short(1150).short(1250).byte(1);
        expect.end();
    }

    #[test]
    fn npc_action_relays_the_choice() {
        let packet = build_npc_action(1_000_000_001, &[3, 0xff]).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcAction as i16);
        expect.int(1_000_000_001).byte(3).byte(0xff);
        expect.end();
    }

    fn page(has_previous: bool, has_next: bool) -> DialoguePage {
        DialoguePage {
            npc_id: 9010000,
            text: "Welcome!".to_string(),
            has_previous,
            has_next,
        }
    }

    #[test]
    fn dialogue_page_with_next_matches_reference() {
        // Reference sendNext: msgType 0, then "00 01" (previous, next).
        let packet = build_npc_dialogue(&page(false, true)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(9010000).byte(0).byte(0);
        expect.string("Welcome!").byte(0).byte(1);
        expect.end();
    }

    #[test]
    fn dialogue_page_with_previous_and_ok_matches_reference() {
        // Reference sendPrev is "01 00"; sendOk is "00 00".
        let prev = build_npc_dialogue(&page(true, false)).unwrap();
        Expect::new(&prev.bytes)
            .short(SendOpcode::NpcDialogue as i16)
            .byte(4)
            .int(9010000)
            .byte(0)
            .byte(0)
            .string("Welcome!")
            .byte(1)
            .byte(0)
            .end();

        let ok = build_npc_dialogue(&page(false, false)).unwrap();
        Expect::new(&ok.bytes)
            .short(SendOpcode::NpcDialogue as i16)
            .byte(4)
            .int(9010000)
            .byte(0)
            .byte(0)
            .string("Welcome!")
            .byte(0)
            .byte(0)
            .end();
    }

    #[test]
    fn prompt_text_is_a_dialogue_page() {
        let prompt = crate::dialogue::Prompt::Text(page(true, true));
        let packet = build_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(9010000).byte(0).byte(0);
        expect.string("Welcome!").byte(1).byte(1);
        expect.end();
    }

    #[test]
    fn prompt_yes_no_uses_message_type_one_without_trailer() {
        let prompt = crate::dialogue::Prompt::YesNo {
            npc_id: 1012100,
            text: "Will you help?".to_string(),
        };
        let packet = build_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(1012100).byte(1).byte(0);
        expect.string("Will you help?");
        expect.end();
    }

    #[test]
    fn prompt_selection_uses_message_type_four_without_trailer() {
        let prompt = crate::dialogue::Prompt::Selection {
            npc_id: 1012100,
            text: "#L0#Yes#l".to_string(),
        };
        let packet = build_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(1012100).byte(4).byte(0);
        expect.string("#L0#Yes#l");
        expect.end();
    }
}
