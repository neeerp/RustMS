use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

pub fn build_quest_prompt(prompt: &crate::quests::QuestPrompt) -> Result<Packet, NetworkError> {
    let (npc_id, message_type, text) = match prompt {
        crate::quests::QuestPrompt::Text(page) => return build_npc_dialogue(page),
        crate::quests::QuestPrompt::YesNo { npc_id, text } => (*npc_id, 1, text),
        crate::quests::QuestPrompt::Selection { npc_id, text } => (*npc_id, 4, text),
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
    packet.write_int(npc.object_id)?;
    packet.write_int(npc.npc_id)?;
    packet.write_short(npc.x)?;
    packet.write_short(npc.y)?;
    packet.write_byte(if npc.flip { 1 } else { 0 })?;
    packet.write_short(npc.foothold)?;
    packet.write_short(npc.rx0)?;
    packet.write_short(npc.rx1)?;
    packet.write_byte(1)?; // show on the minimap
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::dialogue::DialoguePage;
    use packet::io::read::PktRead;
    use std::io::Cursor;

    #[test]
    fn build_spawn_npc_matches_expected_payload() {
        let packet = build_spawn_npc(&ForeignNpc {
            object_id: 1_000_000_000,
            npc_id: 2100,
            x: 1200,
            y: 175,
            flip: true,
            foothold: 32,
            rx0: 1150,
            rx1: 1250,
        })
        .expect("build spawn npc");

        let mut cursor = Cursor::new(&packet.bytes[..]);
        assert_eq!(
            cursor.read_short().expect("opcode"),
            SendOpcode::SpawnNpc as i16
        );
        assert_eq!(cursor.read_int().expect("object id"), 1_000_000_000);
        assert_eq!(cursor.read_int().expect("npc id"), 2100);
        assert_eq!(cursor.read_short().expect("x"), 1200);
        assert_eq!(cursor.read_short().expect("y"), 175);
        assert_eq!(cursor.read_byte().expect("flip"), 1);
        assert_eq!(cursor.read_short().expect("foothold"), 32);
        assert_eq!(cursor.read_short().expect("rx0"), 1150);
        assert_eq!(cursor.read_short().expect("rx1"), 1250);
    }

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
        expect.byte(1); // reference: 0 when the life entry has f = 1, else 1
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
    fn quest_prompt_text_is_a_dialogue_page() {
        let prompt = crate::quests::QuestPrompt::Text(page(true, true));
        let packet = build_quest_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(9010000).byte(0).byte(0);
        expect.string("Welcome!").byte(1).byte(1);
        expect.end();
    }

    #[test]
    fn quest_prompt_yes_no_uses_message_type_one_without_trailer() {
        let prompt = crate::quests::QuestPrompt::YesNo {
            npc_id: 1012100,
            text: "Will you help?".to_string(),
        };
        let packet = build_quest_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(1012100).byte(1).byte(0);
        expect.string("Will you help?");
        expect.end();
    }

    #[test]
    fn quest_prompt_selection_uses_message_type_four_without_trailer() {
        let prompt = crate::quests::QuestPrompt::Selection {
            npc_id: 1012100,
            text: "#L0#Yes#l".to_string(),
        };
        let packet = build_quest_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(1012100).byte(4).byte(0);
        expect.string("#L0#Yes#l");
        expect.end();
    }
}
