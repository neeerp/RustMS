#pragma once

#include <cstdint>

namespace ms
{
	// How many times the player may attack, or use a skill that sends where
	// it stands, without moving: v83 counts each from the spot it last moved
	// to and silently sends nothing more past its limit (FUN_009536d0,
	// report b377-same-spot). A new map starts both over (0x948c44).
	namespace same_spot
	{
		// The attacks of every kind share one limit, the skills another.
		constexpr int32_t ATTACKS = 100;
		constexpr int32_t SKILLS = 20;

		class Limiter
		{
		public:
			explicit Limiter(int32_t limit) : limit(limit) {}

			// Whether the player at (x, y) may go on. Moving 6 px across or
			// 150 px up or down makes it a new spot, which counts from none.
			bool allows(int32_t at_x, int32_t at_y)
			{
				int32_t dx = x - at_x;
				int32_t dy = y - at_y;

				if (dx > -6 && dx < 6 && dy > -150 && dy < 150)
				{
					if (count >= limit)
						return false;

					count++;

					return true;
				}

				x = at_x;
				y = at_y;
				count = 0;

				return true;
			}

		private:
			int32_t limit;
			int32_t x = 0;
			int32_t y = 0;
			int32_t count = 0;
		};

		// The shots of Hurricane, Rapid Fire and the Wind Archer's Hurricane
		// aren't counted (0x95416b).
		inline bool counts_shot(int32_t skill)
		{
			return skill != 3121004 && skill != 5221004 && skill != 13111002;
		}
	}
}
