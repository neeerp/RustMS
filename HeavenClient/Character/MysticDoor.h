#pragma once

#include "../Template/Point.h"

#include <cstdint>

namespace ms
{
	// A Mystic Door as its owner and their party know it: the town it leads
	// to, the field it stands in, and where in the field. v83 keeps the
	// player's own (SPAWN_PORTAL) at context +0x2ec0 and each member's (party
	// op 0x25) in the party's door table at +0xca.
	struct MysticDoor
	{
		// The map id where there is no door.
		static constexpr int32_t NONE = 999999999;

		int32_t town = NONE;
		int32_t field = NONE;
		Point<int16_t> position;

		// Whether it stands in town `mapid` with a field to lead back to
		// (FUN_005375ed).
		bool leads_from(int32_t mapid) const
		{
			return town == mapid && field != 0 && field != NONE;
		}
	};
}
