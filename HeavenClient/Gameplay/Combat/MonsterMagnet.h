#pragma once

#include <cstdint>
#include <cstdlib>

namespace ms
{
	// Monster Magnet, as in the MapleRoyals client: let go, it catches the
	// mobs in front of the user (0x96c215), and pulls them in 300 ms later
	// (0x66b6fc).
	namespace magnet
	{
		inline bool is_magnet(int32_t skill)
		{
			return skill == 1121001 || skill == 1221001 || skill == 1321001;
		}

		// It reaches no more mobs than this, or isn't used at all.
		constexpr int32_t MAX_COUNT = 15;
		// Its search starts this far in front of the user.
		constexpr int16_t START = 50;
		// How long after it is let go the mobs it caught are pulled.
		constexpr int16_t HIT_DELAY = 300;
		// A mob caught stops this far in front of its stand frame's width.
		constexpr int16_t REACH = 45;
		// The pull's steps, in ms, and its speeds, in px/s.
		constexpr int16_t STEP = 30;
		constexpr double PULL_SPEED = 1000.0;
		constexpr double FLY_SPEED = 100.0;
		// Within this much more than its reach, a mob slows to a third each
		// step.
		constexpr int16_t SLOW_MARGIN = 40;

		// How many mobs it reaches when held for `charge` of `cap` ms: one,
		// and the rest of its mob count as it charges.
		inline int32_t count(int32_t mobcount, int32_t charge, int32_t cap)
		{
			if (cap == 0)
				return mobcount;

			return (mobcount - 1) * charge / cap + 1;
		}

		// Whether it may catch a mob: never a boss, nor one that can't move.
		// The rest are caught `prop` percent of the time, with `roll` from
		// 0 to 99.
		inline bool catches(bool boss, bool canmove, int32_t prop, int32_t roll)
		{
			return !boss && canmove && roll < prop;
		}

		// How fast a caught mob at `x` starts toward the user at `user_x`:
		// not at all if it is already within `reach` of them.
		inline double start_speed(int16_t x, int16_t user_x, int16_t reach)
		{
			if (std::abs(x - user_x) < reach)
				return 0.0;

			return user_x < x ? -PULL_SPEED : PULL_SPEED;
		}

		// Its speed for the next step: a third as fast once it nears the
		// user.
		inline double step_speed(double speed, int16_t x, int16_t user_x, int16_t reach)
		{
			if (std::abs(x - user_x) < reach + SLOW_MARGIN)
				return speed / 3;

			return speed;
		}

		// A flier rises or sinks toward the user as well.
		inline double fly_speed(int16_t y, int16_t user_y)
		{
			return user_y > y ? FLY_SPEED : -FLY_SPEED;
		}
	}
}
