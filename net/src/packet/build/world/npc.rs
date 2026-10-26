use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Any server-driven NPC prompt: a text page, a yes/no question, a menu, a
/// choice of styles, a line of text to type, or a number.
pub fn build_prompt(prompt: &crate::dialogue::Prompt) -> Result<Packet, NetworkError> {
    use crate::dialogue::Prompt;
    let (npc_id, message_type, text) = match prompt {
        Prompt::Text(page) => return build_npc_dialogue(page),
        Prompt::YesNo { npc_id, text } => (*npc_id, 1, text),
        Prompt::Selection { npc_id, text } => (*npc_id, 4, text),
        Prompt::Style { npc_id, text, .. } => (*npc_id, 7, text),
        Prompt::AskText { npc_id, text, .. } => (*npc_id, 2, text),
        Prompt::AskNumber { npc_id, text, .. } => (*npc_id, 3, text),
        Prompt::Slide { npc_id, set, menu } => return build_slide_menu(*npc_id, *set, menu),
    };
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::NpcDialogue as i16)?;
    p.write_byte(4)?;
    p.write_int(npc_id)?;
    p.write_byte(message_type)?;
    p.write_byte(0)?;
    p.write_str_with_length(text)?;
    if let Prompt::Style { styles, .. } = prompt {
        // The client takes a count byte and that many ids.
        p.write_byte(styles.len() as u8)?;
        for &style in styles {
            p.write_int(style)?;
        }
    }
    if let Prompt::AskText { min, max, .. } = prompt {
        // The box starts empty.
        p.write_str_with_length("")?;
        p.write_short(*min)?;
        p.write_short(*max)?;
    }
    if let Prompt::AskNumber {
        default, min, max, ..
    } = prompt
    {
        p.write_int(*default)?;
        p.write_int(*min)?;
        p.write_int(*max)?;
    }
    Ok(p)
}

/// Message type 14, a slide menu: the art set, then the `#<id>#<label>`
/// pairs. The client reads the speaker and the NPC but heeds neither.
fn build_slide_menu(npc_id: i32, set: i32, menu: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::NpcDialogue as i16)?;
    p.write_byte(4)?;
    p.write_int(npc_id)?;
    p.write_byte(14)?;
    p.write_byte(0)?;
    p.write_int(set)?;
    p.write_str_with_length(menu)?;
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
    /// False hides the NPC: it is not drawn and takes no clicks.
    pub enabled: bool,
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
    packet.write_byte(if npc.enabled { 1 } else { 0 })?;
    Ok(())
}

/// Show the NPC with Effect/MapEff.img/NpcSummon, or hide it with NpcReturn,
/// for everyone on the map. Keep its spawn's `enabled` the same for those
/// who come later.
pub fn build_npc_enabled(object_id: i32, enabled: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NpcEnabled as i16)?;
    packet.write_int(object_id)?;
    packet.write_byte(if enabled { 1 } else { 0 })?;
    Ok(packet)
}

/// The NPC's controller asks for the action called `name` with an NPC_ACTION
/// request, unless the NPC is playing a special action or has no such action.
pub fn build_npc_special_action(object_id: i32, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NpcSpecialAction as i16)?;
    packet.write_int(object_id)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// A script an NPC runs from `start` to `end`, both YYYYMMDD and included.
pub struct NpcScriptWindow<'a> {
    pub npc_id: i32,
    pub script: &'a str,
    pub start: i32,
    pub end: i32,
}

/// Adds script windows to NPCs for the rest of the client's session. When
/// talked to, an NPC with a script open today asks the server; one without
/// shows its own String/Npc.img line. The newest open window wins.
pub fn build_set_npc_scriptable(windows: &[NpcScriptWindow]) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetNpcScriptable as i16)?;
    packet.write_byte(windows.len() as u8)?;
    for window in windows {
        packet.write_int(window.npc_id)?;
        packet.write_str_with_length(window.script)?;
        packet.write_int(window.start)?;
        packet.write_int(window.end)?;
    }
    Ok(packet)
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
            enabled: true,
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

    #[test]
    fn a_hidden_npc_spawns_with_zero_last() {
        let npc = ForeignNpc {
            enabled: false,
            ..foreign_npc(true)
        };
        let packet = build_spawn_npc(&npc).unwrap();

        assert_eq!(packet.bytes.last(), Some(&0));
    }

    #[test]
    fn npc_enabled_is_the_object_and_the_flag() {
        let packet = build_npc_enabled(1_000_000_001, false).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcEnabled as i16);
        expect.int(1_000_000_001).byte(0);
        expect.end();
    }

    #[test]
    fn npc_special_action_names_the_action() {
        let packet = build_npc_special_action(1_000_000_001, "act1").unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcSpecialAction as i16);
        expect.int(1_000_000_001).string("act1");
        expect.end();
    }

    #[test]
    fn set_npc_scriptable_lists_the_windows() {
        let windows = [
            NpcScriptWindow {
                npc_id: 9000021,
                script: "gaga",
                start: 20261201,
                end: 20261231,
            },
            NpcScriptWindow {
                npc_id: 2100,
                script: "sera",
                start: 19000101,
                end: 20790101,
            },
        ];
        let packet = build_set_npc_scriptable(&windows).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SetNpcScriptable as i16).byte(2);
        expect
            .int(9000021)
            .string("gaga")
            .int(20261201)
            .int(20261231);
        expect.int(2100).string("sera").int(19000101).int(20790101);
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

    #[test]
    fn prompt_slide_sends_the_art_set_before_the_menu() {
        let prompt = crate::dialogue::Prompt::Slide {
            npc_id: 9010022,
            set: 0,
            menu: "#1#Dojo#2#Carnival".to_string(),
        };
        let packet = build_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(9010022).byte(14).byte(0).int(0);
        expect.string("#1#Dojo#2#Carnival");
        expect.end();
    }

    #[test]
    fn prompt_style_lists_the_styles_after_the_text() {
        let prompt = crate::dialogue::Prompt::Style {
            npc_id: 1012103,
            text: "Pick one".to_string(),
            styles: vec![30060, 30140],
        };
        let packet = build_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(1012103).byte(7).byte(0);
        expect.string("Pick one");
        expect.byte(2).int(30060).int(30140);
        expect.end();
    }

    #[test]
    fn prompt_number_carries_the_default_and_range() {
        let prompt = crate::dialogue::Prompt::AskNumber {
            npc_id: 1022003,
            text: "How many?".to_string(),
            default: 1,
            min: 1,
            max: 100,
        };
        let packet = build_prompt(&prompt).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NpcDialogue as i16);
        expect.byte(4).int(1022003).byte(3).byte(0);
        expect.string("How many?");
        expect.int(1).int(1).int(100);
        expect.end();
    }
}
