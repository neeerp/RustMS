#pragma once

#include "Attack.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>

namespace ms
{
	namespace hit_timing
	{
		// How long after an attack's first hit its `target`th mob (in target
		// order) is hit, as in the MapleRoyals client. Every hit on one mob
		// lands at the same time.
		inline uint16_t target_delay(Attack::Type type, int32_t skill, size_t target)
		{
			switch (type)
			{
			case Attack::Type::CLOSE:
				// Meso Explosion hits every mob at once (FUN_00980c90).
				if (skill == 4211006)
					return 0;

				// Each mob 70 ms after the last, up to 280 ms (FUN_00950921).
				return static_cast<uint16_t>(std::min<size_t>(target * 70, 280));
			case Attack::Type::MAGIC:
				// FUN_0095571f.
				switch (skill)
				{
				case 2201005: // Thunderbolt
				case 2301002: // Heal
				case 2111002: // Explosion
				case 2211002: // Ice Strike
				case 2311004: // Shining Ray
				case 2121001: // Big Bang
				case 2221001:
				case 2321001:
				case 4121004: // Ninja Ambush
				case 4221004:
				case 12101006: // Fire Pillar
					return static_cast<uint16_t>(std::min<size_t>(target * 50, UINT16_MAX));
				case 2221006: // Chain Lightning
					return static_cast<uint16_t>(std::min<size_t>(target * 100, UINT16_MAX));
				default:
					return 0;
				}
			default:
				return 0;
			}
		}
	}
}
