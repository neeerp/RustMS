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

#include "../Net/InPacket.h"

#include <cctype>
#include <cstdint>
#include <string>

namespace ms
{
	// Name Change (5400000), bought in the Cash Shop and cancelled by using
	// its receipt from the Cash tab (reports c061 and c062): the texts v83
	// shows, and the checks it makes before asking the server.
	namespace name_change
	{
		constexpr int32_t ITEM = 5400000;
		constexpr size_t NAME_LIMIT = 12;

		// StringPool 0x204-0x20b, 0x187, 0xdf2 and 0x251.
		constexpr const char* CANNOT_USE = "You cannot use this name. \\nPlease try again.";
		constexpr const char* IN_USE = "This name is currently being used. \\nPlease use another name.";
		constexpr const char* CAN_USE = "This name can be used. \\nIf you wish to use this name,\\nplease press OK.";
		constexpr const char* SAME_NAME = "This name is same as your current name. \\nPlease use another name.";
		constexpr const char* CHECK_FIRST = "Press the CHECK button \\nto check if you can use\\nthe name of the character.";
		constexpr const char* NOT_REQUESTED = "The character you logged on with\\nhas not requested a name change.";
		constexpr const char* UNKNOWN_ERROR = "An unknown error has occured.";
		constexpr const char* NO_PREPAID = "You don't have enough NX Prepaid";

		// What v83 (FUN_0047bbb6) says of the rules check (0x149), or
		// nullptr when the rules come up.
		inline const char* rules_refusal(uint8_t result)
		{
			switch (result)
			{
			case 0:
				return nullptr;
			case 1:
				return "The name change is already submitted \\ndue to the item purchase";
			case 2:
				return "This applies to the limitations on the request.\\nPlease check if you were recently banned \\nwithin 3 months.";
			case 3:
				return "This applies to the limitations on the request.\\nPlease check if you requested \\nfor the name change within a month.";
			default:
				return UNKNOWN_ERROR;
			}
		}

		inline bool same_name(const std::string& a, const std::string& b)
		{
			if (a.size() != b.size())
				return false;

			for (size_t i = 0; i < a.size(); i++)
				if (std::tolower(static_cast<unsigned char>(a[i])) != std::tolower(static_cast<unsigned char>(b[i])))
					return false;

			return true;
		}

		// Why CHECK sends nothing, or nullptr to ask the server. allowed is
		// the client's own name rules, curse words and all.
		inline const char* check_refusal(const std::string& name, const std::string& current, bool allowed)
		{
			if (!allowed)
				return CANNOT_USE;

			if (same_name(name, current))
				return SAME_NAME;

			return nullptr;
		}

		// Why OK buys nothing, or nullptr to buy: the name must be the one
		// the server last said could be used.
		inline const char* buy_refusal(const std::string& name, const std::string& current, const std::string& checked)
		{
			if (same_name(name, current))
				return SAME_NAME;

			if (checked.empty() || name != checked)
				return CHECK_FIRST;

			return nullptr;
		}

		// What the server said of a name checked (0x148): nullptr when it
		// is free.
		inline const char* check_answer(int8_t taken)
		{
			if (taken == 0)
				return nullptr;

			return taken > 0 ? IN_USE : UNKNOWN_ERROR;
		}

		// The rules sheet (String NameChange.img) as one text: v83 breaks
		// a line at "\r\n", and its curly quotes become straight ones.
		inline std::string rules_text(const std::string& raw)
		{
			std::string text;

			for (size_t i = 0; i < raw.size(); i++)
			{
				if (raw.compare(i, 4, "\\r\\n") == 0)
				{
					text += "\\n";
					i += 3;
				}
				else if (raw.compare(i, 3, "\xE2\x80\x9C") == 0 || raw.compare(i, 3, "\xE2\x80\x9D") == 0)
				{
					text += '"';
					i += 2;
				}
				else
				{
					text += raw[i];
				}
			}

			return text;
		}

		// The answer to cancelling with the receipt (0x71, v83 FUN_00a2a677).
		struct CancelAnswer
		{
			enum Kind
			{
				DONE,
				FAILED,
				MESSAGE
			};

			Kind kind;
			std::string message;
		};

		inline CancelAnswer read_cancel_answer(InPacket& recv)
		{
			uint8_t result = recv.read_byte();

			if (result == 0)
				return { CancelAnswer::DONE, {} };

			if (result == 0xFF)
				return { CancelAnswer::FAILED, {} };

			if (recv.read_byte() == 0)
				return { CancelAnswer::MESSAGE, UNKNOWN_ERROR };

			return { CancelAnswer::MESSAGE, recv.read_string() };
		}
	}
}
