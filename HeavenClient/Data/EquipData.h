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

#include <array>
#include <string>

#include <cstdint>

#include "ItemData.h"

#include "../Character/EquipStat.h"
#include "../Character/MapleStat.h"

#include "../Character/Look/EquipSlot.h"

namespace ms
{
	// Contains information about an equip
	class EquipData : public Cache<EquipData>
	{
	public:
		// Returns whether the equip was loaded correctly
		bool is_valid() const;

		// Returns whether the equip was loaded correctly.
		explicit operator bool() const;

		// Returns whether this equip has equipslot WEAPON
		bool is_weapon() const;

		// Returns a required base stat.
		int16_t get_reqstat(MapleStat::Id stat) const;

		// Returns a default stat.
		int16_t get_defstat(EquipStat::Id stat) const;

		// Returns the equip slot.
		EquipSlot::Id get_eqslot() const;

		// Returns the category name.
		const std::string &get_type() const;

		// Returns the general item data (name, price, etc.).
		const ItemData &get_itemdata() const;

		// Returns how much faster the wearer recovers on maps that already
		// recover faster (info/recovery, 1 when absent).
		double get_recovery() const;

		// The percent of a spell's damage a weapon gives for its element
		// (FUN_005d284f): incRMAI, F, L, S or H for ice, fire, lightning,
		// poison and holy, else elemDefault, else 100. Other elements get
		// 100.
		int16_t get_elemboost(uint8_t element) const;

		// The style of a medal's name tag, UI NameTag.img/medal/<n>
		// (info/medalTag, 0 when absent).
		int32_t get_medaltag() const;

	private:
		// Allow the cache to use the constructor
		friend Cache<EquipData>;

		// Load an equip from the game files.
		EquipData(int32_t id);

		const ItemData &itemdata;

		EnumMap<MapleStat::Id, int16_t> reqstats;
		EnumMap<EquipStat::Id, int16_t> defstats;
		std::string type;
		EquipSlot::Id eqslot;
		uint8_t slots;
		bool cash;
		bool tradeblock;
		double recovery;
		int32_t medaltag;
		std::array<int16_t, 8> elemboost;
	};
}