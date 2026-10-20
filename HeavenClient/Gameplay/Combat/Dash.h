#pragma once

#include <algorithm>
#include <cstdint>

namespace ms
{
	// Dash: a double tap on Left or Right casts it, and it lasts until the arrow is let go, the player attacks
	// or casts, or the server ends it (report 194-dash).
	namespace dash
	{
		constexpr int32_t ID = 5001005;
		constexpr int32_t DAWN_ID = 15001003;

		// How soon after the first press the second must come (FUN_0074ab67).
		constexpr int64_t WINDOW = 300;

		// The player's Dash (FUN_009433a2).
		inline int32_t skill(int32_t job)
		{
			return job / 1000 == 1 ? DAWN_ID : ID;
		}

		inline bool is_dash(int32_t skill)
		{
			return skill == ID || skill == DAWN_ID;
		}

		// What Dash adds to the speed a Speed buff of `buff` gives: the larger of the two counts, and a
		// slowing buff wins (0x8c457c).
		inline int32_t speed_bonus(int32_t buff, int32_t dash)
		{
			return buff < 0 ? 0 : std::max(buff, dash) - buff;
		}

		// The same for the jump, which has no slowing case (0x8c45e8).
		inline int32_t jump_bonus(int32_t buff, int32_t dash)
		{
			return std::max(buff, dash) - buff;
		}

		// The double tap (FUN_0074d993): an arrow down, up, and down again within WINDOW of the first press.
		// Any other key between drops it, and a key repeating its last event is ignored.
		class Command
		{
		public:
			// The arrows: 1 for Left, 2 for Right, 0 for any other key. Whether this event completes a
			// double tap; one only starts while the player isn't dashing.
			bool feed(int32_t arrow, bool down, int64_t now, bool dashing)
			{
				if (arrow == last_arrow && down == last_down)
					return false;

				last_arrow = arrow;
				last_down = down;

				if (step == 1 && arrow == key && !down)
				{
					step = 2;
					return false;
				}

				if (step == 2 && arrow == key && down && now <= start + WINDOW)
				{
					step = 0;
					return true;
				}

				step = 0;

				if (arrow != 0 && down && !dashing)
				{
					key = arrow;
					step = 1;
					start = now;
				}

				return false;
			}

		private:
			int32_t key = 0;
			uint8_t step = 0;
			int64_t start = 0;
			int32_t last_arrow = -1;
			bool last_down = false;
		};
	}
}
