#pragma once

#include <algorithm>
#include <cstdint>
#include <cstdlib>

namespace ms
{
	// How the guild and alliance rank panels slide out from behind the user
	// list and back (v83 report b333). The panel is built at the window's
	// left edge, under the list, and its x moves 1 px a millisecond to 311
	// out and back to 0 (FUN_0091e37e). Closing counts the distance down
	// by 40 a 30 ms tick and drops the panel below 0 (FUN_0091e7fa), as it
	// is hidden. Opened again before that, the guild's panel is the same
	// one, edits and all.
	struct RankPanelSlide
	{
		static constexpr int16_t TRAVEL = 0x137;
		// The countdown's step and the tick it is taken on.
		static constexpr int32_t STEP = 40;
		static constexpr int32_t TICK = 30;

		// Whether the panel exists, and whether it is out or going out.
		bool alive = false;
		bool out = false;
		// Its x from the window's left.
		int16_t x = 0;
		// The distance left before a closing panel goes, times TICK.
		int32_t countdown = 0;

		// Slides the panel out; true if it was built anew. `reuse` keeps a
		// panel still sliding back (the guild's; the alliance's builds anew,
		// FUN_0091e66f).
		bool open(bool reuse)
		{
			bool fresh = !alive || !reuse;

			if (fresh)
			{
				alive = true;
				x = 0;
			}

			slide(true);

			return fresh;
		}

		// Slides an open panel back.
		void close()
		{
			if (alive && out)
				slide(false);
		}

		// Drops the panel at once, as the window closes.
		void drop()
		{
			alive = false;
			out = false;
			x = 0;
			countdown = 0;
		}

		// Moves the panel on by `ms`.
		void update(int32_t ms)
		{
			if (!alive)
				return;

			int16_t target = out ? TRAVEL : 0;
			int32_t step = std::min<int32_t>(ms, std::abs(target - x));
			x += static_cast<int16_t>(target > x ? step : -step);

			if (!out)
			{
				countdown -= STEP * ms;

				if (countdown < 0)
					drop();
			}
		}

	private:
		void slide(bool going_out)
		{
			countdown = std::abs(x - (going_out ? TRAVEL : 0)) * TICK;
			out = going_out;
		}
	};
}
