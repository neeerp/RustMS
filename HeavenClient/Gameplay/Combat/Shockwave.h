#pragma once

#include <cstdint>

namespace ms
{
	// Shockwave runs along the floor from the user, the way it faces, a step
	// at a time, and stops where the floor does.
	namespace shockwave
	{
		constexpr int32_t ID = 5111006;
		constexpr int32_t ID_TB = 15111003;
		constexpr int32_t STEP = 55;
		constexpr int32_t MAX_STEPS = 5;
		constexpr int32_t FLOOR_REACH = 30;
		constexpr int32_t EDGE = 30;

		inline bool is(int32_t skill)
		{
			return skill == ID || skill == ID_TB;
		}

		// How many steps it runs; with none it doesn't go. A step needs a floor
		// just below the next spot or just above this one, so the wave leaves a
		// floor's edge by a step, and it keeps the user's height (0x96cb93).
		template <typename Under, typename Over>
		int32_t steps(int32_t x, int32_t y, bool left, Under under, Over over)
		{
			int32_t dx = left ? -STEP : STEP;
			int32_t step = 0;

			for (; step < MAX_STEPS; step++, x += dx)
				if (!under(x + dx, y, FLOOR_REACH) && !over(x, y, FLOOR_REACH))
					break;

			return step;
		}

		// Its area's far side ends just past the wave (0x9512d4).
		inline int32_t far_side(int32_t x, int32_t count, bool left)
		{
			int32_t reach = count * STEP + EDGE;

			return left ? x - reach : x + reach;
		}
	}
}
