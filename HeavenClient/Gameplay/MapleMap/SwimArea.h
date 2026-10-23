#pragma once

#include <cstdint>
#include <vector>

namespace ms
{
	// A map's swimArea: rectangles of water in a map that is not under water as a whole (CField+0x758). A
	// character whose feet are in one swims as in a swim map (IsSwimming FUN_007046ac).
	namespace swim_area
	{
		struct Rect
		{
			int32_t x1;
			int32_t x2;
			int32_t y1;
			int32_t y2;
		};

		// Whether the point is in one of the rectangles, edges included (FUN_005983f4).
		inline bool contains(const std::vector<Rect> &rects, int32_t x, int32_t y)
		{
			for (const Rect &rect : rects)
				if (x >= rect.x1 && x <= rect.x2 && y >= rect.y1 && y <= rect.y2)
					return true;

			return false;
		}

		// Whether a character there swims: the whole map is under water (info/swim) or it is in the water.
		inline bool swimming(bool swim_map, const std::vector<Rect> &rects, int32_t x, int32_t y)
		{
			return swim_map || contains(rects, x, y);
		}
	}
}
