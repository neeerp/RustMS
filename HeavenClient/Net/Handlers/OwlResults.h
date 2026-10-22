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

#include "StorageContents.h"

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The Owl of Minerva (USE class 231, reports c024 and c026): it searches
	// the Free Market's personal shops on every channel for an item.
	namespace owl
	{
		// The Free Market entrance and its 22 rooms (FUN_00a0a2dc).
		inline bool in_free_market(int32_t mapid)
		{
			return mapid >= 910000000 && mapid <= 910000022;
		}

		// SP 0x96, while a mini-room dialog is open.
		constexpr const char *NOT_AVAILABLE = "Not available";
		// SP 0xe39.
		constexpr const char *OUTSIDE = "This can only be used inside the Free Market.";
		// SP 0xe35, for a search with no rows.
		constexpr const char *NOTHING_FOUND = "Unable to find the item\\nyou have entered.";

		// OWL_RESULT's modes (FUN_00a28c29).
		constexpr int8_t RESULTS = 6;
		constexpr int8_t POPULAR = 7;

		// One shop's listing of the item searched for.
		struct Row
		{
			std::string owner;
			int32_t mapid = 0;
			std::string title;
			int32_t bundles = 0;
			int32_t per_bundle = 0;
			int32_t price = 0;
			// What GO sends back to name the shop.
			int32_t room = 0;
			// 0-based; 0xff once GO found it closed.
			uint8_t channel = 0;
			// The inventory type, 1 for an equip.
			int8_t type = 0;
			// The whole item, for an equip.
			std::optional<storage_contents::Stored> equip;
		};

		struct Results
		{
			// A count the client turns into a header row of its own when
			// above 0; RustMS sends 0.
			int32_t header = 0;
			int32_t itemid = 0;
			std::vector<Row> rows;

			bool empty() const
			{
				return header == 0 && rows.empty();
			}
		};

		// Mode 6 after its mode byte.
		inline Results read_results(InPacket &recv)
		{
			Results results;
			results.header = recv.read_int();
			results.itemid = recv.read_int();

			for (int32_t count = recv.read_int(); count > 0; count--)
			{
				Row row;
				row.owner = recv.read_string();
				row.mapid = recv.read_int();
				row.title = recv.read_string();
				row.bundles = recv.read_int();
				row.per_bundle = recv.read_int();
				row.price = recv.read_int();
				row.room = recv.read_int();
				row.channel = static_cast<uint8_t>(recv.read_byte());
				row.type = recv.read_byte();

				if (row.type == 1)
					row.equip = storage_contents::read_item(recv);

				results.rows.push_back(row);
			}

			return results;
		}

		// Mode 7 after its mode byte: the items searched for most.
		inline std::vector<int32_t> read_popular(InPacket &recv)
		{
			std::vector<int32_t> items;

			for (auto count = static_cast<uint8_t>(recv.read_byte()); count > 0; count--)
				items.push_back(recv.read_int());

			return items;
		}

		// OWL_GO_RESULT's codes (FUN_008a4e7a).
		constexpr int8_t GO = 0;
		constexpr int8_t CLOSED = 1;

		// What a GO answer says, or nothing for go and codes the client
		// leaves unread.
		inline std::string go_text(int8_t code)
		{
			switch (code)
			{
			case 1:
				return "The room is already closed.";
			case 2:
				return "You can't enter the room due to full capacity.";
			case 3:
				return "Other requests are being fulfilled this minute.";
			case 4:
				return "You can't do it while you're dead.";
			case 7:
				return "You are not allowed to trade other items at this point.";
			case 0x11:
				return "You may not enter this store.";
			case 0x12:
				return "The owner of the store is currently\\nundergoing store maintenance.\\nPlease try again in a bit.";
			default:
				return "";
			}
		}
	}
}
