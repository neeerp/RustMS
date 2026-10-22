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

namespace ms
{
	// The Cash Shop's coupons (v83 0x471240 and 0x479d8a): the code typed
	// in the Coupon dialog, and what the shop says of the answer.
	namespace cash_coupon
	{
		// The longest code the dialog takes.
		constexpr size_t CODE_LIMIT = 32;
		// Said instead of sending an empty code.
		constexpr const char* NO_CODE = "Please enter the coupon code.";
		// The third wrong code (0xB1): said after the default notice,
		// then the shop sends the player back.
		constexpr uint8_t STRIKES = 0xB1;
		constexpr const char* STRUCK_OUT = "You are disconnected \\nfrom Cash Shop \\nbecause you put wrong \\nCoupon numbers three times.";

		// The code without spaces around it.
		inline std::string trim(const std::string& text)
		{
			size_t first = text.find_first_not_of(' ');

			if (first == std::string::npos)
				return {};

			return text.substr(first, text.find_last_not_of(' ') - first + 1);
		}

		// The art asks for the code without its "-", and v83 drops one
		// typed at the end.
		inline std::string without_hyphen(std::string text)
		{
			while (!text.empty() && text.back() == '-')
				text.pop_back();

			return text;
		}

		// What v83 says of a failure code that only coupons get, or
		// nullptr for the rest.
		inline const char* failure(uint8_t code)
		{
			switch (code)
			{
			case 0xB0:
				return "Please check and see if \\nthe coupon number is right.";
			case 0xB2:
				return "This coupon has expired.";
			case 0xB3:
				return "This coupon was already used.";
			case 0xB4:
				return "This coupon can only be used at\\nNexon-affiliated Internet Cafe's.\\nPlease use the Nexon-affiliated Internet Cafe's.";
			case 0xB5:
				return "This coupon is a Nexon-affiliated Internet Cafe-only coupon,\\nand it had already been used.";
			case 0xB6:
				return "This coupon is a Nexon-affiliated Internet Cafe-only coupon,\\nand it had already been expired.";
			case 0xB7:
				return "This is the NX coupon number.\\nRegister your coupon at www.nexon.net.";
			case 0xB8:
				return "Due to gender restrictions, the coupon \\nis unavailable for use.";
			case 0xB9:
				return "This coupon is only for regular items, and \\nit's unavailable to give away as a gift.";
			case 0xBA:
				return "This coupon is only for MapleStory, and\\nit cannot be gifted to others.";
			case 0xC7:
				return "This coupon is only available to the users buying cash item for the first time.";
			case 0xC8:
				return "You have already applied for this.";
			case 0xD0:
				return "You have exceeded the maximum number\\nof usage per account\\for this account.\\nPlease check the coupon for detail.";
			case 0xD2:
				return "The coupon system will be available soon.";
			default:
				return nullptr;
			}
		}

		// An item a coupon gave, by name.
		struct Received
		{
			int32_t count;
			std::string name;
		};

		// What v83 (0x479d8a) says of a redeemed coupon: the items, one to
		// a line, then the MaplePoints and the mesos. Its "and" sticks to
		// the word before it (report b073).
		inline std::string notice(const std::vector<Received>& items, int32_t points, int32_t mesos)
		{
			std::string text = "You have received";

			for (const Received& item : items)
				text += "\\n" + std::to_string(item.count) + " " + item.name;

			if (!items.empty() && points + mesos != 0)
				text += "and \\n";

			if (points)
			{
				text += std::to_string(points) + " MaplePoints";

				if (mesos)
					text += "and \\n";
			}

			if (mesos)
				text += std::to_string(mesos) + " mesos";

			return text + "\\nusing the coupon.";
		}
	}
}
