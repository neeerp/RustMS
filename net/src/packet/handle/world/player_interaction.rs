use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::hired_merchant::{HiredMerchantRequest, ROOM_TYPE as HIRED_MERCHANT};
use crate::mini_game::{MiniGameRequest, MATCH_CARDS, OMOK};
use crate::packet::build::world::mini_room::{PERSONAL_SHOP, TRADE_ROOM};
use crate::personal_shop::PersonalShopRequest;
use crate::trade::TradeRequest;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `PLAYER_INTERACTION`, for trading rooms, personal shops and the games
/// Omok and match cards. Mode 0
/// creates a room of a type; 4 visits (accepting a trade invitation or
/// entering a shop), 6 chats and 0xa leaves either. For trades, 2 invites, 3
/// declines, 0xf puts up an item, 0x10 mesos, and 0x11 presses Trade. For
/// shops, 0x16 lists an item, 0x1e sends the blacklist and 0x1f/0x20 change
/// it, 0xb opens the shop, 0x17 buys, 0x1b takes a listing back, and
/// 0x1c/0x1d ban a visitor. A hired merchant's modes are the shop's plus
/// 0xb (0x21 lists, 0x22 buys, 0x26 takes back), and its owner's buttons
/// send 0x27 to 0x31 (report 165 §3). The games send 0x32 to 0x41 and
/// 0x44 (report 164 §4–5); their owner also sends 0xb on entering, which
/// opens no shop. Mode 0xe wraps an open or a visit
/// with the birthday, when a cash item is involved. The runtime holds the
/// goods and the field the room.
pub struct PlayerInteractionHandler;

impl WorldHandler for PlayerInteractionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let shop = |request| Ok(HandlerResult::empty().with(WorldAction::PersonalShop(request)));
        let merchant =
            |request| Ok(HandlerResult::empty().with(WorldAction::HiredMerchant(request)));
        let game = |request| Ok(HandlerResult::empty().with(WorldAction::MiniGame(request)));
        let request = match reader.read_byte()? {
            0x00 => match reader.read_byte()? {
                TRADE_ROOM => TradeRequest::Create,
                PERSONAL_SHOP => {
                    let title = reader.read_str_with_length()?;
                    reader.read_byte()?;
                    return shop(PersonalShopRequest::Create {
                        title,
                        permit_slot: reader.read_short()?,
                        permit_id: reader.read_int()?,
                    });
                }
                HIRED_MERCHANT => {
                    let title = reader.read_str_with_length()?;
                    reader.read_byte()?;
                    return merchant(HiredMerchantRequest::Create {
                        title,
                        permit_slot: reader.read_short()?,
                        permit_id: reader.read_int()?,
                    });
                }
                kind @ (OMOK | MATCH_CARDS) => {
                    let title = reader.read_str_with_length()?;
                    let password = if reader.read_byte()? != 0 {
                        Some(reader.read_str_with_length()?)
                    } else {
                        None
                    };
                    return game(MiniGameRequest::Create {
                        kind,
                        title,
                        password,
                        piece: reader.read_byte()?,
                    });
                }
                kind => {
                    debug!(kind, "Ignoring mini room of another type");
                    return Ok(HandlerResult::empty());
                }
            },
            0x02 => TradeRequest::Invite {
                target: reader.read_int()?,
            },
            0x03 => TradeRequest::Decline {
                sn: reader.read_int()?,
                reason: reader.read_byte()?,
            },
            0x04 => {
                let sn = reader.read_int()?;
                let password = if reader.read_byte().unwrap_or(0) != 0 {
                    Some(reader.read_str_with_length()?)
                } else {
                    None
                };
                TradeRequest::Visit { sn, password }
            }
            0x06 => TradeRequest::Chat {
                text: reader.read_str_with_length()?,
            },
            0x0a => TradeRequest::Leave,
            0x0f => TradeRequest::PutItem {
                kind: i16::from(reader.read_byte()?),
                slot: reader.read_short()?,
                count: reader.read_short()?,
                trade_slot: reader.read_byte()?,
            },
            0x10 => TradeRequest::Meso(reader.read_int()?),
            0x11 => TradeRequest::Confirm,
            0x0b => return shop(PersonalShopRequest::Open),
            // The client asks for the birthday first when a cash item is
            // involved: opening with one listed, or a remote visit
            // (0x6fcb17, 0xa28074).
            0x0e => {
                let mode = reader.read_byte()?;
                reader.read_byte()?;
                let birthday = reader.read_int()?;
                if !crate::cash_shop::birthday_code(birthday) {
                    debug!(birthday, "Ignoring a room request with a bad birthday");
                    return Ok(HandlerResult::empty());
                }
                match mode {
                    0x0b => return shop(PersonalShopRequest::Open),
                    0x04 => {
                        let sn = reader.read_int()?;
                        if reader.read_byte()? == 0 {
                            TradeRequest::Visit { sn, password: None }
                        } else {
                            return merchant(HiredMerchantRequest::RemoteVisit {
                                sn,
                                slot: reader.read_short()?,
                                cash_sn: reader.read_long()?,
                            });
                        }
                    }
                    mode => {
                        debug!(mode, "Ignoring a birthday-checked room request");
                        return Ok(HandlerResult::empty());
                    }
                }
            }
            0x16 => {
                return shop(PersonalShopRequest::PutItem {
                    kind: i16::from(reader.read_byte()?),
                    slot: reader.read_short()?,
                    bundles: reader.read_short()?,
                    per_bundle: reader.read_short()?,
                    price: reader.read_int()?,
                })
            }
            // The item's data checksum follows.
            0x17 => {
                return shop(PersonalShopRequest::Buy {
                    index: reader.read_byte()?,
                    bundles: reader.read_short()?,
                })
            }
            0x1b => {
                return shop(PersonalShopRequest::TakeBack {
                    index: reader.read_short()?,
                })
            }
            mode @ (0x1c | 0x1d) => {
                return shop(PersonalShopRequest::Ban {
                    slot: reader.read_byte()?,
                    name: reader.read_str_with_length()?,
                    overstayed: mode == 0x1d,
                })
            }
            0x1e => {
                let count = reader.read_short()?;
                let names = (0..count)
                    .map(|_| reader.read_str_with_length())
                    .collect::<Result<_, _>>()?;
                return shop(PersonalShopRequest::Blacklist(names));
            }
            0x1f => {
                return shop(PersonalShopRequest::BlacklistAdd(
                    reader.read_str_with_length()?,
                ))
            }
            0x20 => {
                return shop(PersonalShopRequest::BlacklistRemove(
                    reader.read_str_with_length()?,
                ))
            }
            0x21 => {
                return merchant(HiredMerchantRequest::PutItem {
                    kind: i16::from(reader.read_byte()?),
                    slot: reader.read_short()?,
                    bundles: reader.read_short()?,
                    per_bundle: reader.read_short()?,
                    price: reader.read_int()?,
                })
            }
            // The item's data checksum follows.
            0x22 => {
                return merchant(HiredMerchantRequest::Buy {
                    index: reader.read_byte()?,
                    bundles: reader.read_short()?,
                })
            }
            0x26 => {
                return merchant(HiredMerchantRequest::TakeBack {
                    index: reader.read_short()?,
                })
            }
            0x27 => return merchant(HiredMerchantRequest::EndMaintenance),
            0x28 => return merchant(HiredMerchantRequest::Organize),
            0x29 => return merchant(HiredMerchantRequest::Close),
            0x2b => return merchant(HiredMerchantRequest::TakeMeso),
            0x2e => return merchant(HiredMerchantRequest::Visitors),
            0x2f => return merchant(HiredMerchantRequest::Blacklist),
            0x30 => {
                return merchant(HiredMerchantRequest::BlacklistAdd(
                    reader.read_str_with_length()?,
                ))
            }
            0x31 => {
                return merchant(HiredMerchantRequest::BlacklistRemove(
                    reader.read_str_with_length()?,
                ))
            }
            0x32 => return game(MiniGameRequest::AskTie),
            0x33 => return game(MiniGameRequest::AnswerTie(reader.read_byte()? != 0)),
            0x34 => return game(MiniGameRequest::GiveUp),
            0x36 => return game(MiniGameRequest::AskUndo),
            0x37 => return game(MiniGameRequest::AnswerUndo(reader.read_byte()? != 0)),
            0x38 => return game(MiniGameRequest::LeaveAfter(true)),
            0x39 => return game(MiniGameRequest::LeaveAfter(false)),
            0x3a => return game(MiniGameRequest::Ready(true)),
            0x3b => return game(MiniGameRequest::Ready(false)),
            0x3c => return game(MiniGameRequest::Expel),
            0x3d => return game(MiniGameRequest::Start),
            0x3f => return game(MiniGameRequest::Skip),
            0x40 => {
                return game(MiniGameRequest::Move {
                    x: reader.read_int()?,
                    y: reader.read_int()?,
                    color: reader.read_byte()?,
                })
            }
            0x44 => {
                return game(MiniGameRequest::Flip {
                    first: reader.read_byte()? != 0,
                    index: reader.read_byte()?,
                })
            }
            other => {
                // 0x14 answers the partner's Trade press with item CRCs;
                // 0x2d renames a merchant, for GMs.
                debug!(mode = other, "Ignoring player interaction");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Trade(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn parse<T>(build: impl FnOnce(&mut Packet) -> T) -> Option<TradeRequest> {
        match action(build) {
            Some(WorldAction::Trade(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    fn parse_shop<T>(build: impl FnOnce(&mut Packet) -> T) -> Option<PersonalShopRequest> {
        match action(build) {
            Some(WorldAction::PersonalShop(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    fn parse_game<T>(build: impl FnOnce(&mut Packet) -> T) -> Option<MiniGameRequest> {
        match action(build) {
            Some(WorldAction::MiniGame(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reads_the_game_modes() {
        let omok = parse_game(|p| {
            p.write_bytes(&[0, 1]).unwrap();
            p.write_str_with_length("omok").unwrap();
            p.write_byte(1).unwrap();
            p.write_str_with_length("pw").unwrap();
            p.write_byte(7).unwrap();
        });
        assert_eq!(
            omok,
            Some(MiniGameRequest::Create {
                kind: 1,
                title: "omok".into(),
                password: Some("pw".into()),
                piece: 7
            })
        );
        let cards = parse_game(|p| {
            p.write_bytes(&[0, 2]).unwrap();
            p.write_str_with_length("cards").unwrap();
            p.write_bytes(&[0, 2]).unwrap();
        });
        assert_eq!(
            cards,
            Some(MiniGameRequest::Create {
                kind: 2,
                title: "cards".into(),
                password: None,
                piece: 2
            })
        );
        for (bytes, request) in [
            (&[0x32][..], MiniGameRequest::AskTie),
            (&[0x33, 1], MiniGameRequest::AnswerTie(true)),
            (&[0x34], MiniGameRequest::GiveUp),
            (&[0x36], MiniGameRequest::AskUndo),
            (&[0x37, 0], MiniGameRequest::AnswerUndo(false)),
            (&[0x38], MiniGameRequest::LeaveAfter(true)),
            (&[0x39], MiniGameRequest::LeaveAfter(false)),
            (&[0x3a], MiniGameRequest::Ready(true)),
            (&[0x3b], MiniGameRequest::Ready(false)),
            (&[0x3c], MiniGameRequest::Expel),
            (&[0x3d], MiniGameRequest::Start),
            (&[0x3f], MiniGameRequest::Skip),
            (
                &[0x44, 1, 9],
                MiniGameRequest::Flip {
                    first: true,
                    index: 9,
                },
            ),
        ] {
            assert_eq!(parse_game(|p| p.write_bytes(bytes).unwrap()), Some(request));
        }
        let stone = parse_game(|p| {
            p.write_byte(0x40).unwrap();
            p.write_int(3).unwrap();
            p.write_int(14).unwrap();
            p.write_byte(2).unwrap();
        });
        assert_eq!(
            stone,
            Some(MiniGameRequest::Move {
                x: 3,
                y: 14,
                color: 2
            })
        );
    }

    fn parse_merchant(build: &dyn Fn(&mut Packet)) -> Option<HiredMerchantRequest> {
        match action(build) {
            Some(WorldAction::HiredMerchant(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    fn action<T>(build: impl FnOnce(&mut Packet) -> T) -> Option<WorldAction> {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::PlayerInteraction as i16)
            .unwrap();
        build(&mut packet);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = PlayerInteractionHandler
            .handle(&mut packet, &mut ctx)
            .unwrap();
        result.actions.into_iter().next()
    }

    #[test]
    fn reads_the_room_modes() {
        let create = parse(|p| p.write_bytes(&[0, 3, 0]).unwrap());
        assert_eq!(create, Some(TradeRequest::Create));
        let invite = parse(|p| {
            p.write_byte(2).unwrap();
            p.write_int(7).unwrap();
        });
        assert_eq!(invite, Some(TradeRequest::Invite { target: 7 }));
        let decline = parse(|p| {
            p.write_byte(3).unwrap();
            p.write_int(5).unwrap();
            p.write_byte(4).unwrap();
        });
        assert_eq!(decline, Some(TradeRequest::Decline { sn: 5, reason: 4 }));
        let visit = parse(|p| {
            p.write_byte(4).unwrap();
            p.write_int(5).unwrap();
            p.write_bytes(&[0, 0]).unwrap();
        });
        assert_eq!(
            visit,
            Some(TradeRequest::Visit {
                sn: 5,
                password: None
            })
        );
        let private = parse(|p| {
            p.write_byte(4).unwrap();
            p.write_int(5).unwrap();
            p.write_byte(1).unwrap();
            p.write_str_with_length("pw").unwrap();
            p.write_byte(0).unwrap();
        });
        assert_eq!(
            private,
            Some(TradeRequest::Visit {
                sn: 5,
                password: Some("pw".into())
            })
        );
        let chat = parse(|p| {
            p.write_byte(6).unwrap();
            p.write_str_with_length("hi").unwrap();
        });
        assert_eq!(
            chat,
            Some(TradeRequest::Chat {
                text: "hi".to_string()
            })
        );
        assert_eq!(parse(|p| p.write_byte(0x0a)), Some(TradeRequest::Leave));
    }

    #[test]
    fn reads_the_trade_modes() {
        let put = parse(|p| {
            p.write_byte(0x0f).unwrap();
            p.write_byte(2).unwrap();
            p.write_short(4).unwrap();
            p.write_short(10).unwrap();
            p.write_byte(1).unwrap();
        });
        assert_eq!(
            put,
            Some(TradeRequest::PutItem {
                kind: 2,
                slot: 4,
                count: 10,
                trade_slot: 1
            })
        );
        let meso = parse(|p| {
            p.write_byte(0x10).unwrap();
            p.write_int(5000).unwrap();
        });
        assert_eq!(meso, Some(TradeRequest::Meso(5000)));
        let confirm = parse(|p| {
            p.write_byte(0x11).unwrap();
            p.write_byte(0).unwrap();
        });
        assert_eq!(confirm, Some(TradeRequest::Confirm));
    }

    #[test]
    fn reads_the_shop_modes() {
        let create = parse_shop(|p| {
            p.write_bytes(&[0, 4]).unwrap();
            p.write_str_with_length("Pots!").unwrap();
            p.write_byte(0).unwrap();
            p.write_short(3).unwrap();
            p.write_int(5140000).unwrap();
        });
        assert_eq!(
            create,
            Some(PersonalShopRequest::Create {
                title: "Pots!".into(),
                permit_slot: 3,
                permit_id: 5140000
            })
        );
        let put = parse_shop(|p| {
            p.write_bytes(&[0x16, 2]).unwrap();
            p.write_short(4).unwrap();
            p.write_short(3).unwrap();
            p.write_short(10).unwrap();
            p.write_int(500).unwrap();
        });
        assert_eq!(
            put,
            Some(PersonalShopRequest::PutItem {
                kind: 2,
                slot: 4,
                bundles: 3,
                per_bundle: 10,
                price: 500
            })
        );
        let blacklist = parse_shop(|p| {
            p.write_byte(0x1e).unwrap();
            p.write_short(2).unwrap();
            p.write_str_with_length("Alice").unwrap();
            p.write_str_with_length("Bobby").unwrap();
        });
        assert_eq!(
            blacklist,
            Some(PersonalShopRequest::Blacklist(vec![
                "Alice".into(),
                "Bobby".into()
            ]))
        );
        let add = parse_shop(|p| {
            p.write_byte(0x1f).unwrap();
            p.write_str_with_length("Carol").unwrap();
        });
        assert_eq!(add, Some(PersonalShopRequest::BlacklistAdd("Carol".into())));
        let remove = parse_shop(|p| {
            p.write_byte(0x20).unwrap();
            p.write_str_with_length("Carol").unwrap();
        });
        assert_eq!(
            remove,
            Some(PersonalShopRequest::BlacklistRemove("Carol".into()))
        );
        assert_eq!(
            parse_shop(|p| p.write_bytes(&[0x0b, 1])),
            Some(PersonalShopRequest::Open)
        );
        let buy = parse_shop(|p| {
            p.write_bytes(&[0x17, 2]).unwrap();
            p.write_short(5).unwrap();
            p.write_int(0x1234).unwrap();
        });
        assert_eq!(
            buy,
            Some(PersonalShopRequest::Buy {
                index: 2,
                bundles: 5
            })
        );
        let take = parse_shop(|p| {
            p.write_byte(0x1b).unwrap();
            p.write_short(1).unwrap();
        });
        assert_eq!(take, Some(PersonalShopRequest::TakeBack { index: 1 }));
        for (mode, overstayed) in [(0x1c, false), (0x1d, true)] {
            let ban = parse_shop(|p| {
                p.write_bytes(&[mode, 2]).unwrap();
                p.write_str_with_length("Bobby").unwrap();
            });
            assert_eq!(
                ban,
                Some(PersonalShopRequest::Ban {
                    slot: 2,
                    name: "Bobby".into(),
                    overstayed
                })
            );
        }
    }

    #[test]
    fn reads_the_merchant_modes() {
        let create = parse_merchant(&|p| {
            p.write_bytes(&[0, 5]).unwrap();
            p.write_str_with_length("Scrolls").unwrap();
            p.write_byte(0).unwrap();
            p.write_short(2).unwrap();
            p.write_int(5030000).unwrap();
        });
        assert_eq!(
            create,
            Some(HiredMerchantRequest::Create {
                title: "Scrolls".into(),
                permit_slot: 2,
                permit_id: 5030000
            })
        );
        let put = parse_merchant(&|p| {
            p.write_bytes(&[0x21, 4]).unwrap();
            p.write_short(7).unwrap();
            p.write_short(2).unwrap();
            p.write_short(50).unwrap();
            p.write_int(900).unwrap();
        });
        assert_eq!(
            put,
            Some(HiredMerchantRequest::PutItem {
                kind: 4,
                slot: 7,
                bundles: 2,
                per_bundle: 50,
                price: 900
            })
        );
        let buy = parse_merchant(&|p| {
            p.write_bytes(&[0x22, 1]).unwrap();
            p.write_short(3).unwrap();
            p.write_int(0x1234).unwrap();
        });
        assert_eq!(
            buy,
            Some(HiredMerchantRequest::Buy {
                index: 1,
                bundles: 3
            })
        );
        let take = parse_merchant(&|p| {
            p.write_byte(0x26).unwrap();
            p.write_short(4).unwrap();
        });
        assert_eq!(take, Some(HiredMerchantRequest::TakeBack { index: 4 }));
        for (mode, request) in [
            (0x27, HiredMerchantRequest::EndMaintenance),
            (0x28, HiredMerchantRequest::Organize),
            (0x29, HiredMerchantRequest::Close),
            (0x2b, HiredMerchantRequest::TakeMeso),
            (0x2e, HiredMerchantRequest::Visitors),
            (0x2f, HiredMerchantRequest::Blacklist),
        ] {
            assert_eq!(
                parse_merchant(&|p| {
                    p.write_byte(mode).unwrap();
                }),
                Some(request)
            );
        }
        let add = parse_merchant(&|p| {
            p.write_byte(0x30).unwrap();
            p.write_str_with_length("Dave").unwrap();
        });
        assert_eq!(add, Some(HiredMerchantRequest::BlacklistAdd("Dave".into())));
        let remove = parse_merchant(&|p| {
            p.write_byte(0x31).unwrap();
            p.write_str_with_length("Dave").unwrap();
        });
        assert_eq!(
            remove,
            Some(HiredMerchantRequest::BlacklistRemove("Dave".into()))
        );
    }

    #[test]
    fn reads_the_birthday_checked_open_and_remote_visit() {
        let open = parse_shop(|p| {
            p.write_bytes(&[0x0e, 0x0b, 5]).unwrap();
            p.write_int(19830113).unwrap();
        });
        assert_eq!(open, Some(PersonalShopRequest::Open));
        let remote = parse_merchant(&|p| {
            p.write_bytes(&[0x0e, 0x04, 5]).unwrap();
            p.write_int(19830113).unwrap();
            p.write_int(0x4000_0001).unwrap();
            p.write_byte(1).unwrap();
            p.write_short(3).unwrap();
            p.write_long(77).unwrap();
        });
        assert_eq!(
            remote,
            Some(HiredMerchantRequest::RemoteVisit {
                sn: 0x4000_0001,
                slot: 3,
                cash_sn: 77
            })
        );
        let bad_birthday = action(|p| {
            p.write_bytes(&[0x0e, 0x0b, 5]).unwrap();
            p.write_int(0).unwrap();
        });
        assert!(bad_birthday.is_none());
    }

    #[test]
    fn ignores_other_rooms_and_the_crc_answer() {
        assert_eq!(parse(|p| p.write_bytes(&[0, 6, 0])), None);
        assert_eq!(parse(|p| p.write_bytes(&[0x14, 0])), None);
        assert_eq!(parse(|p| p.write_byte(0x2d)), None);
    }
}
