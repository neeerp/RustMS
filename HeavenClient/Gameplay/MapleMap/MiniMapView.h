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

#include <algorithm>
#include <cstdint>

namespace ms
{
	// The part of a minimap's canvas v83 shows, which follows the player
	// over a big map.
	namespace minimap_view
	{
		// A side of the canvas under 280 wide or 150 tall shows whole, a
		// longer one only 210 or 112 of it (0x8588d9).
		inline int32_t width(int32_t canvas)
		{
			return canvas < 280 ? canvas : 210;
		}

		inline int32_t height(int32_t canvas)
		{
			return canvas < 150 ? canvas : 112;
		}

		// Where the shown part starts along one side, in canvas pixels:
		// centred on the player, up to the map's edges (0x85abbc). The
		// player, centre and map size are in map pixels, as the miniMap
		// node's centerX/Y, width and height.
		inline int32_t scroll(int32_t player, int32_t view, int32_t canvas, int32_t center, int32_t map_size, int32_t mag)
		{
			int32_t span = view << mag;
			int32_t start = std::max(player - span / 2, -center);
			start = std::min(start, map_size - center - span);

			// Kept on the canvas, which v83 leaves to the map's numbers.
			return std::clamp((start + center) >> mag, 0, canvas - view);
		}
	}
}
