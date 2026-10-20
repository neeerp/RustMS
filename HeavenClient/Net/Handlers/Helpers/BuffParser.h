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

		// Energy Charge, a two-state stat: its data is the energy and the skill charging it (report 194).
		constexpr uint8_t ENERGY = 82;
		// Dash's speed and jump, two-state stats: each one's data is its value and the skill (report 194).
		constexpr uint8_t DASH_SPEED = 83;
		constexpr uint8_t DASH_JUMP = 84;
		// Riding, a two-state stat: its data is the mount and the skill riding it (0x793ef2).
		constexpr uint8_t RIDING = 85;
		// Speed Infusion, a two-state stat: its data is the attack speed it adds, the skill and its seconds, with a
		// second time between (0x793e28).
		constexpr uint8_t INFUSION = 86;
		// Homing Beacon's mark, a two-state stat: its data is the skill that marked a mob and the mob (0x77c442).
		constexpr uint8_t GUIDED = 87;

		struct Ride
		{
			int32_t vehicle;
			int32_t skill;
		};

		// The icon's time is the only one Dash's data carries: the client never ends it itself.
		struct Dash
		{
			int32_t speed;
			int32_t jump;
			int32_t skill;
			int16_t seconds;
		};

		struct Infusion
		{
			int32_t speed;
			int32_t skill;
			int16_t seconds;
		};

		struct Beacon
		{
			int32_t skill;
			int32_t mob;
		};

		struct TwoState
		{
			int32_t energy;
			int32_t energy_skill;
			Dash dash;
			Ride ride;
			Infusion infusion;
			Beacon beacon;
		};

		// The two-state stats' data, which follows the defense attack and state: the energy's, Dash's and
		// riding's, if set.
		TwoState read_two_state(InPacket& recv, const Flags& flags);

		// The stats another character shows, and each one's value (FUN_00788156),
		// as SPAWN_PLAYER and GIVE_FOREIGN_BUFF carry them: their data, the
		// defense attack and state, then the two-state stats' data.
		std::vector<std::pair<Buffstat::Id, int32_t>> parse_shown(InPacket& recv, const Flags& flags);
	}
}
