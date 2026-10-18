#pragma once

#include "../../Template/Point.h"

#include <cstdint>

namespace ms
{
	// What the v83 client checks before asking to pick up a drop
	// (CDropPool::TryPickUpDrop, FUN_0050463c).
	namespace pickup
	{
		// A drop isn't tried again for 3 s after the last try.
		constexpr int32_t RETRY = 3000;
		// For 15 s after it appears, a drop owned by another character or
		// party is left alone.
		constexpr int32_t OWNERSHIP = 15000;

		// Whether a drop's spot is within 25 px to either side of the
		// player's feet, from 50 px above them to 10 px below.
		inline bool in_reach(Point<int16_t> feet, Point<int16_t> spot)
		{
			return spot.x() >= feet.x() - 25 && spot.x() < feet.x() + 25
				   && spot.y() >= feet.y() - 50 && spot.y() < feet.y() + 10;
		}

		// Whether someone else still owns a drop of this type (0 a
		// character, 1 a party, 2 anyone, 3 explosive) that appeared this
		// many milliseconds ago.
		inline bool owned_by_other(int8_t type, int32_t owner, int32_t self, int32_t party, int32_t age)
		{
			if (age >= OWNERSHIP)
				return false;

			return (type == 0 && owner != self) || (type == 1 && owner != party);
		}

		// Whether a pet leaves a drop to someone else (FUN_0050483a). Unlike
		// its owner it never takes another's drop, unless it wears Magic
		// Scales (`others`), and then only once 15 s have passed.
		inline bool pet_owned_by_other(int8_t type, int32_t owner, int32_t self, int32_t party, int32_t age,
									   bool others)
		{
			if (others && age > OWNERSHIP)
				return false;

			return (type == 0 && owner != self) || (type == 1 && owner != party);
		}
	}
}
