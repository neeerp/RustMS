#pragma once

#include <algorithm>
#include <cstdint>
#include <vector>

namespace ms
{
	// Which reactor a basic attack swings at, and which of its events a
	// swing sets off, as v83 does.
	namespace reactor_rules
	{
		// The flags a swing sends with 0xCD and keeps for its event.
		constexpr int32_t FACING_LEFT = 1;
		constexpr int32_t ON_FOOTHOLD = 2;

		inline int32_t flags(bool left, bool grounded)
		{
			return (left ? FACING_LEFT : 0) | (grounded ? ON_FOOTHOLD : 0);
		}

		// How well an event of `type` suits a swing with `flags`, best at 0,
		// or -1 if it doesn't (FUN_00736091). Types 1 and 2 take only a swing
		// facing right or left, and 3 and 4 the same but only in the air.
		inline int32_t rank(int32_t flags, int32_t type)
		{
			bool left = flags & FACING_LEFT;
			bool grounded = flags & ON_FOOTHOLD;

			switch (type)
			{
				case 0:
					return grounded ? 1 : 2;
				case 1:
					return left ? -1 : grounded ? 0 : 1;
				case 2:
					return left ? grounded ? 0 : 1 : -1;
				case 3:
					return !left && !grounded ? 0 : -1;
				case 4:
					return left && !grounded ? 0 : -1;
				default:
					return -1;
			}
		}

		// Whether a state whose events have these types takes a swing: any
		// type from 0 to 4 does (FUN_007356c7).
		inline bool swingable(const std::vector<int32_t> &types)
		{
			return std::any_of(types.begin(), types.end(), [](int32_t type) { return type >= 0 && type < 5; });
		}

		// The event a swing with `flags` sets off: the first of the best
		// rank, or -1.
		inline int32_t pick_event(const std::vector<int32_t> &types, int32_t flags)
		{
			int32_t best = -1;
			int32_t best_rank = -1;

			for (size_t i = 0; i < types.size(); i++)
			{
				int32_t r = rank(flags, types[i]);

				if (r < 0 || (best >= 0 && r >= best_rank))
					continue;

				best = static_cast<int32_t>(i);
				best_rank = r;

				if (r == 0)
					break;
			}

			return best;
		}

		// A rectangle on the map, its right and bottom edges outside it.
		struct Box
		{
			int32_t left = 0;
			int32_t top = 0;
			int32_t right = 0;
			int32_t bottom = 0;
		};

		inline bool empty(const Box &box)
		{
			return box.right <= box.left || box.bottom <= box.top;
		}

		inline bool overlap(const Box &a, const Box &b)
		{
			return std::max(a.left, b.left) < std::min(a.right, b.right) &&
				   std::max(a.top, b.top) < std::min(a.bottom, b.bottom);
		}

		// The reactor a swing over `range` hits, as an index into `boxes`, or
		// -1 (FUN_0095719b). The boxes are in the order the reactors entered
		// the map, an empty one for each that can't take a swing. The first
		// whose middle passes v83's test for being in front of the user wins,
		// else the last the swing reaches. Facing left, v83 negates the
		// middle rather than the difference, so that test fails there for
		// any reactor right of x = 0.
		inline int32_t pick_target(const std::vector<Box> &boxes, const Box &range, int32_t user_x, bool left)
		{
			int32_t sign = left ? -1 : 1;
			int32_t last = -1;

			for (size_t i = 0; i < boxes.size(); i++)
			{
				const Box &box = boxes[i];

				if (empty(box) || !overlap(box, range))
					continue;

				if ((box.left + box.right) / 2 * sign - user_x >= 0)
					return static_cast<int32_t>(i);

				last = static_cast<int32_t>(i);
			}

			return last;
		}
	}
}
