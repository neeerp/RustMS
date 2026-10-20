#pragma once

#include <algorithm>
#include <cstdint>

namespace ms
{
	// Energy Charge: the gauge a Marauder's (or a Thunder Breaker's) hits fill. A full gauge, 10000, frees the
	// Energy skills and lends the player the skill's own attack, accuracy and avoid.
	namespace energy
	{
		constexpr int32_t FULL = 10000;

		// Energy Charge for a job's line.
		inline int32_t skill(int32_t job)
		{
			return job / 1000 == 1 ? 15100004 : 5110001;
		}

		// Energy Blast, Energy Drain, Energy Orb and the Thunder Breaker's two need a full gauge
		// (FUN_00969465, FUN_009696a2).
		inline bool needs_full(int32_t skill)
		{
			switch (skill)
			{
				case 5111002:
				case 5111004:
				case 5121002:
				case 15101005:
				case 15111001:
					return true;
				default:
					return false;
			}
		}

		// What a full gauge adds to a stat whose buffs come to `buffed`: v83 takes the larger of the two,
		// the skill's `own` pad, acc or eva (0x77dd8c, 0x77de1c, 0x77deb2).
		inline int32_t bonus(int32_t buffed, int32_t own)
		{
			return std::max(buffed, own) - buffed;
		}
	}
}
