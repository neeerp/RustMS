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
#pragma once
#pragma once

#include "BuffParser.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// A mob's temporary statuses (CMobStat), as v83 reads them. Their flags
	// lie in the 16 bytes as a character's do.
	namespace MobStatParser
	{
		enum Stat : uint8_t
		{
			PAD = 0,
			PDR = 1,
			MAD = 2,
			MDR = 3,
			ACC = 4,
			EVA = 5,
			SPEED = 6,
			STUN = 7,
			FREEZE = 8,
			POISON = 9,
			SEAL = 10,
			DARKNESS = 11,
			POWER_UP = 12,
			MAGIC_UP = 13,
			PGUARD_UP = 14,
			MGUARD_UP = 15,
			DOOM = 16,
			WEB = 17,
			PIMMUNE = 18,
			MIMMUNE = 19,
			HARD_SKIN = 21,
			AMBUSH = 22,
			VENOM = 24,
			BURNED = 27,
			PCOUNTER = 29,
			MCOUNTER = 30,
			DISABLE = 31,
			WEAKNESS = 34
		};

		struct Status
		{
			uint8_t stat;
			int16_t value;
			// The skill that caused it, or a mob skill's id | level << 16.
			int32_t reason;
			// How long it lasts, in 500 ms.
			int16_t duration;
			// A counter's spread, and the chance in percent shared by both
			// counters that one hurts (0 always).
			int32_t spread = 0;
			int32_t chance = 0;
		};

		// The statuses whose flags are set (FUN_0078b0b1), as MOB_STAT_SET
		// and the spawn packets carry them. The burns are read past; the
		// counters keep their spreads and chance; Disable is kept with its
		// first byte as its value.
		std::vector<Status> parse(InPacket& recv, const BuffParser::Flags& flags);

		// Read past the burns MOB_STAT_RESET ends (FUN_0078c70a).
		void skip_reset(InPacket& recv, const BuffParser::Flags& flags);

		// What a counter hurts an attacker with for one attack, or 0
		// (FUN_007937e3): with a chance, only when the first roll modulo
		// 100 is under it; its value, give or take half its spread by the
		// second roll.
		inline int32_t counter(const Status& status, uint32_t chance_roll, uint32_t spread_roll)
		{
			if (status.chance != 0 && chance_roll % 100 >= static_cast<uint32_t>(status.chance))
				return 0;

			if (status.spread == 0)
				return status.value;

			int32_t spread = static_cast<int32_t>(spread_roll % static_cast<uint32_t>(status.spread));

			return status.value + spread - status.spread / 2;
		}

		// Whether a byte follows for the mob's next action: Stun, Freeze,
		// Doom or bit 32 changed (FUN_0068ad7f).
		bool stops(const BuffParser::Flags& flags);
	}
}
