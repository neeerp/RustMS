#pragma once

#include <cstdint>

#include "../../Template/Point.h"

// The tutorial guide that follows a new player, without the map (v83
// SPAWN_GUIDE 223 and TALK_GUIDE 224, reports b097 and b099).
namespace ms::guide_rules
{
	// Its look is a summon's: the Cygnus fairy, or the one everyone else
	// gets (0x96014f).
	constexpr int32_t CYGNUS = 10001013;
	constexpr int32_t OTHERS = 20001013;

	inline int32_t skill_for(int32_t job)
	{
		return job / 1000 == 1 ? CYGNUS : OTHERS;
	}

	// A hint ends with `say`, for anyone but Aran and Evan (0x7a958e).
	inline bool says_after_hint(int32_t job)
	{
		return job / 1000 != 2;
	}

	// Its summon/height when the skill has none.
	constexpr int16_t DEFAULT_HEIGHT = 80;

	// It goes when a level-up reaches 11 (FUN_00a1fb52).
	inline bool leaves(int32_t before, int32_t after)
	{
		return before < after && after == 11;
	}

	// A balloon or hint stays `duration` ms, and goes on the first update
	// after (start + duration < now, 0x7a95b5).
	inline bool expired(int32_t elapsed, int32_t duration)
	{
		return elapsed > duration;
	}

	// The balloon (FUN_007a7b46): text `width` px wide and `height` tall in
	// a frame of ChatBalloon.img/tutorial. Its top left is this far from
	// the guide's foot, `guide` px tall.
	inline Point<int16_t> balloon_corner(int16_t width, int16_t height, int16_t guide)
	{
		return Point<int16_t>(static_cast<int16_t>(-(width / 2)), static_cast<int16_t>(-90 - height - guide));
	}

	// The frame's corners are 19 px wide, the top 21 tall and the bottom
	// 19; the text starts inside the top-left corner.
	constexpr int16_t SIDE = 19;
	constexpr int16_t TOP = 21;

	// The arrow hangs under the text, right of its middle.
	inline Point<int16_t> balloon_arrow(int16_t width, int16_t height)
	{
		return Point<int16_t>(static_cast<int16_t>(width / 2 + 10), static_cast<int16_t>(height + 35));
	}

	// A hint's picture (FUN_007a9283) is centred over the guide, its bottom
	// edge the guide's height above its foot.
	inline Point<int16_t> hint_corner(Point<int16_t> size, int16_t guide)
	{
		return Point<int16_t>(static_cast<int16_t>(-(size.x() / 2)), static_cast<int16_t>(-(size.y() + guide)));
	}
}
