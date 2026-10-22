#pragma once

#include <cstdint>

namespace ms
{
	// How a v83 pet follows its owner on the ground (CVecCtrlPet::update,
	// FUN_009c4122). Its mind steps every 30 ms.
	namespace petfollow
	{
		constexpr int32_t TICK = 30;
		// A run lasts 700 ms at twice the pet's walking speed, and while it
		// lasts the pet acts 100 px further from its spot than it is.
		constexpr int32_t BURST = 700;
		constexpr int32_t RUN_BONUS = 100;
		constexpr double BURST_SPEED = 2.0;

		// The role of the pet in `slot` (0-2), by which of the owner's other
		// slots hold one: 0 alone, 1 and 2 for two, 5, 3 and 4 for three.
		inline uint8_t role(bool second, bool third, uint8_t slot)
		{
			if (third)
				return slot == 0 ? 5 : slot == 1 ? 3 : 4;

			if (second)
				return slot == 0 ? 1 : 2;

			return 0;
		}

		// Where a pet heads (`x`) and how near it must get (`radius`).
		struct Goal
		{
			int32_t x;
			int32_t radius;
		};

		// The spot of a pet of `role` for an owner at `ownerx`: 160 px ahead
		// of them, at them, or 40 px behind.
		inline Goal goal(uint8_t role, int32_t ownerx, bool owner_left)
		{
			switch (role)
			{
				case 1:
					return { owner_left ? ownerx - 160 : ownerx, 80 };
				case 2:
					return { owner_left ? ownerx : ownerx + 160, 80 };
				case 3:
					return owner_left ? Goal{ ownerx - 160, 80 } : Goal{ ownerx - 40, 70 };
				case 4:
					return owner_left ? Goal{ ownerx + 40, 70 } : Goal{ ownerx + 160, 80 };
				case 5:
					return { ownerx, 60 };
				default:
					return { ownerx, 80 };
			}
		}

		// How far past its spot a walking pet goes before it turns back:
		// 80 px, 170 with a Binocular.
		inline int32_t turn_distance(bool longrange)
		{
			return (longrange ? 150 : 60) + 20;
		}

		// Which way a pet walks (-1, 0, 1) at `x`, `away` px from its spot
		// `goal` (run bonus included), having walked `held`. It stops once
		// nearer than `goal.radius`, starts only once farther, and turns only
		// past `turn`, unless it is sweeping or didn't move last step
		// (`stuck`).
		inline int8_t walk(int32_t away, const Goal &goal, int32_t turn, bool sweeping, bool stuck, int8_t held, int32_t x)
		{
			if (away < goal.radius)
				return 0;

			if (sweeping || (stuck && away > goal.radius) || away > turn)
				return x <= goal.x ? 1 : -1;

			return held;
		}

		// Whether a pet that moved last step and isn't sweeping or running
		// starts a run now, from a roll of 0-32767: a hop at one in 1000,
		// else a dash at one in 700. Never for a lone pet.
		inline bool hops(uint8_t role, bool sweeping, bool running, bool moved, int32_t roll)
		{
			return role != 0 && !sweeping && !running && moved && roll % 1000 == 0;
		}

		inline bool dashes(uint8_t role, bool sweeping, bool running, bool moved, int32_t roll)
		{
			return role != 0 && !sweeping && !running && moved && roll % 700 == 0;
		}

		// Whether a run that has lasted `ran` ms is over.
		inline bool run_over(int32_t ran)
		{
			return ran > BURST;
		}

		// Whether a pet on the ground jumps toward its owner, who stands on
		// another group of platforms above it, reaching from `left` to
		// `right` across over the pet.
		inline bool jumps_up(uint16_t petgroup, uint16_t ownergroup, int32_t ownery, int32_t pety, int32_t left,
							 int32_t right, int32_t petx)
		{
			return petgroup != ownergroup && ownery < pety && left < petx && petx < right;
		}

		// Which way a pet jumps up: toward its owner, or the way they face
		// when right above it.
		inline int8_t jump_direction(int32_t ownerx, int32_t petx, bool owner_left)
		{
			if (ownerx < petx)
				return -1;

			if (ownerx > petx)
				return 1;

			return owner_left ? -1 : 1;
		}

		// Counts `elapsed` ms toward the next 30 ms step; true when one is due.
		inline bool step(int32_t &elapsed, int32_t elapsed_now)
		{
			elapsed += elapsed_now;

			if (elapsed < TICK)
				return false;

			elapsed -= TICK;
			return true;
		}
	}
}
