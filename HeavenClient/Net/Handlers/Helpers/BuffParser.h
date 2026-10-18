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

#include "../Net/InPacket.h"
#include "../Character/Buff.h"

#include <cstdint>
#include <utility>
#include <vector>

namespace ms
{
	// The temporary stats of buff packets, as v83 reads them.
	namespace BuffParser
	{
		// The 128 stat flags: bit n of the 16 bytes as they come.
		struct Flags
		{
			uint64_t first;
			uint64_t second;

			bool has(uint8_t bit) const;
			void set(uint8_t bit);
		};

		Flags read_flags(InPacket& recv);

		// The set flags below the two-state stats, in the order GIVE_BUFF
		// carries their data (FUN_00781d0e).
		std::vector<uint8_t> given(const Flags& flags);

		// The stats HeavenClient keeps among those set.
		std::vector<Buffstat::Id> stats(const Flags& flags);

		// The stats another character shows, and each one's value (FUN_00788156),
		// as SPAWN_PLAYER and GIVE_FOREIGN_BUFF carry them: their data, the
		// defense attack and state, then the two-state stats' data.
		std::vector<std::pair<Buffstat::Id, int32_t>> parse_shown(InPacket& recv, const Flags& flags);
	}
}
