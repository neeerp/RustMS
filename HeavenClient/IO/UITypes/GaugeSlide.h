#pragma once

#include <cstdint>

// The status bar's HP and MP gauges (v83 0x8d850b). Each is a gray layer over the full bar, as wide as the part
// that is missing. A change moves it linearly over 700 ms, from where it is shown to the new width, for a gain or
// a loss alike (RelMove with vTime = now + 0x2bc). The EXP gauge and the numbers change at once.
namespace ms::gauge_slide
{
	constexpr int64_t TIME = 700;

	// The gray width over a gauge `width` wide: width - value * width / max (0x8d8b43, 0x8d92c3).
	inline int32_t gray(int64_t value, int64_t max, int32_t width)
	{
		if (max <= 0)
			return 0;

		return width - static_cast<int32_t>(value * width / max);
	}

	struct Slide
	{
		double from = 0.0;
		double to = 0.0;
		int64_t start = 0;
		bool seeded = false;

		// The width shown at `now`.
		double at(int64_t now) const
		{
			if (now - start >= TIME)
				return to;

			return from + (to - from) * static_cast<double>(now - start) / TIME;
		}

		// Aims at `target` from what is shown now. The first aim shows it at once, so a login shows the gauge as
		// it is.
		void aim(double target, int64_t now)
		{
			if (!seeded)
			{
				from = to = target;
				start = now - TIME;
				seeded = true;
			} else if (target != to)
			{
				from = at(now);
				to = target;
				start = now;
			}
		}
	};
}
