#pragma once

#include <cstdint>

namespace ms
{
	// A cash weapon (1702xxx, worn at -111) has art for each weapon type it
	// covers, under Weapon/<id>.img/<type>. v83 draws the regular weapon's
	// type from it, and no weapon when it has none (0x41272c, 0x412d63);
	// the regular weapon keeps its stances, sounds and speed.
	namespace cash_weapon
	{
		// v83 0x460aa0: id/10000 % 100 for the weapon types, else 0.
		inline int32_t type(int32_t weapon)
		{
			int32_t type = weapon / 10000 % 100;

			if (weapon / 1000000 != 1 || type < 30 || type > 49 || type > 33 && type < 37)
				return 0;

			return type;
		}
	}
}
