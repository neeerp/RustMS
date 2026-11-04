#pragma once

#include <cstdint>

namespace ms
{
	// Assaulter: a strike at the mob in reach, and a dash past it onto the nearest floor ahead (0x950d83, 0x96cb93).
	namespace assaulter
	{
		constexpr int32_t ID = 4211002;

		// How far forward it dashes, its `lt.x` at every level (0x96cb93).
		constexpr int32_t DASH = 200;

		// The dash's element in the movement path (0x952df2).
		constexpr uint8_t MOVE = 7;

		// Both Barrages' action, whose afterimage says how far they reach.
		constexpr const char *BARRAGE_ACTION = "fist";

		// Whether a skill does nothing at all with no mob in its reach, where the others swing at the air: no
		// action, no cost, no packet (0x951342). Assaulter, both Energy Drains and both Barrages.
		inline bool needs_mob(int32_t skill)
		{
			return skill == ID || skill == 5111004 || skill == 15111001 || skill == 5121007 || skill == 15111004;
		}
	}
}
