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
	// SHOW_STATUS_INFO lines, worded as in the 2025 MapleRoyals client (its
	// handlers under FUN_00a209d4, and its string pool).
	namespace status_messages
	{
		// The inventory tab an item goes to, by its id.
		inline std::string tab_name(int32_t itemid)
		{
			switch (itemid / 1000000)
			{
			case 1:
				return "Eqp";
			case 2:
				return "Use";
			case 3:
				return "Setup";
			case 4:
				return "Etc";
			case 5:
				return "Cash";
			default:
				return "";
			}
		}

		// An item picked up: one is "an item", and the count follows the name.
		inline std::string item_line(const std::string& tab, const std::string& name, int32_t count)
		{
			if (count < 2)
				return "You have gained an item in the " + tab + " tab (" + name + ")";

			return "You have gained items in the " + tab + " tab (" + name + " " + std::to_string(count) + ")";
		}

		// Mode 5. Zero counts as a loss.
		inline std::string meso_line(int32_t amount)
		{
			if (amount > 0)
				return "You have gained mesos (+" + std::to_string(amount) + ")";

			return "You have lost mesos. (" + std::to_string(amount) + ")";
		}

		// Mode 4. Zero counts as a gain.
		inline std::string fame_line(int32_t amount)
		{
			if (amount < 0)
				return "You have lost fame (" + std::to_string(amount) + ")";

			return "You have gained fame. (+" + std::to_string(amount) + ")";
		}

		// Mode 6. Zero counts as a loss.
		inline std::string guild_points_line(int32_t amount)
		{
			if (amount > 0)
				return "You have earned Guild Points (+" + std::to_string(amount) + ")";

			return "You have lost Guild Points (" + std::to_string(amount) + ")";
		}

		// SHOW_STATUS_INFO mode 3, as FUN_00a21ac5 reads it.
		struct ExpGain
		{
			bool white;
			int32_t gain;
			// Quest EXP, which goes to the chat log instead.
			bool inchat;
			int32_t event;
			// The bonus percent for hunting a while, and for how many hours.
			// The same percent is the bonus for every 3rd monster.
			uint8_t hunting_rate;
			uint8_t hunting_hours;
			int32_t wedding;
			// Spirit Week quest bonus: a percent, and how many quests it lasts.
			uint8_t quest_rate;
			uint8_t quest_count;
			// 0, or the event's party EXP rate in percent.
			uint8_t party_rate;
			int32_t party;
			int32_t equip;
			int32_t cafe;
			int32_t rainbow;
		};

		inline ExpGain read_exp(InPacket& recv)
		{
			ExpGain exp = {};
			exp.white = recv.read_bool();
			exp.gain = recv.read_int();
			exp.inchat = recv.read_bool();
			exp.event = recv.read_int();
			exp.hunting_rate = static_cast<uint8_t>(recv.read_byte());
			recv.skip(1);
			exp.wedding = recv.read_int();

			if (exp.hunting_rate > 0)
				exp.hunting_hours = static_cast<uint8_t>(recv.read_byte());

			if (exp.inchat)
			{
				exp.quest_rate = static_cast<uint8_t>(recv.read_byte());

				if (exp.quest_rate > 0)
					exp.quest_count = static_cast<uint8_t>(recv.read_byte());
			}

			exp.party_rate = static_cast<uint8_t>(recv.read_byte());
			exp.party = recv.read_int();

			if (recv.length() >= 4)
				exp.equip = recv.read_int();

			if (recv.length() >= 4)
				exp.cafe = recv.read_int();

			if (recv.length() >= 4)
				exp.rainbow = recv.read_int();

			return exp;
		}

		struct Line
		{
			std::string text;
			bool yellow;
		};

		// The status lines for EXP not gained in chat: the gain, white unless
		// the server says otherwise, then each bonus in yellow.
		inline std::vector<Line> exp_lines(const ExpGain& exp)
		{
			std::vector<Line> lines;
			lines.push_back({ "You have gained experience (+" + std::to_string(exp.gain) + ")", !exp.white });

			if (exp.equip != 0)
				lines.push_back({ "Equip Item Bonus EXP (+" + std::to_string(exp.equip) + ")", true });

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

			if (exp.hunting_hours != 0 && exp.white)
			{
				int32_t bonus = exp.hunting_rate * exp.gain / 100;
				lines.push_back({ "Bonus EXP for hunting over " + std::to_string(exp.hunting_hours) + " hrs. (+" + std::to_string(bonus) + ")", true });
			}

			if (exp.event != 0)
				lines.push_back({ "Bonus Event EXP (+" + std::to_string(exp.event) + ")", true });

			if (exp.rainbow != 0)
				lines.push_back({ "Rainbow Week Bonus EXP (+" + std::to_string(exp.rainbow) + ")", true });

			return lines;
		}

		// When EXP not gained in chat brings a new bonus percent, v83 says once
		// in chat (type 0xc) that every 3rd monster gives it, keeping the last
		// percent said in `last` (CWvsContext+0x36d0, never reset).
		inline std::string mob_bonus_line(const ExpGain& exp, int32_t& last)
		{
			if (exp.inchat || exp.hunting_rate == 0 || exp.hunting_rate == last)
				return "";

			last = exp.hunting_rate;

			return "A bonus EXP " + std::to_string(last) + "% is awarded for every 3rd monster defeated.";
		}

		// The chat-log lines for EXP gained in chat: the gain, then any Spirit
		// Week quest bonus.
		inline std::vector<std::string> chat_exp_lines(const ExpGain& exp)
		{
			std::vector<std::string> lines;
			lines.push_back("You have gained experience (+" + std::to_string(exp.gain) + ")");

			int32_t bonus = exp.quest_rate * exp.gain / 100;

			if (bonus != 0)
			{
				lines.push_back("Earned 'Spirit Week Event' bonus EXP. (+" + std::to_string(bonus) + ")");

				if (exp.quest_count != 0)
					lines.push_back("The next " + std::to_string(exp.quest_count) + " completed quests will include additional Event Bonus EXP.");
			}

			return lines;
		}
	}
}
