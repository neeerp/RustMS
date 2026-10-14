#pragma once

#include <cstdint>

namespace ms
{
	// Where the thumb of a horizontal slider sits. The thumb stops at one of
	// steps + 1 places spread evenly over travel, the track width minus the
	// thumb width.
	namespace slider_steps
	{
		// The thumb's left edge for a step, counted from the track's left edge.
		inline int16_t thumb_x(int16_t step, int16_t steps, int16_t travel)
		{
			if (steps <= 0)
				return 0;

			return static_cast<int16_t>((travel * step + steps / 2) / steps);
		}

		// The step closest to a thumb left edge at x, which may be past either end.
		inline int16_t nearest(int16_t x, int16_t steps, int16_t travel)
		{
			if (steps <= 0 || travel <= 0 || x <= 0)
				return 0;

			if (x >= travel)
				return steps;

			return static_cast<int16_t>((x * steps + travel / 2) / travel);
		}
	}
}
