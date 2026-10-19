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

#include <algorithm>
#include <cstdint>
#include <utility>
#include <vector>

namespace ms
{
	// One of the clouds drifting over a map: which MapHelper.img/weather/cloud
	// canvas, its opacity out of 255, where it starts, and how far it drifts
	// every 30 s.
	struct MapCloud
	{
		int32_t tile;
		int32_t alpha;
		int32_t x;
		int32_t y;
		int32_t dx;
		int32_t dy;

		bool operator==(const MapCloud &other) const
		{
			return tile == other.tile && alpha == other.alpha && x == other.x && y == other.y &&
				   dx == other.dx && dy == other.dy;
		}
	};

	namespace map_clouds
	{
		// The ms a cloud takes to drift by its (dx, dy).
		constexpr int32_t PERIOD = 30000;

		// Where clouds drift and wrap.
		struct Area
		{
			int32_t left;
			int32_t top;
			int32_t width;
			int32_t height;
		};

		// v83 grows the field's physical space (0xa4488c) by 200 px on every
		// side.
		inline Area area(int32_t left, int32_t top, int32_t right, int32_t bottom)
		{
			return { left - 200, top - 200, right - left + 400, bottom - top + 400 };
		}

		// How v83 scatters a map's clouds (FUN_0063f717): 10 to 29, and up to 30
		// more as the area nears 9,000,000 px². `next()` is the C library's
		// rand() (0 to 32767), drawn as the client does: the count, then for
		// each cloud its tile, its drift, its opacity, x and y. A cloud drifts
		// 500 to 1490 px right each 30 s, and 15 up to 9 down.
		template<class Next>
		std::vector<MapCloud> scatter(Area area, int32_t tiles, Next next)
		{
			std::vector<MapCloud> clouds;

			if (tiles <= 0 || area.width <= 0 || area.height <= 0)
				return clouds;

			float size = std::min(1.0f, static_cast<float>(static_cast<double>(area.width) * area.height / 9e6));
			int32_t count = next() % 20 + static_cast<int32_t>(30 * size) + 10;

			for (int32_t i = 0; i < count; i++)
			{
				int32_t tile = next() % tiles;
				int32_t drift = next() % 100;
				int32_t alpha = next() % 80 + 140;
				int32_t x = next() % area.width + area.left;
				int32_t y = next() % area.height + area.top;
				clouds.push_back({ tile, alpha, x, y, (drift + 50) * 10, (drift - 60) / 4 });
			}

			return clouds;
		}

		inline int32_t wrap(int64_t value, int32_t from, int32_t length)
		{
			int64_t offset = (value - from) % length;

			return static_cast<int32_t>(from + (offset < 0 ? offset + length : offset));
		}

		// Where a cloud is `elapsed` ms after its map loaded. Shape2D moves it
		// by (dx, dy) every 30 s for good (0x5140809f) and wraps it around the
		// area (0x514088a9).
		inline std::pair<int32_t, int32_t> position(const MapCloud &cloud, int64_t elapsed, Area area)
		{
			int64_t periods = elapsed / PERIOD;
			int64_t part = elapsed % PERIOD;
			int64_t x = cloud.x + periods * cloud.dx + cloud.dx * part / PERIOD;
			int64_t y = cloud.y + periods * cloud.dy + cloud.dy * part / PERIOD;

			return { wrap(x, area.left, area.width), wrap(y, area.top, area.height) };
		}
	}
}
