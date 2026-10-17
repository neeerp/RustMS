#pragma once

#include "MagicDamage.h"

#include "../../Template/Rectangle.h"

#include <cstdint>

namespace ms
{
	// Heal, as in the MapleRoyals client (FUN_0096a69d): it tells the server
	// whom in the party it heals, then hurts the undead around the user as
	// a spell.
	namespace heal
	{
		constexpr int32_t ID = 2301002;

		// It hits the undead in 60% of the skill's area (0x9563a8).
		inline Rectangle<int16_t> area(const Rectangle<int16_t> &range)
		{
			auto scale = [](int16_t side)
			{
				return static_cast<int16_t>(side * 0.6);
			};

			return {
					scale(range.left()),
					scale(range.right()),
					scale(range.top()),
					scale(range.bottom())
			};
		}

		// What a hit on an undead monster comes from in place of the spell's
		// magic attack (0x791aa8): the skill's `hp` percent, the player's
		// magic, INT and LUK, and how many it reaches, mobs and party members
		// together. The more it reaches, the less each gets.
		inline double base(int32_t hp, int32_t magic, int32_t intl, int32_t luk, int32_t reached, double roll)
		{
			if (reached < 1)
				reached = 1;

			double rolled = magic_damage::between(intl * 0.2, intl * 0.8, roll);

			return (reached * 0.3 + 1.0) * hp * 0.01 * (rolled * 1.5 + luk) * magic * 0.005 / reached;
		}
	}
}
