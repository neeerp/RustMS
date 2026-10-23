#pragma once

#include <cmath>
#include <cstdint>

namespace ms
{
	// How loud a sound from somewhere else in the field plays, in percent of
	// the effects volume (FUN_009894f3): full within 250 px of the player,
	// falling linearly to 40 at 1000 px, and 40 beyond.
	namespace sound_distance
	{
		inline uint8_t volume(int32_t dx, int32_t dy)
		{
			double d = std::sqrt(static_cast<double>(dx) * dx + static_cast<double>(dy) * dy + 0.001);

			if (d < 250.0)
				return 100;

			if (d <= 1000.0)
				return static_cast<uint8_t>(120.0 - 0.08 * d);

			return 40;
		}
	}
}
