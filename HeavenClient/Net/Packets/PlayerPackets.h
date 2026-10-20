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

#include <cstddef>

#include <cstdint>

#include "../OutPacket.h"

#include "../../Character/MapleStat.h"
#include "../../Character/SkillMacros.h"

#include "../IO/UITypes/UIKeyConfig.h"

namespace ms
{
	// Reports natural HP/MP recovery; the server checks the amount and timing
	// Opcode: HEAL_OVER_TIME(89)
	class HealOverTimePacket : public OutPacket
	{
	public:
		HealOverTimePacket(int16_t hp, int16_t mp) : OutPacket(OutPacket::Opcode::HEAL_OVER_TIME)
		{
			write_time();
			write_int(0);
			write_short(hp);
			write_short(mp);
		}
	};

	// Reports sitting down on a portable chair (a 301xxxx setup item)
	// Opcode: USE_CHAIR(43)
	class UseChairPacket : public OutPacket
	{
	public:
		UseChairPacket(int32_t itemid) : OutPacket(OutPacket::Opcode::USE_CHAIR)
		{
			write_int(itemid);
		}
	};

	// Asks to stand up from a chair; -1 is a portable chair, otherwise the map
	// seat's id
	// Opcode: CANCEL_CHAIR(42)
	class CancelChairPacket : public OutPacket
	{
	public:
		CancelChairPacket(int16_t seat) : OutPacket(OutPacket::Opcode::CANCEL_CHAIR)
		{
			write_short(seat);
		}
	};

	// Requests a stat increase by spending AP. HP and MP buttons send the
	// MAXHP and MAXMP codes.
	// Opcode: SPEND_AP(87)
	class SpendApPacket : public OutPacket
	{
	public:
		SpendApPacket(MapleStat::Id stat) : OutPacket(OutPacket::Opcode::SPEND_AP)
		{
			write_time();
			write_int(MapleStat::codes[stat]);
		}
	};

	// Requests a skill level increase by spending SP
	// Opcode: SPEND_SP(90)
	class SpendSpPacket : public OutPacket
	{
	public:
		SpendSpPacket(int32_t skill_id) : OutPacket(OutPacket::Opcode::SPEND_SP)
		{
			write_time();
			write_int(skill_id);
		}
	};

	// Requests taking in the gacha EXP from a Writ of Solomon, v83
	// FUN_00a1288f
	// Opcode: USE_GACHA_EXP(158)
	class GachaExpPacket : public OutPacket
	{
	public:
		GachaExpPacket() : OutPacket(OutPacket::Opcode::USE_GACHA_EXP)
		{
			write_time();
		}
	};

	// Requests the server to show a collected card on the Monster Book's
	// cover, or none for 0
	// Opcode: MONSTER_BOOK_COVER(57)
	class MonsterBookCoverPacket : public OutPacket
	{
	public:
		MonsterBookCoverPacket(int32_t card) : OutPacket(OutPacket::Opcode::MONSTER_BOOK_COVER)
		{
			write_int(card);
		}
	};

	// The whole macro list, sent after every edit
	// Opcode: SKILL_MACRO(110)
	class SkillMacroPacket : public OutPacket
	{
	public:
		SkillMacroPacket(const SkillMacros& macros) : OutPacket(OutPacket::Opcode::SKILL_MACRO)
		{
			write_byte(static_cast<int8_t>(macros.size()));

			for (const SkillMacro& macro : macros.get())
			{
				write_string(macro.name);
				write_byte(macro.silent);

				for (int32_t skill : macro.skills)
					write_int(skill);
			}
		}
	};

	// Requests the server to change key mappings
	// Opcode: CHANGE_KEYMAP(135)
	class ChangeKeyMapPacket : public OutPacket
	{
	public:
		ChangeKeyMapPacket(std::vector<std::tuple<KeyConfig::Key, KeyType::Id, int32_t>> updated_actions) : OutPacket(OutPacket::Opcode::CHANGE_KEYMAP)
		{
			write_int(0); // mode
			write_int(updated_actions.size()); // Number of key changes

			for (size_t i = 0; i < updated_actions.size(); i++)
			{
				auto keymap = updated_actions[i];

				write_int(std::get<0>(keymap));		// key
				write_byte(std::get<1>(keymap));	// type
				write_int(std::get<2>(keymap));		// action
			}
		}
	};
}