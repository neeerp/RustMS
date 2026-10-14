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
	}
}
