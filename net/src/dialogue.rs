//! Conversation progression independent of transport and asset storage.
//!
//! Quest-aware conversations live in `crate::quests`. Closing or finishing
//! these normal speech pages has no persistence side effects.

use game_data::NpcDialogueTemplate;

#[derive(Clone, Copy, Debug)]
pub enum NpcInteraction {
    Start {
        object_id: i32,
        request_id: u64,
    },
    Respond {
        message_type: u8,
        response: i8,
        selection: Option<i32>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialoguePage {
    pub npc_id: i32,
    pub text: String,
    pub has_previous: bool,
    pub has_next: bool,
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
