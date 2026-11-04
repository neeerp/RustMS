#pragma once

#include <cstdint>

namespace ms
{
	// Boomerang Step: the step out and back is its effect's art, which stands in for the user. They are gone from
	// sight and from harm while it plays, and stay where they were, but it wants a floor where it steps
	// (0x950d38, 0x951857, 0x9325cc, 0x9581f2).
	namespace boomerang_step
	{
		constexpr int32_t ID = 4221007;

		// How far ahead it wants its floor: half its `lt.x`, at every level (0x96cc3b).
		constexpr int32_t REACH = 150;

		// How long the user is gone: the length of its action `alert5`, and of its effect's 15 frames of
		// 60 ms (0x951857, 0x980f6a).
		constexpr int32_t GONE = 900;

		// How long after that it stays refused (0x950d41, 0x932201).
		constexpr int32_t REST = 500;

		// Whether it is refused now, given when its user last came back, or 0.
		inline bool resting(int64_t now, int64_t back)
		{
			return back != 0 && now - back <= REST;
		}
	}
}
