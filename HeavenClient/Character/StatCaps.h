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

#include <unordered_map>

namespace ms
{
	// v83's bounds on the totals: attack and defence at 1999, accuracy and
	// avoid at 999 (FUN_0077f4c9), max HP and MP at 30000. STR, DEX, INT and
	// LUK have none (FUN_0077ec9f).
	const std::unordered_map<EquipStat::Id, int32_t> EQSTAT_CAPS =
			{
					{EquipStat::Id::HP,    30000},
					{EquipStat::Id::MP,    30000},
					{EquipStat::Id::WATK,  1999},
					{EquipStat::Id::MAGIC, 1999},
					{EquipStat::Id::WDEF,  1999},
					{EquipStat::Id::MDEF,  1999},
					{EquipStat::Id::ACC,   999},
					{EquipStat::Id::AVOID, 999},
					{EquipStat::Id::HANDS, 999},
					{EquipStat::Id::SPEED, 140},
					{EquipStat::Id::JUMP,  123},
			};
}