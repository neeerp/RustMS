#pragma once

#include <cstdint>

namespace ms
{
	// Soul Arrow, which shoots without arrows, as in the MapleRoyals client
	// (FUN_009537d5).
	namespace soul_arrow
	{
		// The skills whose ball every Soul Arrow shot flies, whichever line
		// cast it: the Hunter's, or the Wind Archer's for a Cygnus Knight
		// (0x9546c9).
		constexpr int32_t BALL = 3101004;
		constexpr int32_t KNIGHTS_BALL = 13101003;

		// Its bit in an attack's display byte (0x955232).
		constexpr uint8_t DISPLAY = 0x02;

		constexpr int32_t ball_skill(int32_t job)
		{
			return job / 1000 == 1 ? KNIGHTS_BALL : BALL;
		}

		constexpr bool is_ball(int32_t bullet)
		{
			return bullet == BALL || bullet == KNIGHTS_BALL;
		}
	}
}
