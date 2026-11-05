#pragma once

#include <cstdint>

namespace ms
{
	// v83 reads the mouse itself and makes a left press a double click when it
	// comes within GetDoubleClickTime of the last plain press and inside
	// SM_CXDOUBLECLK and SM_CYDOUBLECLK of its place (59ad20). The press after
	// a double click is a plain one again.
	class DoublePress
	{
	public:
		// Windows' defaults.
		static constexpr int64_t TIME_MS = 500;
		static constexpr double REACH = 4.0;

		// Whether a press now, at this place, is a double click's second.
		bool press(int64_t now_ms, double x, double y)
		{
			bool second = armed && now_ms - pressed_ms <= TIME_MS && at_first(x, y);

			armed = !second;

			if (!second)
			{
				pressed_ms = now_ms;
				first_x = x;
				first_y = y;
			}

			return second;
		}

		// Whether this place is inside the reach of the last plain press.
		bool at_first(double x, double y) const
		{
			return x >= first_x - REACH && x < first_x + REACH && y >= first_y - REACH && y < first_y + REACH;
		}

	private:
		bool armed = false;
		int64_t pressed_ms = 0;
		double first_x = 0.0;
		double first_y = 0.0;
	};
}
