//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published	//
//	by the Free Software Foundation, either version 3 of the License, or		//
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

#include <algorithm>
#include <array>
#include <cstdint>
#include <numeric>
#include <tuple>
#include <vector>

namespace ms
{
	// Which of the equipment worn counts toward its wearer's stats. v83
	// rebuilds the list on every stat change (FUN_0077e227): an item whose
	// requirements its wearer no longer meets stays worn but adds nothing,
	// and a weapon that doesn't count can't attack.
	namespace equip_count
	{
		// STR, DEX, INT and LUK.
		using Stats = std::array<int32_t, 4>;

		// An item's reqLevel, reqJob, reqSTR to reqLUK and reqPOP.
		struct Requirements
		{
			int16_t level = 0;
			int16_t job = 0;
			Stats stats = {};
			int16_t fame = 0;
		};

		struct Wearer
		{
			int32_t level = 0;
			int32_t job = 0;
			int32_t fame = 0;
			// The character's own STR to LUK.
			Stats stats = {};
			// Maple Warrior's percent of them.
			int32_t percent = 0;
		};

		// An item worn in `slot`, positive, cash items 100 above their slot
		// and the dragon's from 1000.
		struct Worn
		{
			int16_t slot = 0;
			int32_t item_id = 0;
			Stats bonus = {};
			Requirements requirements;
		};

		// Whether `wearer` meets `needs` with `totals` of STR to LUK
		// (FUN_004f2cee). GMs meet any.
		inline bool met_by(const Requirements& needs, const Wearer& wearer, const Stats& totals)
		{
			int32_t branch = wearer.job % 1000 / 100;

			if (branch == 8 || branch == 9)
				return true;

			int32_t mask = branch == 0 ? 0 : 1 << (branch - 1);
			bool job_ok = needs.job == 0 || (needs.job == -1 && mask == 0) || (needs.job > 0 && (needs.job & mask));

			for (size_t i = 0; i < totals.size(); i++)
				if (totals[i] < needs.stats[i])
					return false;

			return job_ok && wearer.level >= needs.level && (needs.fame == 0 || wearer.fame >= needs.fame);
		}

		// A slot's turn: slots in order with the cash item after the normal
		// one, and the dragon's last.
		inline std::tuple<int32_t, int32_t, bool> order(int16_t slot)
		{
			if (slot >= 1000)
				return { 1, slot, false };

			if (slot > 100 && slot < 200)
				return { 0, slot - 100, true };

			return { 0, slot, false };
		}

		// Whether a slot's item adds its stats to what the others are
		// checked with; pet equipment doesn't, but the first pet's clothes.
		inline bool adds_stats(int32_t index)
		{
			return index != 30 && index != 38 && (index == 14 || index <= 20 || index > 48);
		}

		// Whether a slot's item is checked against its wearer; pet
		// equipment asks things of its pet instead.
		inline bool checked(int32_t index)
		{
			return index != 14 && adds_stats(index);
		}

		// Which of `worn` count, in their order:
		// - Each is checked with the wearer's own stats, raised by Maple
		//   Warrior, plus every other counted item's, never its own.
		// - The first to fail stops counting and the check starts over,
		//   until a whole pass drops none.
		// - A cash weapon counts only over a counted weapon, and a cash
		//   shield only over a counted shield.
		inline std::vector<bool> counted(const Wearer& wearer, const std::vector<Worn>& worn)
		{
			std::vector<size_t> turns(worn.size());
			std::iota(turns.begin(), turns.end(), 0);
			std::stable_sort(turns.begin(), turns.end(), [&](size_t a, size_t b) {
				return order(worn[a].slot) < order(worn[b].slot);
			});

			Stats own;

			for (size_t i = 0; i < own.size(); i++)
				own[i] = wearer.stats[i] + wearer.stats[i] * wearer.percent / 100;

			std::vector<bool> counts(worn.size(), true);
			Stats added = {};

			for (const Worn& item : worn)
				if (adds_stats(std::get<1>(order(item.slot))))
					for (size_t i = 0; i < added.size(); i++)
						added[i] += item.bonus[i];

			auto counted_at = [&](int16_t slot) {
				for (size_t i = 0; i < worn.size(); i++)
					if (worn[i].slot == slot && counts[i])
						return true;

				return false;
			};

			bool dropped = true;

			while (dropped)
			{
				dropped = false;

				for (size_t i : turns)
				{
					if (!counts[i])
						continue;

					const Worn& item = worn[i];
					auto [group, index, cash] = order(item.slot);
					Stats mine = adds_stats(index) ? item.bonus : Stats {};

					bool over = cash && ((index == 10 && item.item_id / 10000 == 109) ||
										 (index == 11 && item.item_id / 10000 == 170));
					bool under = !over || counted_at(static_cast<int16_t>(index));

					Stats totals;

					for (size_t s = 0; s < totals.size(); s++)
						totals[s] = own[s] + added[s] - mine[s];

					bool met = !checked(index) || met_by(item.requirements, wearer, totals);

					if (!(met && under))
					{
						counts[i] = false;

						for (size_t s = 0; s < added.size(); s++)
							added[s] -= mine[s];

						dropped = true;
						break;
					}
				}
			}

			return counts;
		}
	}
}
