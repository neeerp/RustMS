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

#include "Stance.h"

#include <array>
#include <cstdint>
#include <vector>

namespace ms
{
	namespace attack_stances
	{
		// The stances a regular attack picks from at random, by the weapon's
		// info/attack, as in the MapleRoyals client. A swing or stab
		// (FUN_00766031) is also what bows, crossbows, claws and guns do
		// without ammo. `shoot` asks for the ranged one (FUN_00766272).
		inline const std::vector<Stance::Id> &get(uint8_t attack, bool shoot)
		{
			using namespace Stance;

			static const std::vector<Id> none;

			static const std::array<std::vector<Id>, 10> close = {
					{
							{},
							{SWINGO1, SWINGO2, SWINGO3, STABO1, STABO2},
							{SWINGT2, SWINGP1, SWINGP2, STABT1, STABT2},
							{SWINGT1, SWINGT3},
							{SWINGT1, STABT1},
							{SWINGT1, SWINGT2, SWINGT3, STABO1, STABO2},
							{SWINGO2},
							{STABO1, STABO2},
							{STABO1, STABO2},
							{SWINGT1, SWINGT2, SWINGT2}
					}
			};

			static const std::array<std::vector<Id>, 10> ranged = {
					{
							{},
							{},
							{},
							{SHOOT1},
							{SHOOT2},
							{},
							{},
							{SWINGO1, SWINGO2, SWINGO3},
							{},
							{SHOT}
					}
			};

			if (attack >= close.size())
				return none;

			return shoot ? ranged[attack] : close[attack];
		}

		// The stances a Final Attack picks from, by the weapon's info/attack
		// (the odd rows, FUN_00766031 and FUN_00766272).
		inline const std::vector<Stance::Id> &get_final(uint8_t attack)
		{
			using namespace Stance;

			static const std::array<std::vector<Id>, 10> stances = {
					{
							{},
							{SWINGOF, STABOF},
							{SWINGPF, STABTF},
							{SHOOTF},
							{SHOOT2},
							{SWINGTF, STABOF},
							{},
							{SWINGO1, SWINGO2, SWINGO3},
							{},
							{SHOT}
					}
			};

			static const std::vector<Id> none;

			return attack < stances.size() ? stances[attack] : none;
		}
	}
}
