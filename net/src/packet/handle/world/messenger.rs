use crate::handler::{
    HandlerResult, MessengerRequest, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::messenger::{
    MESSENGER_CHAT_MODE, MESSENGER_DECLINE_MODE, MESSENGER_INVITE_MODE, MESSENGER_LEAVE_MODE,
    MESSENGER_OPEN_MODE,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// The longest name a character can have.
const MAX_NAME_LEN: usize = 12;

pub struct MessengerHandler;

impl WorldHandler for MessengerHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            MESSENGER_OPEN_MODE => MessengerRequest::Open {
                messenger_id: reader.read_int()?,
                look: ctx.player.look(),
            },
            MESSENGER_LEAVE_MODE => MessengerRequest::Leave,
            MESSENGER_INVITE_MODE => {
                let target = reader.read_str_with_length()?;
                if !valid_name(&target) || target == ctx.player.name() {
                    return Ok(HandlerResult::empty());
                }
                MessengerRequest::Invite { target }
            }
            MESSENGER_DECLINE_MODE => {
                // The inviter, the decliner's own name, and whether the
                // Game Option turned the invitation away (FUN_00851478).
                let inviter = reader.read_str_with_length()?;
                reader.read_str_with_length()?;
                let blocking = reader.read_byte()? != 0;
                if !valid_name(&inviter) {
                    return Ok(HandlerResult::empty());
                }
                MessengerRequest::Decline { inviter, blocking }
            }
            MESSENGER_CHAT_MODE => {
                // The v83 client sends the whole line, "name : text"; only
                // accept lines that speak as this character.
                let line = reader.read_str_with_length()?;
                let prefix = format!("{} : ", ctx.player.name());
                if !line.starts_with(&prefix) || line.len() == prefix.len() {
                    return Ok(HandlerResult::empty());
                }
                MessengerRequest::Chat { line }
            }
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Messenger(request)))
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= MAX_NAME_LEN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use game::{CharacterStats, Inventory, Keymap, Player, QuestLog, SkillBook};
    use packet::io::write::PktWrite;

    fn player() -> Player {
        Player::new(
            1,
            0,
            CharacterStats {
                id: 1,
                name: "Alice".into(),
                gender: 1,
                skin: 0,
                face: 21000,
                hair: 31000,
                level: 10,
                job: 0,
                str: 4,
                dex: 4,
                int: 4,
                luk: 4,
                hp: 50,
                max_hp: 50,
                mp: 5,
                max_mp: 5,
                ap: 0,
                sp: 0,
                exp: 0,
                gacha_exp: 0,
                fame: 0,
                map_id: 100000000,
                spawn_point: 0,
            },
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::default(),
            QuestLog::default(),
        )
    }

    fn handle(mode: u8, body: impl FnOnce(&mut Packet)) -> Option<MessengerRequest> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::Messenger as i16).unwrap();
        packet.write_byte(mode).unwrap();
        body(&mut packet);
        let mut player = player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let mut actions = MessengerHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        match actions.pop() {
            Some(WorldAction::Messenger(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected action {other:?}"),
        }
    }

    #[test]
    fn open_carries_the_messenger_id_and_the_players_look() {
        let request = handle(MESSENGER_OPEN_MODE, |p| {
            p.write_int(7).unwrap();
        });
        let Some(MessengerRequest::Open { messenger_id, look }) = request else {
            panic!("expected open, got {request:?}");
        };
        assert_eq!(messenger_id, 7);
        assert_eq!(look, player().look());
    }

    #[test]
    fn chat_must_speak_as_the_sender() {
        let line = |text: &'static str| {
            move |p: &mut Packet| {
                p.write_str_with_length(text).unwrap();
            }
        };
        assert!(matches!(
            handle(MESSENGER_CHAT_MODE, line("Alice : hi")),
            Some(MessengerRequest::Chat { line }) if line == "Alice : hi"
        ));
        assert!(handle(MESSENGER_CHAT_MODE, line("Bob : hi")).is_none());
        assert!(handle(MESSENGER_CHAT_MODE, line("Alice : ")).is_none());
        assert!(handle(MESSENGER_CHAT_MODE, line("hi")).is_none());
    }

    #[test]
    fn players_cannot_invite_themselves() {
        let name = |text: &'static str| {
            move |p: &mut Packet| {
                p.write_str_with_length(text).unwrap();
            }
        };
        assert!(handle(MESSENGER_INVITE_MODE, name("Alice")).is_none());
        assert!(handle(MESSENGER_INVITE_MODE, name("")).is_none());
        assert!(matches!(
            handle(MESSENGER_INVITE_MODE, name("Bob")),
            Some(MessengerRequest::Invite { target }) if target == "Bob"
        ));
    }

    #[test]
    fn a_decline_says_whether_the_options_blocked_it() {
        let decline = |flag: u8| {
            move |p: &mut Packet| {
                p.write_str_with_length("Bob").unwrap();
                p.write_str_with_length("Alice").unwrap();
                p.write_byte(flag).unwrap();
            }
        };
        assert!(matches!(
            handle(MESSENGER_DECLINE_MODE, decline(0)),
            Some(MessengerRequest::Decline { inviter, blocking: false }) if inviter == "Bob"
        ));
        assert!(matches!(
            handle(MESSENGER_DECLINE_MODE, decline(1)),
            Some(MessengerRequest::Decline { blocking: true, .. })
        ));
    }
}
