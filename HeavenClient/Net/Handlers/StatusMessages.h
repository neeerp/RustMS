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

#include "../InPacket.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// SHOW_STATUS_INFO lines, worded as in the v83 client (MapleLegends
	// client: the message handlers at 0x846d5a and its string pool).
	namespace status_messages
	{
		// An item picked up: the count comes first, and one is "an item".
		inline std::string item_line(const std::string& name, int32_t count)
		{
			if (count < 2)
				return "You have gained an item (" + name + ")";

			return "You have gained items (" + std::to_string(count) + " " + name + ")";
		}

		inline std::string meso_line(int32_t amount)
		{
			if (amount > 0)
				return "You have gained mesos (+" + std::to_string(amount) + ")";

			return "You have lost mesos (" + std::to_string(amount) + ")";
		}

		inline std::string fame_line(int32_t amount)
		{
			if (amount < 0)
				return "You have lost fame (" + std::to_string(amount) + ")";

			return "You have gained fame (+" + std::to_string(amount) + ")";
		}

		// SHOW_STATUS_INFO mode 3, in the OdinMS v83 layout.
		struct ExpGain
		{
			bool white;
			int32_t gain;
			// Quest EXP, which goes to the chat log instead.
			bool inchat;
			int32_t event;
			int32_t wedding;
			// 0, or the event's party EXP rate in percent.
			uint8_t party_rate;
			int32_t party;
			int32_t equip;
			int32_t cafe;
		};

		inline ExpGain read_exp(InPacket& recv)
		{
			ExpGain exp = {};
			exp.white = recv.read_bool();
			exp.gain = recv.read_int();
			exp.inchat = recv.read_bool();
			exp.event = recv.read_int();
			recv.skip(2);
			exp.wedding = recv.read_int();

			if (exp.inchat)
				recv.skip(1);

			exp.party_rate = static_cast<uint8_t>(recv.read_byte());
			exp.party = recv.read_int();

			if (recv.length() >= 4)
				exp.equip = recv.read_int();

			if (recv.length() >= 4)
				exp.cafe = recv.read_int();

			// Rainbow week bonus EXP, which v83 does not show.
			if (recv.length() >= 4)
				recv.skip(4);

			return exp;
		}

		struct Line
		{
			std::string text;
			bool yellow;
		};

		// The status lines for EXP not gained in chat: the gain, white unless
		// the server says otherwise, then each bonus in yellow. v83 has no
		// line for an equipment bonus.
		inline std::vector<Line> exp_lines(const ExpGain& exp)
		{
			std::vector<Line> lines;
			lines.push_back({ "You have gained experience (+" + std::to_string(exp.gain) + ")", !exp.white });

			if (exp.wedding != 0)
				lines.push_back({ "Bonus Wedding EXP (+" + std::to_string(exp.wedding) + ")", true });

			if (exp.party > 0)
			{
				if (exp.party_rate == 0)
					lines.push_back({ "Bonus EXP for PARTY (+" + std::to_string(exp.party) + ")", true });
				else
					lines.push_back({ "Bonus Event Party EXP (+" + std::to_string(exp.party) + ") x" + std::to_string(exp.party_rate / 100), true });
			}

			if (exp.cafe != 0)
				lines.push_back({ "Internet Cafe EXP Bonus (+" + std::to_string(exp.cafe) + ")", true });

			if (exp.event != 0)
				lines.push_back({ "Bonus Event EXP (+" + std::to_string(exp.event) + ")", true });

			return lines;
		}
	}
}
