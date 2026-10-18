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
			PUT_ITEM = 0x0F,
			PUT_MESO = 0x10,
			TRADE = 0x11,
			TRADE_REPLY = 0x14,
			SHOP_PUT_ITEM = 0x16,
			SHOP_BUY = 0x17,
			SHOP_TAKE_BACK = 0x1B,
			SHOP_BAN = 0x1C,
			SHOP_BLACKLIST = 0x1E
		};

		MiniRoomPacket(Mode mode) : OutPacket(OutPacket::Opcode::PLAYER_INTERACTION)
		{
			write_byte(mode);
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

	// Joins the room we were invited to.
	class MiniRoomVisitPacket : public MiniRoomPacket
	{
	public:
		MiniRoomVisitPacket(int32_t serial) : MiniRoomPacket(Mode::VISIT)
		{
			write_int(serial);
			write_byte(0);
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

	// Opens a personal shop (type 4) with a permit from the cash tab (v83
	// FUN_00a1dd99). The shop stays closed to visitors until it opens.
	class ShopCreatePacket : public MiniRoomPacket
	{
	public:
		static constexpr int8_t PERSONAL_SHOP = 4;

		ShopCreatePacket(const std::string &title, int16_t permit_slot, int32_t permit_id) : MiniRoomPacket(Mode::CREATE)
		{
			write_byte(PERSONAL_SHOP);
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
		ShopPutItemPacket(int8_t tab, int16_t slot, int16_t bundles, int16_t per_bundle, int32_t price)
			: MiniRoomPacket(Mode::SHOP_PUT_ITEM)
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
		ShopBuyPacket(uint8_t index, int16_t bundles) : MiniRoomPacket(Mode::SHOP_BUY)
		{
			write_byte(static_cast<int8_t>(index));
			write_short(bundles);
			write_int(0);
		}
	};

	// Takes a row of my shop back into the inventory, before it opens.
	class ShopTakeBackPacket : public MiniRoomPacket
	{
	public:
		ShopTakeBackPacket(int16_t index) : MiniRoomPacket(Mode::SHOP_TAKE_BACK)
		{
			write_short(index);
		}
	};

	// The names my shop turns away. v83 sends its blacklist right before
	// opening; HC has no blacklist yet and sends none.
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

	// Opens my shop to visitors.
	class ShopOpenPacket : public MiniRoomPacket
	{
	public:
		ShopOpenPacket() : MiniRoomPacket(Mode::OPEN)
		{
			write_byte(1);
		}
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
}
