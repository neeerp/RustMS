//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "../OutPacket.h"

namespace ms
{
	// Mini room (trading room and personal shop) requests. Every one starts
	// with a mode byte.
	// Opcode: PLAYER_INTERACTION(123)
	class MiniRoomPacket : public OutPacket
	{
	public:
		// v83's reasons for turning an invite down
		enum Decline : int8_t
		{
			DECLINED = 3,
			NOT_ACCEPTING = 4
		};

	protected:
		enum Mode : int8_t
		{
			CREATE = 0x00,
			INVITE = 0x02,
			DECLINE = 0x03,
			VISIT = 0x04,
			CHAT = 0x06,
			LEAVE = 0x0A,
			OPEN = 0x0B,
			BIRTHDAY = 0x0E,
			PUT_ITEM = 0x0F,
			PUT_MESO = 0x10,
			TRADE = 0x11,
			TRADE_REPLY = 0x14,
			SHOP_PUT_ITEM = 0x16,
			SHOP_BUY = 0x17,
			SHOP_TAKE_BACK = 0x1B,
			SHOP_BAN = 0x1C,
			SHOP_BLACKLIST = 0x1E,
			SHOP_BLACKLIST_ADD = 0x1F,
			SHOP_BLACKLIST_REMOVE = 0x20
		};

		MiniRoomPacket(Mode mode) : OutPacket(OutPacket::Opcode::PLAYER_INTERACTION)
		{
			write_byte(mode);
		}

		// A hired merchant's put, buy and take back are the personal shop's
		// modes plus 0xb (vt+0x5c).
		static Mode shop_mode(Mode mode, bool merchant)
		{
			return merchant ? static_cast<Mode>(mode + 0x0B) : mode;
		}

		// The item ids of an offer, each with the item's template CRC. HC
		// can't compute the CRC (v83 FUN_005dacd3) and sends 0.
		void write_items(const std::vector<int32_t>& itemids)
		{
			write_byte(static_cast<int8_t>(itemids.size()));

			for (int32_t itemid : itemids)
			{
				write_int(itemid);
				write_int(0);
			}
		}
	};

	// Opens a trading room (type 3). Sent right before the invite (v83 FUN_0052fad4).
	class TradeCreatePacket : public MiniRoomPacket
	{
	public:
		static constexpr int8_t TRADING_ROOM = 3;

		TradeCreatePacket() : MiniRoomPacket(Mode::CREATE)
		{
			write_byte(TRADING_ROOM);
			write_byte(0);
		}
	};

	class MiniRoomInvitePacket : public MiniRoomPacket
	{
	public:
		MiniRoomInvitePacket(int32_t character_id) : MiniRoomPacket(Mode::INVITE)
		{
			write_int(character_id);
		}
	};

	class MiniRoomDeclinePacket : public MiniRoomPacket
	{
	public:
		MiniRoomDeclinePacket(int32_t serial, Decline reason) : MiniRoomPacket(Mode::DECLINE)
		{
			write_int(serial);
			write_byte(reason);
		}
	};

	// Joins the room we were invited to, or a locked game with its
	// password (0x94fbbf).
	class MiniRoomVisitPacket : public MiniRoomPacket
	{
	public:
		MiniRoomVisitPacket(int32_t serial) : MiniRoomPacket(Mode::VISIT)
		{
			write_int(serial);
			write_byte(0);
			write_byte(0);
		}

		MiniRoomVisitPacket(int32_t serial, const std::string& password) : MiniRoomPacket(Mode::VISIT)
		{
			write_int(serial);
			write_byte(1);
			write_string(password);
			write_byte(0);
		}
	};

	// The server adds the speaker's name.
	class MiniRoomChatPacket : public MiniRoomPacket
	{
	public:
		MiniRoomChatPacket(const std::string& text) : MiniRoomPacket(Mode::CHAT)
		{
			write_string(text);
		}
	};

	class MiniRoomLeavePacket : public MiniRoomPacket
	{
	public:
		MiniRoomLeavePacket() : MiniRoomPacket(Mode::LEAVE) {}
	};

	// Offers an item from the inventory in one of my nine trade slots (1-based).
	class TradePutItemPacket : public MiniRoomPacket
	{
	public:
		TradePutItemPacket(int8_t tab, int16_t slot, int16_t count, int8_t trade_slot) : MiniRoomPacket(Mode::PUT_ITEM)
		{
			write_byte(tab);
			write_short(slot);
			write_short(count);
			write_byte(trade_slot);
		}
	};

	// Adds mesos to my offer.
	class TradePutMesoPacket : public MiniRoomPacket
	{
	public:
		TradePutMesoPacket(int32_t meso) : MiniRoomPacket(Mode::PUT_MESO)
		{
			write_int(meso);
		}
	};

	// The Trade button, listing the items of both sides.
	class TradeConfirmPacket : public MiniRoomPacket
	{
	public:
		TradeConfirmPacket(const std::vector<int32_t>& itemids) : MiniRoomPacket(Mode::TRADE)
		{
			write_items(itemids);
		}
	};

	// The answer to the partner pressing Trade, listing the partner's items.
	class TradeConfirmReplyPacket : public MiniRoomPacket
	{
	public:
		TradeConfirmReplyPacket(const std::vector<int32_t>& itemids) : MiniRoomPacket(Mode::TRADE_REPLY)
		{
			write_items(itemids);
		}
	};

	// Opens a personal shop (type 4) or a hired merchant (type 5) with a
	// permit from the cash tab (v83 FUN_00a1dd99). The shop stays closed to
	// visitors until it opens.
	class ShopCreatePacket : public MiniRoomPacket
	{
	public:
		static constexpr int8_t PERSONAL_SHOP = 4;
		static constexpr int8_t HIRED_MERCHANT = 5;

		ShopCreatePacket(const std::string &title, int16_t permit_slot, int32_t permit_id, bool merchant)
			: MiniRoomPacket(Mode::CREATE)
		{
			write_byte(merchant ? HIRED_MERCHANT : PERSONAL_SHOP);
			write_string(title);
			write_byte(0);
			write_short(permit_slot);
			write_int(permit_id);
		}
	};

	// Lists bundles of an inventory item in my shop, at a price a bundle.
	class ShopPutItemPacket : public MiniRoomPacket
	{
	public:
		ShopPutItemPacket(int8_t tab, int16_t slot, int16_t bundles, int16_t per_bundle, int32_t price, bool merchant)
			: MiniRoomPacket(shop_mode(Mode::SHOP_PUT_ITEM, merchant))
		{
			write_byte(tab);
			write_short(slot);
			write_short(bundles);
			write_short(per_bundle);
			write_int(price);
		}
	};

	// Buys bundles of a shop's row. Like the trade, HC sends 0 for the
	// item's CRC.
	class ShopBuyPacket : public MiniRoomPacket
	{
	public:
		ShopBuyPacket(uint8_t index, int16_t bundles, bool merchant)
			: MiniRoomPacket(shop_mode(Mode::SHOP_BUY, merchant))
		{
			write_byte(static_cast<int8_t>(index));
			write_short(bundles);
			write_int(0);
		}
	};

	// Takes a row of my shop back into the inventory, before it opens (or,
	// for a hired merchant, in maintenance).
	class ShopTakeBackPacket : public MiniRoomPacket
	{
	public:
		ShopTakeBackPacket(int16_t index, bool merchant) : MiniRoomPacket(shop_mode(Mode::SHOP_TAKE_BACK, merchant))
		{
			write_short(index);
		}
	};

	// The names my shop turns away: the User List's Blacklist, in the
	// order added, sent right before opening even when empty (0x6fdeda).
	class ShopBlacklistPacket : public MiniRoomPacket
	{
	public:
		ShopBlacklistPacket(const std::vector<std::string> &names) : MiniRoomPacket(Mode::SHOP_BLACKLIST)
		{
			write_short(static_cast<int16_t>(names.size()));

			for (const std::string &name : names)
				write_string(name);
		}
	};

	// A name added to or deleted from the Blacklist while my personal shop
	// is open (0x53792e, 0x5379a6).
	class ShopBlacklistChangePacket : public MiniRoomPacket
	{
	public:
		ShopBlacklistChangePacket(bool add, const std::string &name)
			: MiniRoomPacket(add ? Mode::SHOP_BLACKLIST_ADD : Mode::SHOP_BLACKLIST_REMOVE)
		{
			write_string(name);
		}
	};

	// Opens my shop to visitors.
	class ShopOpenPacket : public MiniRoomPacket
	{
	public:
		ShopOpenPacket() : MiniRoomPacket(Mode::OPEN)
		{
			write_byte(1);
		}
	};

	// Opens my shop, room type 4 or 5, with a cash item listed: the open
	// with the birthday code in its place (0x6fca94).
	class ShopOpenBirthdayPacket : public MiniRoomPacket
	{
	public:
		ShopOpenBirthdayPacket(int8_t room_type, int32_t birthday) : MiniRoomPacket(Mode::BIRTHDAY)
		{
			write_byte(Mode::OPEN);
			write_byte(room_type);
			write_int(birthday);
		}
	};

	// Opens an Omok (type 1) or match cards (type 2) room with a game set
	// from the Etc bag, which the server finds itself (0xa1d7b0). `piece` is
	// the Omok stones, or the cards' board size.
	class GameCreatePacket : public MiniRoomPacket
	{
	public:
		GameCreatePacket(int8_t type, const std::string& title, const std::string& password, int8_t piece)
			: MiniRoomPacket(Mode::CREATE)
		{
			write_byte(type);
			write_string(title);
			write_byte(!password.empty());

			if (!password.empty())
				write_string(password);

			write_byte(piece);
		}
	};

	// The owner's game room opens to the map's players as it opens on
	// screen (0x6e38d6).
	class GameOpenPacket : public MiniRoomPacket
	{
	public:
		GameOpenPacket() : MiniRoomPacket(Mode::OPEN)
		{
			write_byte(1);
		}
	};

	// A player's requests at an Omok or match cards game.
	class GamePacket : public MiniRoomPacket
	{
	public:
		enum Request : int8_t
		{
			ASK_TIE = 0x32,
			GIVE_UP = 0x34,
			ASK_UNDO = 0x36,
			LEAVE_AFTER = 0x38,
			STAY_AFTER = 0x39,
			READY = 0x3A,
			UNREADY = 0x3B,
			EXPEL = 0x3C,
			START = 0x3D,
			// The turn's time ran out.
			SKIP = 0x3F
		};

		enum Answer : int8_t
		{
			TIE = 0x33,
			UNDO = 0x37
		};

		GamePacket(Request request) : MiniRoomPacket(static_cast<Mode>(request)) {}

		// Yes or no to the opponent's request.
		GamePacket(Answer answer, bool yes) : MiniRoomPacket(static_cast<Mode>(answer))
		{
			write_byte(yes);
		}
	};

	// Puts my stone on the Omok board's cell (x, y).
	class OmokMovePacket : public MiniRoomPacket
	{
	public:
		OmokMovePacket(int32_t x, int32_t y, int8_t color) : MiniRoomPacket(static_cast<Mode>(MOVE))
		{
			write_int(x);
			write_int(y);
			write_byte(color);
		}

	private:
		static constexpr int8_t MOVE = 0x40;
	};

	// Turns up match card `index`, the turn's first or its second
	// (0x64ee2b).
	class MatchCardsFlipPacket : public MiniRoomPacket
	{
	public:
		MatchCardsFlipPacket(bool first, uint8_t index) : MiniRoomPacket(static_cast<Mode>(FLIP))
		{
			write_byte(first);
			write_byte(index);
		}

	private:
		static constexpr int8_t FLIP = 0x44;
	};

	// Sends the visitor in a seat away, for good.
	class ShopBanPacket : public MiniRoomPacket
	{
	public:
		ShopBanPacket(uint8_t slot, const std::string &name) : MiniRoomPacket(Mode::SHOP_BAN)
		{
			write_byte(static_cast<int8_t>(slot));
			write_string(name);
		}
	};

	// The owner's requests in a hired merchant (0x51804c).
	class MerchantPacket : public MiniRoomPacket
	{
	public:
		enum Request : int8_t
		{
			// Ends maintenance; the merchant opens again.
			REOPEN = 0x27,
			// Strikes off the sold-out rows.
			ORGANIZE = 0x28,
			// Closes the store, with its items and mesos back.
			CLOSE = 0x29,
			TAKE_MESO = 0x2B,
			VISITORS = 0x2E,
			BLACKLIST = 0x2F
		};

		MerchantPacket(Request request) : MiniRoomPacket(static_cast<Mode>(request)) {}
	};

	// Adds a name to my merchant's blacklist, or takes one off (0x519611,
	// 0x519695).
	class MerchantBlacklistPacket : public MiniRoomPacket
	{
	public:
		MerchantBlacklistPacket(bool add, const std::string &name)
			: MiniRoomPacket(static_cast<Mode>(add ? BLACKLIST_ADD : BLACKLIST_REMOVE))
		{
			write_string(name);
		}

	private:
		static constexpr int8_t BLACKLIST_ADD = 0x30;
		static constexpr int8_t BLACKLIST_REMOVE = 0x31;
	};

	// The owner comes into their hired merchant from elsewhere on the
	// channel, with the Store Remote Controller in cash slot `slot`: the
	// visit, sure of the birthday code (0xa2806c).
	class MerchantRemoteVisitPacket : public MiniRoomPacket
	{
	public:
		MerchantRemoteVisitPacket(int32_t birthday, int32_t serial, int16_t slot, int64_t cash_id)
			: MiniRoomPacket(Mode::BIRTHDAY)
		{
			write_byte(Mode::VISIT);
			write_byte(5);
			write_int(birthday);
			write_int(serial);
			write_byte(1);
			write_short(slot);
			write_long(cash_id);
		}
	};

	// Uses the Store Remote Controller in cash slot `slot` (0xa24606); the
	// server answers with ENTRUSTED_SHOP_CHECK_RESULT 0x10 or 0x11.
	// Opcode: REMOTE_SHOP_REQUEST(59)
	class RemoteShopRequestPacket : public OutPacket
	{
	public:
		RemoteShopRequestPacket(int16_t slot) : OutPacket(OutPacket::Opcode::REMOTE_SHOP_REQUEST)
		{
			write_short(slot);
		}
	};

	// Asks whether a hired merchant permit may be used (0xa27d17); the
	// server answers with ENTRUSTED_SHOP_CHECK_RESULT.
	// Opcode: ENTRUSTED_SHOP_REQUEST(63)
	class EntrustedShopRequestPacket : public OutPacket
	{
	public:
		EntrustedShopRequestPacket(int64_t cash_id) : OutPacket(OutPacket::Opcode::ENTRUSTED_SHOP_REQUEST)
		{
			write_byte(0);
			write_long(cash_id);
		}
	};
}
