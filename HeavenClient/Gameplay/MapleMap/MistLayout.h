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
#include <cstdlib>
#include <vector>

namespace ms
{
	// One cloud of a mist: which of the skill's `tile` animations, where its
	// origin goes, its order among the mist's clouds, and whether it's flipped.
	struct MistCloud
	{
		int32_t tile;
		int32_t x;
		int32_t y;
		int32_t z;
		bool flip;

		bool operator==(const MistCloud &other) const
		{
			return tile == other.tile && x == other.x && y == other.y && z == other.z && flip == other.flip;
		}
	};

	// How v83 fills a mist's area with clouds (FUN_00432776): columns from the
	// left edge, 35 to 54 px apart, each a pile of random tiles, every one 0.8
	// of its height below the last. A column spans the area's full height at
	// the centre and half of it at the edges, so the mist is a diamond.
	// `heights` has each tile's first frame height; `next(n)` is a random
	// number below n, drawn as the client does: tile, order, flip.
	template<class Next>
	std::vector<MistCloud>
	layout_mist(int32_t left, int32_t top, int32_t right, int32_t bottom, const std::vector<int16_t> &heights,
				Next next)
	{
		std::vector<MistCloud> clouds;

		if (heights.empty() || left >= right)
			return clouds;

		int32_t width = right - left;
		int32_t height = bottom - top;
		int32_t centre = (left + right) / 2;

		for (int32_t x = left; x < right; x += 35 + next(20))
		{
			double inset = height * (std::abs(x - centre) / static_cast<double>(width)) * 0.5;
			auto y = static_cast<int32_t>(top + inset);

			while (y < bottom - inset)
			{
				auto tile = static_cast<int32_t>(next(static_cast<int32_t>(heights.size())));
				int32_t z = next(10);
				bool flip = next(2) == 1;
				clouds.push_back({ tile, x, y, z, flip });

				auto below = static_cast<int32_t>(heights[tile] * 0.8 + y);
				y = below > y ? below : y + 1;
			}
		}

		return clouds;
	}

	// What a mist does to the player standing in it.
	namespace mist
	{
		// Its type, AFFECTED_AREA_CREATED's second int: a mob's mist hurts,
		// a smoke screen shelters, and type 3 is a cash item's.
		enum Type : int32_t
		{
			MOB = 0,
			SMOKE = 2,
			ITEM = 3
		};

		// The mob skills whose mist hurts by their level's `x`: 130 and
		// Poison Mist (FUN_00431a63).
		constexpr bool hurts(int32_t skillid)
		{
			return skillid == 130 || skillid == 131;
		}

		struct Area
		{
			int32_t type;
			int32_t owner;
			int32_t skillid;
			int32_t level;
			int32_t left;
			int32_t top;
			int32_t right;
			int32_t bottom;
			// The element its damage is resisted as, the packet's last int.
			int32_t element;
			// What it does a second, its level's `x`, if it hurts.
			int32_t damage;
			// Whether its delay is over.
			bool started;

			// The client asks PtInRect, so the right and bottom edges are out.
			constexpr bool contains(int32_t x, int32_t y) const
			{
				return left <= x && x < right && top <= y && y < bottom;
			}
		};

		// The hurt a mist does, and the code TAKE_DAMAGE names it by: its
		// skill's low byte, then its level.
		struct Hurt
		{
			int32_t damage;
			uint8_t element;
			uint16_t code;
		};

		// The first started mob mist the player at (x, y) is in hurts them
		// its damage (FUN_00431783). Nothing comes of one that does none.
		template<class Areas>
		bool hurt_at(const Areas &areas, int32_t x, int32_t y, Hurt &hurt)
		{
			for (const Area &area : areas)
			{
				if (area.type != MOB || !area.started || !area.contains(x, y))
					continue;

				hurt = { area.damage, static_cast<uint8_t>(area.element),
						 static_cast<uint16_t>((area.skillid & 0xFF) << 8 | (area.level & 0xFF)) };

				return area.damage > 0;
			}

			return false;
		}

		// A started smoke screen of the player's own or a party member's
		// keeps mobs off them (FUN_0043193a); `ally(owner)` says whose.
		template<class Areas, class Ally>
		bool shelters(const Areas &areas, int32_t x, int32_t y, Ally ally)
		{
			for (const Area &area : areas)
				if (area.type == SMOKE && area.started && area.contains(x, y) && ally(area.owner))
					return true;

			return false;
		}

		// A cash mist (528xxxx) waits while the user's own started one from
		// that item lasts, or while they stand in anyone's (FUN_00431866).
		template<class Areas>
		bool item_busy(const Areas &areas, int32_t owner, int32_t itemid, int32_t x, int32_t y)
		{
			for (const Area &area : areas)
				if (area.type == ITEM && area.started
					&& ((area.owner == owner && area.skillid == itemid) || area.contains(x, y)))
					return true;

			return false;
		}

		// The element a mob mist's hurt is resisted as: 125 poison, 130
		// fire; Poison Mist (131) none, so it takes its full x
		// (FUN_00765c0f).
		constexpr int32_t resisted_as(int32_t skillid)
		{
			return skillid == 125 ? 4 : skillid == 130 ? 2 : 0;
		}

		// What's left of `damage` after `percent` of it is resisted
		// (0x94b7f2).
		constexpr int32_t resisted(int32_t damage, int32_t percent)
		{
			return (100 - percent) * damage / 100;
		}

		// How long apart a mist hurts the player, in ms: more than this
		// since the last time (0x94b7a2).
		constexpr int32_t HURT_INTERVAL = 1000;
	}
}
