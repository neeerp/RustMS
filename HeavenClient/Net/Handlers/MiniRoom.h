#pragma once

#include "ShopList.h"
#include "StorageContents.h"

#include "../InPacket.h"
#include "../../Character/MiniRoomBox.h"
#include "../Login.h"

#include <array>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The parts of v83's mini rooms (PLAYER_INTERACTION, FUN_0065df4c) that
	// need no UI. Only the trading room is supported.
	namespace mini_room
	{
		// The room types of FUN_0065de20 that HC opens.
		constexpr int8_t TRADING_ROOM = 3;
		constexpr int8_t PERSONAL_SHOP = 4;

		// Each side of a trade has nine slots, numbered from 1.
		constexpr int8_t TRADE_SLOTS = 9;

		// What a side of a trade offers; v83 shades the side once it pressed
		// Trade.
		struct Offer
		{
			std::array<std::optional<storage_contents::Stored>, TRADE_SLOTS> items;
			int32_t meso = 0;
			bool confirmed = false;
		};

		// The room's own modes call my side 0 and the partner's 1.
		enum Side : uint8_t
		{
			MINE,
			PARTNER
		};

		struct Member
		{
			uint8_t slot;
			LookEntry look;
			std::string name;
		};

		// A row of a personal shop (FUN_007a0a3a): bundles left of
		// per_bundle items each, at a price per bundle. Sold-out rows stay
		// with no bundles.
		struct Listing
		{
			int16_t bundles = 0;
			int16_t per_bundle = 0;
			int32_t price = 0;
			storage_contents::Stored item;
		};

		inline std::vector<Listing> read_listings(InPacket& recv)
		{
			std::vector<Listing> listings;

			for (auto count = static_cast<uint8_t>(recv.read_byte()); count > 0; count--)
			{
				Listing listing;
				listing.bundles = recv.read_short();
				listing.per_bundle = recv.read_short();
				listing.price = recv.read_int();
				listing.item = storage_contents::read_item(recv);
				listings.push_back(listing);
			}

			return listings;
		}

		// An enter (FUN_0065dff3): the room's type, or 0 and why we can't
		// come in. Rooms other than the trading room and the personal shop
		// are left unread.
		struct Enter
		{
			int8_t type = 0;
			int8_t error = 0;
			uint8_t max_users = 0;
			uint8_t own_slot = 0;
			std::vector<Member> members;
			// A personal shop's title, its room for rows, and its rows.
			std::string title;
			uint8_t max_items = 0;
			std::vector<Listing> listings;
		};

		using LookReader = LookEntry(*)(InPacket&);

		// An AvatarLook and a name (FUN_0065ed1c).
		inline Member read_member(InPacket& recv, uint8_t slot, LookReader read_look)
		{
			LookEntry look = read_look(recv);
			std::string name = recv.read_string();

			return { slot, look, name };
		}

		// The balloon block of UPDATE_CHAR_BOX and the player spawn
		// (FUN_00938ba5): the room's type, 0 for none, then the room.
		inline std::optional<MiniRoomBox> read_box(InPacket& recv)
		{
			int8_t type = recv.read_byte();

			if (type == 0)
				return std::nullopt;

			MiniRoomBox box;
			box.type = type;
			box.serial = recv.read_int();
			box.title = recv.read_string();
			recv.skip_byte(); // private
			box.skin = static_cast<uint8_t>(recv.read_byte());
			box.users = static_cast<uint8_t>(recv.read_byte());
			box.max_users = static_cast<uint8_t>(recv.read_byte());
			recv.skip_byte(); // a game in progress

			return box;
		}

		// The members follow the room's size and our seat, up to slot 0xFF
		// (FUN_0065ec3d); the trading room reads nothing more, and a
		// personal shop its title, size and rows (FUN_007a0a3a).
		inline Enter read_enter(InPacket& recv, LookReader read_look)
		{
			Enter enter;
			enter.type = recv.read_byte();

			if (enter.type == 0)
			{
				enter.error = recv.read_byte();
			}
			else if (enter.type == TRADING_ROOM || enter.type == PERSONAL_SHOP)
			{
				enter.max_users = static_cast<uint8_t>(recv.read_byte());
				enter.own_slot = static_cast<uint8_t>(recv.read_byte());

				for (auto slot = static_cast<uint8_t>(recv.read_byte()); slot != 0xFF; slot = static_cast<uint8_t>(recv.read_byte()))
					enter.members.push_back(read_member(recv, slot, read_look));

				if (enter.type == PERSONAL_SHOP)
				{
					enter.title = recv.read_string();
					enter.max_items = static_cast<uint8_t>(recv.read_byte());
					enter.listings = read_listings(recv);
				}
			}

			return enter;
		}

		// An item a side put up (FUN_007c1fb7) in one of its slots, 1 to 9.
		struct PutItem
		{
			Side side;
			int8_t slot;
			storage_contents::Stored item;
		};

		// Who, the slot, then the item; nothing if the side or slot is out of
		// range.
		inline std::optional<PutItem> read_put_item(InPacket& recv)
		{
			auto who = static_cast<uint8_t>(recv.read_byte());
			auto slot = static_cast<uint8_t>(recv.read_byte());
			storage_contents::Stored item = storage_contents::read_item(recv);

			if (who > Side::PARTNER || slot < 1 || slot > TRADE_SLOTS)
				return std::nullopt;

			return PutItem{ static_cast<Side>(who), static_cast<int8_t>(slot), item };
		}

		// The whole amount a side offers (FUN_007c208e).
		struct PutMeso
		{
			Side side;
			int32_t meso;
		};

		inline std::optional<PutMeso> read_put_meso(InPacket& recv)
		{
			auto who = static_cast<uint8_t>(recv.read_byte());
			int32_t meso = recv.read_int();

			if (who > Side::PARTNER)
				return std::nullopt;

			return PutMeso{ static_cast<Side>(who), meso };
		}

		// Whether putting up an item asks how many (FUN_007c359f): equipment,
		// throwing stars and bullets go whole, as does a single item.
		inline bool asks_count(int32_t itemid, int16_t count)
		{
			return count > 1 && itemid / 1000000 != 1 && !shop_list::is_rechargeable(itemid);
		}

		// The item ids of one side, by slot.
		inline std::vector<int32_t> item_ids(const Offer& offer)
		{
			std::vector<int32_t> ids;

			for (const auto& item : offer.items)
				if (item)
					ids.push_back(item->itemid);

			return ids;
		}

		// What the Trade button lists (FUN_007c39a0): slot by slot, mine then
		// the partner's.
		inline std::vector<int32_t> item_ids(const std::array<Offer, 2>& offers)
		{
			std::vector<int32_t> ids;

			for (size_t i = 0; i < TRADE_SLOTS; i++)
				for (const Offer& offer : offers)
					if (offer.items[i])
						ids.push_back(offer.items[i]->itemid);

			return ids;
		}

		// The trade slot at a point of the window (FUN_007c3b52): 1 to 9 for
		// mine, -1 to -9 for the partner's, 0 for none.
		inline int8_t slot_at(int16_t x, int16_t y)
		{
			for (int8_t i = 0; i < TRADE_SLOTS; i++)
			{
				int16_t left = 39 * (i % 3);
				int16_t top = 131 + 37 * (i / 3);

				if (y < top || y >= top + 32)
					continue;

				if (x >= 172 + left && x < 204 + left)
					return i + 1;

				if (x >= 10 + left && x < 42 + left)
					return -1 - i;
			}

			return 0;
		}

		// A number with commas between the thousands, as v83 shows the
		// offers (FUN_00988690).
		inline std::string meso_text(int64_t meso)
		{
			std::string text = std::to_string(meso);
			size_t digits = meso < 0 ? 1 : 0;

			for (size_t i = text.size(); i > digits + 3; i -= 3)
				text.insert(i - 3, 1, ',');

			return text;
		}

		// The answer to our invite (FUN_0065e848), or nothing.
		inline std::string invite_result_text(int8_t result, const std::string& name)
		{
			switch (result)
			{
			case 1:
				return "Unable to find the character.";
			case 2:
				return "'" + name + "' is doing something else right now.";
			case 3:
				return "'" + name + "' have denied invitation.";
			case 4:
				return "'" + name + "' is currently not accepting any invitation.";
			default:
				return "";
			}
		}

		// Whether the invite result carries the invitee's name.
		inline bool invite_result_named(int8_t result)
		{
			return result >= 2 && result <= 4;
		}

		// Why we couldn't enter a room (FUN_0065dff3), or nothing. Notices
		// break lines with "\\n".
		inline std::string enter_error_text(int8_t error)
		{
			switch (error)
			{
			case 1:
				return "The room is already closed.";
			case 2:
				return "You can't enter the room due to full capacity.";
			case 3:
				return "Other requests are being fulfilled this minute.";
			case 4:
				return "You can't do it while you're dead.";
			case 5:
				return "You can't do it in the middle of the event.";
			case 6:
				return "This character is unable to do it.";
			case 7:
			case 0x14:
				return "You are not allowed to trade other items at this point.";
			case 9:
				return "You can only make trade\\nif you two are in the same map.";
			case 10:
				return "You may not open the store near a portal.";
			case 0xb:
			case 0xe:
				return "You can't start the game here.";
			case 0xc:
				return "You cannot open a store in this channel. Please use a different channel.";
			case 0xd:
				return "You can't establish a miniroom right here.";
			case 0xf:
				return "The personal store can only be built\\nat the free market available at main towns.";
			case 0x10:
				return "You may only use it at Room 7~22, located at the 2nd floor and above of the Free Market Entrance.";
			case 0x11:
				return "You may not enter this store.";
			case 0x12:
				return "The owner of the store is currently\\nundergoing store maintenance.\\nPlease try again in a bit.";
			case 0x13:
				return "You are unable to enter the tournament room.";
			case 0x15:
				return "You don't have enough mesos to enter.";
			case 0x16:
				return "The password is incorrect.";
			case 0x18:
				return "This item has expired.";
			default:
				return "";
			}
		}

		// The mesos a successful trade brought after fees, or 0 (FUN_007c221d):
		// shown when my mesos rose since the room opened, but by less than
		// the partner offered over me.
		inline int32_t received_after_fees(int64_t meso_at_open, int64_t meso_now, const std::array<Offer, 2>& offers)
		{
			int64_t received = meso_now - meso_at_open;
			int64_t offered = static_cast<int64_t>(offers[Side::PARTNER].meso) - offers[Side::MINE].meso;

			return received > 0 && received < offered ? static_cast<int32_t>(received) : 0;
		}

		// What the trading room says as it closes, or nothing.
		inline std::string leave_text(int8_t reason, int32_t received)
		{
			switch (reason)
			{
			case 2:
				return "Trade cancelled.\\nby the other character.";
			case 7:
				if (received > 0)
				{
					char text[128];
					std::snprintf(text, sizeof(text),
						"Trade successful.\\nReceived %d mesos after fees.\\nPlease check the results.", received);

					return text;
				}

				return "Trade successful.\\nPlease check the results.";
			case 8:
				return "Trade unsuccessful.";
			case 9:
				return "You cannot make the trade because\\nthere are some items which\\nyou cannot carry more than one.";
			case 0xc:
				return "You cannot make the trade because\\nthe other person's on a different map.";
			case 0xd:
				return "The game file has been damaged\\nso you cannot participate in an item trade. Please try again\\nafter reinstalling the game.";
			default:
				return "";
			}
		}

		// A partner's chat line, "name : text", with its text filtered as
		// v83 does (FUN_0065eec6); a line without a name is filtered whole.
		template <typename Filter>
		std::string filter_chat(const std::string& line, Filter filter)
		{
			const std::string separator = " : ";
			size_t at = line.find(separator);

			if (at == std::string::npos)
				return filter(line);

			size_t text = at + separator.size();

			return line.substr(0, text) + filter(line.substr(text));
		}
	}
}
