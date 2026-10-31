#pragma once

#include "FlashJump.h"

#include <cstdint>

namespace ms
{
	// Combat Step, which slides an Aran along the ground, as in the
	// MapleRoyals client (FUN_0096d95e, report b366).
	namespace combat_step
	{
		constexpr int32_t ID = 21001001;

		// Its path element, which other clients see it by (0x980308).
		constexpr uint8_t MOVE = 0x14;

		// The action it plays (0x88).
		constexpr const char *ACTION = "combatStep";

		// It may step again 1 s after it stepped (0x9535e3), or 200 ms
		// after the jump key, whichever came last (0x94f5c7).
		constexpr int64_t REUSE_DELAY = 1000;
		constexpr int64_t JUMP_DELAY = 200;

		// The push, in pixels per second: the level's x forward, and none
		// up (FUN_007a6353).
		inline flash_jump::Impulse impulse(int32_t x, bool left)
		{
			return { left ? -static_cast<double>(x) : static_cast<double>(x), 0.0 };
		}

		// It steps only from a foothold, where the map lets movement skills
		// go or for a GM, once ready, and not while a push is waiting. It
		// says nothing when it doesn't.
		inline bool may_step(bool onground, bool climbing, bool allowed, int64_t now, int64_t ready_at, bool pushed)
		{
			return onground && !climbing && allowed && now >= ready_at && !pushed;
		}
	}
}
