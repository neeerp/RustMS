#pragma once

#include <cstdint>

namespace ms
{
	// Wings: the Gunslinger jumps and, once falling, glides down at a set speed until landing, grabbing a rope,
	// or five seconds after the cast (report 194-wings).
	namespace wings
	{
		constexpr int32_t ID = 5201005;

		// The path elements: one at the jump (0x9b21ed), then the glide in place of the usual walk and fall.
		constexpr uint8_t TAKEOFF = 16;
		constexpr uint8_t GLIDE = 17;

		inline bool is_wings(int32_t skill)
		{
			return skill == ID;
		}

		// The glide's fall speed in pixels per second: the level's `x` thousandths of the jump speed (0x9b2ce8).
		inline double fall_speed(int32_t x)
		{
			return x * 555.0 / 1000.0;
		}

		// It starts only from a foothold or in the water, and not while the player acts (0x957a9a, 0x957af0).
		inline bool may_cast(bool onground, bool swimming, bool acting)
		{
			return (onground || swimming) && !acting;
		}

		// Gliding is falling with Wings on; rising keeps the usual gravity (FUN_00932ce4).
		inline bool gliding(bool active, double vspeed)
		{
			return active && vspeed > 0.0;
		}

		// The deadline, `left` milliseconds away, ends only a glide, so a jump still rising when it passes ends on
		// the way down (0x94a834).
		inline bool expired(int32_t left, bool gliding)
		{
			return gliding && left <= 0;
		}
	}
}
