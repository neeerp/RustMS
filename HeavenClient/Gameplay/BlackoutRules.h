#pragma once

#include "Combat/PqSkills.h"

#include <algorithm>
#include <cstdint>
#include <vector>

// The black that falls over the field with Bamboo Rain and Rage of Pharaoh: an 800x600 black layer
// fixed on the screen, just behind the local player (their z less 100), fading in over 720 ms,
// staying 3200 ms and fading out over 520 ms (FUN_004370cd, called at 0x951b59/0x951bb8 for one's
// own and 0x980973/0x9809be for someone else's). Not gated by the ShakeScreen option.
namespace ms::blackout
{
	constexpr int32_t FADE_IN = 720;
	constexpr int32_t HOLD = 3200;
	constexpr int32_t FADE_OUT = 520;
	constexpr int32_t LENGTH = FADE_IN + HOLD + FADE_OUT;

	// How opaque one layer is this long after it began.
	inline float alpha(int32_t elapsed)
	{
		if (elapsed < 0 || elapsed >= LENGTH)
			return 0.0f;

		if (elapsed < FADE_IN)
			return static_cast<float>(elapsed) / FADE_IN;

		if (elapsed < FADE_IN + HOLD)
			return 1.0f;

		return 1.0f - static_cast<float>(elapsed - FADE_IN - HOLD) / FADE_OUT;
	}

	// How many layers a skill lays: one's own Bamboo Rain two (the 1009 test and the 1009-or-1020
	// test both pass), one's own Rage of Pharaoh one, but none for the Evan copy 20011020, which the
	// own test leaves out; someone else's either one.
	inline int32_t layers(int32_t skill, bool own)
	{
		switch (pq_skills::kind(skill))
		{
			case pq_skills::Kind::BAMBOO_RAIN:
				return own ? 2 : 1;
			case pq_skills::Kind::RAGE_OF_PHARAOH:
				return own && skill == 20011020 ? 0 : 1;
			default:
				return 0;
		}
	}

	// The layers under way, each by when it began.
	struct State
	{
		std::vector<int32_t> starts;

		void add(int32_t now, int32_t count)
		{
			starts.insert(starts.end(), std::max(count, 0), now);
		}

		// Drops the layers that have ended, and gives how opaque they are together.
		float update(int32_t now)
		{
			starts.erase(std::remove_if(starts.begin(), starts.end(),
										[now](int32_t start) { return now - start >= LENGTH; }),
						 starts.end());

			float clear = 1.0f;

			for (int32_t start : starts)
				clear *= 1.0f - alpha(now - start);

			return 1.0f - clear;
		}
	};
}
