#pragma once

#include "../../Template/Point.h"

#include <array>
#include <cstdint>
#include <cstdlib>

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

		// Whether a pet in Wing Boots walks toward a drop at `spot`
		// (FUN_00508268): within 124 px to either side of its owner, 274
		// with a Binocular (`longrange`), and from 49 px above the pet's
		// feet to 9 px below them.
		inline bool in_sweep_reach(Point<int16_t> owner, Point<int16_t> pet, Point<int16_t> spot, bool longrange)
		{
			int32_t reach = longrange ? 274 : 124;

			return std::abs(spot.x() - owner.x()) <= reach && spot.y() >= pet.y() - 49 && spot.y() <= pet.y() + 9;
		}

		// Which way a pet walks to a drop at `dropx`: left (-1) for one at or
		// left of its feet, else right (1).
		inline int8_t sweep_direction(int16_t dropx, int16_t petx)
		{
			return dropx <= petx ? -1 : 1;
		}

		// The pet of those that `can` reach a drop at `dropx` that walks to
		// it: the nearest across, the first on a tie, or -1 for none.
		inline int8_t sweeper_for(const std::array<int16_t, 3> &petx, const std::array<bool, 3> &can, int16_t dropx)
		{
			int8_t found = -1;
			int32_t nearest = 0;

			for (int8_t i = 0; i < 3; i++)
			{
				if (!can[i])
					continue;

				int32_t gap = std::abs(dropx - petx[i]);

				if (found < 0 || gap < nearest)
				{
					found = i;
					nearest = gap;
				}
			}

			return found;
		}

		// A pet walks only once its owner has stood still for more than
		// 100 ms (FUN_009c4122).
		inline bool sweep_ready(int32_t still)
		{
			return still > 100;
		}

		// Whether the owner is near enough that its pet isn't called back
		// (FUN_009c4122): from 300 px left of the pet to just short of 300 px
		// right of it, and from 200 px above to just short of 200 px below.
		inline bool within_leash(Point<int16_t> pet, Point<int16_t> owner)
		{
			int32_t dx = owner.x() - pet.x();
			int32_t dy = owner.y() - pet.y();

			return dx >= -300 && dx < 300 && dy >= -200 && dy < 200;
		}
	}
}
