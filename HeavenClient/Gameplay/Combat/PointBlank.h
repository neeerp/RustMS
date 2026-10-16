#pragma once

#include "../../Template/Rectangle.h"

#include <algorithm>
#include <cstdint>

namespace ms
{
	// A regular attack with a bow, crossbow, claw or gun first looks for a mob
	// right in front of the user, and whacks it instead of shooting
	// (FUN_0094e256 passing FUN_00950921 an out pointer).
	namespace point_blank
	{
		// How far in front of the user the whack reaches at most.
		constexpr int16_t MAX_REACH = 65;

		// The whack's range, from the melee stance's afterimage range (lt.x is
		// the front reach, as when facing left).
		inline Rectangle<int16_t> range(Rectangle<int16_t> afterimage)
		{
			int16_t left = std::max<int16_t>(afterimage.left(), -MAX_REACH);
			left = std::min<int16_t>(left, afterimage.right() - 1);

			return { left, afterimage.right(), afterimage.top(), afterimage.bottom() };
		}
	}
}
