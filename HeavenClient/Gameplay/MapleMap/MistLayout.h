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
}
