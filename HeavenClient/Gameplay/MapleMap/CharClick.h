#pragma once

#include "../../Template/Point.h"

#include <cstdint>
#include <cstdlib>

namespace ms
{
	// Whether a click lands on a character: a box about the size of a
	// standing body above their feet, which is where they are positioned.
	namespace char_click
	{
		constexpr int16_t HALF_WIDTH = 18;
		constexpr int16_t HEIGHT = 70;

		inline bool hits(Point<int16_t> feet, Point<int16_t> cursor)
		{
			return std::abs(cursor.x() - feet.x()) <= HALF_WIDTH && cursor.y() <= feet.y() && cursor.y() >= feet.y() - HEIGHT;
		}

		// v83's box for opening the menu on someone (FUN_0045183b): 44 wide
		// and 65 tall above the feet, or 46 by 31 lying prone, toward the
		// head. Left and top edges are in, right and bottom ones out.
		inline bool menu_hits(Point<int16_t> feet, bool prone, bool facing_right, Point<int16_t> cursor)
		{
			int16_t left = prone ? -46 : -22;
			int16_t right = prone ? 0 : 22;
			int16_t top = prone ? -31 : -65;

			if (facing_right)
			{
				int16_t mirrored = -right;
				right = -left;
				left = mirrored;
			}

			int16_t x = cursor.x() - feet.x();
			int16_t y = cursor.y() - feet.y();

			return x >= left && x < right && y >= top && y < 0;
		}
	}
}
