#pragma once

#include <cmath>
#include <cstdint>

namespace ms
{
	// How much a monster's body or physical attack takes from the player, by
	// the pre-Big Bang procedure. The monster rolls a base damage between 85%
	// and 100% of its attack. The player's defense counts a hidden part from
	// their stats on top of the W.DEF from equipment, and three quarters of
	// it comes off the base damage. A hit always takes at least 1.
	namespace touch_damage
	{
		// The lowest base damage; the base stays below the attack itself.
		inline int32_t min_base(int32_t mobatk)
		{
			return static_cast<int32_t>(mobatk * 0.85);
		}

		// The part of the player's defense that a hit loses.
		inline int32_t defense(int32_t str, int32_t dex, int32_t intl, int32_t luk, int32_t wdef)
		{
			double total = 1.2 * str + 0.5 * dex + 0.4 * intl + 0.5 * luk + wdef;

			return static_cast<int32_t>(std::floor(total * 0.75));
		}

		// The damage of a hit with the given base. Skills like Achilles reduce
		// it by a fraction afterwards.
		inline int32_t damage(int32_t base, int32_t defense, float reduce)
		{
			int32_t damage = base - defense;
			damage -= static_cast<int32_t>(damage * reduce);

			return damage < 1 ? 1 : damage;
		}
	}
}
