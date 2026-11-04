#pragma once

#include "AttackDash.h"

#include <cstdint>
#include <cstdlib>

namespace ms
{
	// The local player's attacks that throw the mobs they flinch to a spot, not 130 px/s away
	// from the attacker: its offset is set per mob at the hit (0x9520eb to 0x952524), its first
	// speed when the mob flinches (0x66b6fc), and each 30 ms step after that (0x9bbdfd).
	namespace mob_push
	{
		// How often a throw changes its speed, in ms.
		constexpr int16_t STEP = 30;
		// Within this of the spot, a mob on the ground lands on it in one step, and a flier
		// slows to a third every step.
		constexpr int16_t NEAR = 100;
		// Within this of the spot a mob stops.
		constexpr int16_t STOP = 10;
		// How far a mob's spot may fall past the attack's reach, on top of `MARGIN`.
		constexpr int32_t SPREAD = 20;
		constexpr int32_t MARGIN = 35;
		constexpr int32_t BACKSPIN_MARGIN = 15;
		constexpr double FAST = 700.0;
		constexpr double SLOW = 300.0;

		enum class Kind : uint8_t
		{
			NONE,
			// Ahead of the attacker, slowly if the mob starts near the spot (type 5).
			AHEAD,
			// Behind the attacker, likewise (type 7).
			BEHIND,
			// Ahead of the attacker, always fast (type 8).
			DRIVEN
		};

		inline Kind kind(int32_t skill)
		{
			switch (skill)
			{
				case attack_dash::RUSH_HERO:
				case attack_dash::RUSH_PALADIN:
				case attack_dash::RUSH_DK:
				case attack_dash::FINAL_CHARGE:
					return Kind::AHEAD;
				case attack_dash::BACKSPIN_BLOW:
					return Kind::BEHIND;
				case attack_dash::CORKSCREW_BLOW:
				case attack_dash::CORKSCREW_BLOW_TB:
					return Kind::DRIVEN;
				default:
					return Kind::NONE;
			}
		}

		// How far from the attacker `skill` throws a mob as wide as `width`: past the edge of
		// its lt/rb (`lt` and `rb` are their x), with `roll` from 0 to SPREAD - 1.
		inline int32_t offset(int32_t skill, int32_t lt, int32_t rb, int32_t width, int32_t roll, bool left)
		{
			int32_t reach;

			if (skill == attack_dash::FINAL_CHARGE)
				reach = width - lt;
			else if (kind(skill) == Kind::BEHIND)
				reach = -BACKSPIN_MARGIN - roll - rb - width;
			else
				reach = roll - lt + width + MARGIN;

			return left ? -reach : reach;
		}

		// How fast, in px/s, a mob at `x` starts toward `target`. It goes the way the attacker
		// faces, or the other way for Backspin Blow, wherever the spot is.
		inline double start_speed(Kind kind, int32_t x, int32_t target, bool left)
		{
			double speed = kind == Kind::DRIVEN || std::abs(x - target) >= NEAR ? FAST : SLOW;

			if (kind == Kind::BEHIND)
				speed = -speed;

			return left ? -speed : speed;
		}

		// The attack that throws a mob: its skill, the x of its lt and rb, and where its user
		// stood.
		struct Hit
		{
			int32_t skill;
			int16_t lt;
			int16_t rb;
			int16_t user_x;
		};

		// A mob's throw: where to, where it started from relative to that, how fast it goes in
		// px/s, and how far into a step it is.
		struct Throw
		{
			int32_t target;
			int32_t start;
			double speed;
			int16_t elapsed = STEP;
			// How far this step takes the mob, in px.
			double distance = 0.0;

			// Start the next step for a mob at `x`, on the ground or in the air.
			void step(int32_t x, bool flying)
			{
				int32_t left = x - target;

				if (flying)
				{
					if (std::abs(left) < NEAR)
						speed /= 3;

					distance = speed * STEP / 1000.0;
				} else if (std::abs(left) < STOP || static_cast<int64_t>(left) * start <= 0)
				{
					speed = 0.0;
					distance = 0.0;
				} else if (std::abs(left) < NEAR)
				{
					speed /= 3;
					distance = speed < 0.0 ? -std::abs(left) : std::abs(left);
				} else
				{
					distance = speed * STEP / 1000.0;
				}
			}
		};
	}
}
