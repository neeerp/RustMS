#pragma once

#include "FlashJump.h"

namespace ms
{
	// Recoil Shot, which shoots and throws the user back up and away from
	// the shot, as in the MapleRoyals client (FUN_009537d5).
	namespace recoil_shot
	{
		constexpr int32_t ID = 5201006;

		// Its path element, which other clients see it by.
		constexpr uint8_t MOVE = 13;

		// It may be shot again only 2 s after it was (0x953a79, set 0x954336).
		constexpr int32_t REUSE_DELAY = 2000;

		// The push, in pixels per second: 250 back and 250 up, with 40 more
		// back and 20 more up for every four levels.
		inline flash_jump::Impulse backstep(int32_t level, bool left)
		{
			int32_t steps = level / 4;
			double vx = 250.0 + 40.0 * steps;

			return { left ? vx : -vx, -250.0 - 20.0 * steps };
		}
	}
}
