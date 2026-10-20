#pragma once

#include <cstdint>

namespace ms::berserk
{
	constexpr int16_t DARK_KNIGHT = 132;

	// The damage percent Berserk adds, or 0 (FUN_00a292bb): a Dark Knight
	// who learned it is berserk at `x` percent of max HP or less.
	inline int32_t damage(int16_t job, int32_t level, int32_t hp, int32_t maxhp, int32_t x, int32_t damage)
	{
		if (job != DARK_KNIGHT || level <= 0)
			return 0;

		return hp <= maxhp * x / 100 ? damage : 0;
	}

	// Whether the client tells the server: only when Berserk turns on or
	// off, not each time HP changes.
	inline bool changed(int32_t was, int32_t now)
	{
		return (was != 0) != (now != 0);
	}
}
