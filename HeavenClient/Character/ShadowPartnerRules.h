#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>

namespace ms
{
	// Shadow Partner's clone, the dark double drawn behind its user in the
	// skill's `special` frames (FUN_0092efe9).
	namespace shadow_partner
	{
		// The skill whose frames draw it: the Night Walker's for a Cygnus
		// Knight (0x92f33d).
		constexpr int32_t skill_for(int32_t job)
		{
			return job / 1000 == 1 ? 14111000 : 4111002;
		}

		// How far behind its user the clone stands, how far below them it
		// hangs on a ladder or rope, and how long it takes to slide to a new
		// place (RelMove).
		constexpr int16_t BEHIND = 30;
		constexpr int16_t BELOW = 50;
		constexpr int16_t SLIDE_MS = 200;

		struct Offset
		{
			int16_t x;
			int16_t y;

			constexpr bool operator==(const Offset &other) const
			{
				return x == other.x && y == other.y;
			}
		};

		// Where the clone stands from its user.
		constexpr Offset offset(bool facing_right, bool climbing)
		{
			if (climbing)
				return { 0, BELOW };

			return { static_cast<int16_t>(facing_right ? -BEHIND : BEHIND), 0 };
		}

		// Whether the clone shows: the buff on and its user alive, and not
		// riding, which turns it invisible (0x00ffffff).
		constexpr bool shown(bool buff, bool dead, bool riding)
		{
			return buff && !dead && !riding;
		}

		// Whether a shown clone plays its `dead` frames as it goes: when the
		// buff ends or its user dies, not when they mount.
		constexpr bool dies(bool buff, bool dead)
		{
			return !buff || dead;
		}

		// The clone's frame for the avatar's: its frames have no delay of
		// their own and step with the avatar's, held on the last when its
		// action has fewer.
		constexpr size_t frame(uint8_t avatar_frame, size_t count)
		{
			return count == 0 ? 0 : std::min<size_t>(avatar_frame, count - 1);
		}

		// How far the clone has slid from one place to the next after
		// `elapsed` ms.
		constexpr int16_t slid(int16_t from, int16_t to, int16_t elapsed)
		{
			int32_t step = std::min<int32_t>(std::max<int32_t>(elapsed, 0), SLIDE_MS);

			return static_cast<int16_t>(from + (to - from) * step / SLIDE_MS);
		}
	}
}
