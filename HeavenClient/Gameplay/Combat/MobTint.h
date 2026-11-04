#pragma once

#include <cstdint>

namespace ms
{
	// The colour a status's reason lays over a mob, by FUN_0066e9f7: each
	// update it walks the mob's reasons, the last with a colour wins, and it
	// sets the mob's layer's red and green-blue tones to that.
	namespace mob_tint
	{
		constexpr int32_t COMBO_TEMPEST = 21120006;

		constexpr uint32_t NONE = 0xFFFFFF;
		constexpr uint32_t ICE = 0xA9DDFE;
		constexpr uint32_t FIRE = 0xFFA0A0;
		constexpr uint32_t POISON = 0x5B8F3B;
		constexpr uint32_t ICE_DEMON = 0x00C0FF;
		constexpr uint32_t PARALYZE = 0x999999;

		inline uint32_t of_skill(int32_t skill)
		{
			switch (skill)
			{
			case 10004:
			case 1211005:
			case 1211006:
			case 2121005:
			case 2201004:
			case 2211002:
			case 2211006:
			case 2221007:
			case 3211003:
			case 3221005:
			case 5211005:
			case 21111005:
			case COMBO_TEMPEST:
			case 22121000:
				return ICE;
			case 2121003:
			case 5211004:
				return FIRE;
			case 2101005:
			case 2111003:
			case 2111006:
				return POISON;
			case 2221003:
				return ICE_DEMON;
			case 2121006:
				return PARALYZE;
			default:
				return NONE;
			}
		}

		// A player's skill names itself; anything else counts by its low 16
		// bits, which drops a mob skill's level.
		inline uint32_t of_reason(int32_t reason)
		{
			uint32_t tint = of_skill(reason);

			return tint != NONE ? tint : of_skill(static_cast<int16_t>(reason & 0xFFFF));
		}

		// `tint` once a status with `reason` is walked past.
		inline uint32_t over(uint32_t tint, int32_t reason)
		{
			uint32_t own = of_reason(reason);

			return own != NONE ? own : tint;
		}
	}
}
