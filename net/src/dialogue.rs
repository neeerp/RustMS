//! Conversation progression independent of transport and asset storage.
//!
//! Quests are not conversations on the server: the client renders their
//! pages and sends `QUEST_ACTION` (see `crate::quests`). Closing or finishing
//! these normal speech pages has no persistence side effects.

use game_data::NpcDialogueTemplate;

#[derive(Clone, Debug)]
pub enum NpcInteraction {
    Start {
        object_id: i32,
        request_id: u64,
    },
    Respond {
        message_type: u8,
        response: i8,
        selection: Option<i32>,
        /// What the player typed into a text prompt.
        text: Option<String>,
    },
    /// A Water of Life was used: Mar the Fairy offers to revive a doll.
    WaterOfLife,
    /// A Remote Gachapon Ticket was used on the gachapon `machine`.
    RemoteGachapon {
        machine: i32,
    },
    /// A Noblesse double-clicked their guide: Mimo's menu opens.
    Mimo,
}

/// One server-driven NPC prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prompt {
    Text(DialoguePage),
    YesNo {
        npc_id: i32,
        text: String,
    },
    /// A menu; choices are embedded in `text` as `#L<id>#label#l`.
    Selection {
        npc_id: i32,
        text: String,
    },
    /// Pick one of `styles` (hair, face or skin) on a preview of the
    /// player; the answer is the index of the chosen one.
    Style {
        npc_id: i32,
        text: String,
        styles: Vec<i32>,
    },
    /// A line of text, `min` to `max` bytes long.
    AskText {
        npc_id: i32,
        text: String,
        min: i16,
        max: i16,
    },
    /// A number from `min` to `max`, starting at `default`.
    AskNumber {
        npc_id: i32,
        text: String,
        default: i32,
        min: i32,
        max: i32,
    },
    /// A slide menu, as the Dimensional Mirror's: an icon per choice,
    /// `UIWindow.img/SlideMenu/<set>/BtMain/<id>`, with its label under
    /// it. `menu` holds `#<id>#<label>` pairs, and the answer is an id.
    Slide {
        npc_id: i32,
        set: i32,
        menu: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialoguePage {
    pub npc_id: i32,
    pub text: String,
    pub has_previous: bool,
    pub has_next: bool,
    /// Said by the player rather than the NPC (speaker bit 0x2), as a
    /// thought.
    pub player_speaks: bool,
}

#[derive(Debug)]
pub struct Conversation {
    definition: NpcDialogueTemplate,
    page: usize,
}

impl Conversation {
    pub fn from_pages(npc_id: i32, pages: &[String]) -> Option<Self> {
        Self::start(&NpcDialogueTemplate {
            npc_id,
            pages: pages.to_vec(),
        })
    }
    pub fn start(definition: &NpcDialogueTemplate) -> Option<Self> {
        if definition.pages.is_empty() {
            return None;
        }
        Some(Self {
            definition: definition.clone(),
            page: 0,
        })
    }

    pub fn current_page(&self) -> DialoguePage {
        DialoguePage {
            npc_id: self.definition.npc_id,
            text: self.definition.pages[self.page].clone(),
            has_previous: self.page > 0,
            has_next: self.page + 1 < self.definition.pages.len(),
            player_speaks: false,
        }
    }

    /// None ends the conversation. Unexpected response types also invalidate it
    /// rather than allowing a stale/forged response to advance server state.
    pub fn respond(&mut self, message_type: u8, response: i8) -> Option<DialoguePage> {
        if message_type != 0 {
            return None;
        }
        match response {
            0 if self.page > 0 => self.page -= 1,
            1 if self.page + 1 < self.definition.pages.len() => self.page += 1,
            _ => return None,
        }
        Some(self.current_page())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conversation() -> Conversation {
        Conversation::start(&NpcDialogueTemplate {
            npc_id: 12101,
            pages: vec!["first".into(), "second".into(), "last".into()],
        })
        .unwrap()
    }

    #[test]
    fn pages_can_advance_go_back_and_finish() {
        let mut c = conversation();
        assert!(!c.current_page().has_previous);
        assert_eq!(c.respond(0, 1).unwrap().text, "second");
        assert_eq!(c.respond(0, 0).unwrap().text, "first");
        c.respond(0, 1).unwrap();
        let last = c.respond(0, 1).unwrap();
        assert!(last.has_previous);
        assert!(!last.has_next);
        assert!(c.respond(0, 1).is_none());
    }

    #[test]
    fn cancel_and_unexpected_responses_end_conversation() {
        assert!(conversation().respond(0, -1).is_none());
        assert!(conversation().respond(1, 1).is_none());
        assert!(conversation().respond(0, 2).is_none());
        assert!(conversation().respond(0, 0).is_none());
    }
}
