#pragma once

#include "RemoteSpell.h"
#include "ShotPath.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>

namespace ms
{
	// How another user's shot is shown, as in the MapleRoyals client
	// (FUN_00981211): from in front of the user on each mob's side, its hits
	// landing 1.5 ms per pixel to where it aims at the mob, and its bullets
	// flying there for as long, mostly only to the first mob.
	namespace remote_shot
	{
		// How far in front of the user a shot starts.
		constexpr int16_t REACH = 50;
		// How far a bullet flies at most.
		constexpr double MAX_FLIGHT = 400.0;
		// How far the bullets of a shot that hit nothing fly, or of a bow's
		// or crossbow's.
		constexpr int16_t MISS_RANGE = 200;
		constexpr int16_t MISS_RANGE_BOW = 300;

		// How long after its action starts a shot hits (FUN_009427c4).
		// Piercing Arrow's is its charge, which other users' don't send.
		inline uint16_t attack_delay(int32_t skill, uint16_t action_delay)
		{
			switch (skill)
			{
			case 5221008: // Battleship Torpedo
				return 600;
			case 5001003: // Double Shot
			case 5210000: // Burst Fire
				return 90;
			case 5221007: // Battleship Cannon
				return 180;
			case 13111006: // Wind Piercing
				return 618;
			case 13111007: // Wind Shot
				return 561;
			case 15111007: // Shark Wave
				return 449;
			default:
				return action_delay;
			}
		}

		// Where a shot at a mob on the left of the user (`left`), or on its
		// right, starts. Shark Wave starts further ahead, and Soul Blade
		// ahead of the way the user faces whichever side the mob is on.
		inline Point<int16_t> start(int32_t skill, Point<int16_t> user, bool left, bool toleft)
		{
			int16_t reach = REACH;

			if (skill == 11101004)
			{
				reach = 70;
				left = toleft;
			} else if (skill == 15111007)
			{
				reach = 150;
			}

			return {
				static_cast<int16_t>(user.x() + (left ? -reach : reach)),
				static_cast<int16_t>(user.y() - shot_path::HEIGHT)
			};
		}

		// Where the bullets from `start` to `aim` stop: there, or 400 px
		// along the way.
		inline Point<int16_t> end(Point<int16_t> start, Point<int16_t> aim)
		{
			double length = remote_spell::distance(start, aim);

			if (length <= MAX_FLIGHT)
				return aim;

			double scale = MAX_FLIGHT / length;

			return {
				static_cast<int16_t>((aim.x() - start.x()) * scale + start.x()),
				static_cast<int16_t>((aim.y() - start.y()) * scale + start.y())
			};
		}

		// How long after the shot hits the `target`th mob's hits land, the
		// first's after `first`: 1.5 ms per pixel all the way to where it
		// aims, however far its bullets go. Arrow Rain's later mobs are hit
		// a whole `delay` and 400 ms after, so twice its delay after it is
		// fired, where the user's own land once.
		inline uint16_t hit_delay(int32_t skill, size_t target, uint16_t first, uint16_t delay, Point<int16_t> start,
								  Point<int16_t> aim)
		{
			if (target > 0)
				switch (skill)
				{
				case 3101005: // Arrow Bomb
					return first + 200;
				case 3111003: // Inferno
				case 3211003: // Blizzard
					return static_cast<uint16_t>(first + 30 * (target + 4));
				case 3111004: // Arrow Rain
				case 3211004: // Arrow Eruption
				case 13111000: // Arrow Rain
					return delay + 400;
				}

			return static_cast<uint16_t>(shot_path::MS_PER_PIXEL * remote_spell::distance(start, aim));
		}

		// How many of a mob's hits show: as many as its bullets carry, each
		// the skill's attack count.
		inline size_t shown_hits(size_t hits, uint8_t bulletcount, uint8_t attackcount)
		{
			size_t carried = std::max<size_t>(bulletcount, 1) * std::max<size_t>(attackcount, 1);

			return std::min(hits, carried);
		}

		// Whether the shot draws no bullets at all.
		inline bool hides_bullet(int32_t skill)
		{
			switch (skill)
			{
			case 15111007: // Shark Wave
			case 11101004: // Soul Blade
			case 13111006: // Wind Piercing
			case 21100004: // Combo Smash
			case 21110004: // Combo Penril
				return true;
			default:
				return shot_path::hides_bullet(skill);
			}
		}

		// Whether the shot's bullets jump from mob to mob instead: Energy Orb
		// and Spark.
		inline bool hops(int32_t skill)
		{
			return skill == 5121002 || skill == 15111006;
		}

		// Whether the shot flies its bullets to every mob it hits, rather
		// than to the first alone.
		inline bool shoots_all(int32_t skill)
		{
			switch (skill)
			{
			case 3121004: // Hurricane
			case 5221007: // Battleship Cannon
			case 13111002: // Hurricane
			case 13111007: // Wind Shot
			case 5211006: // Homing Beacon
			case 5220011: // Bullseye
			case 22151002: // Killer Wings
			case 5221004: // Rapid Fire
				return true;
			default:
				return false;
			}
		}

		// Whether the shot flies bullets to its `target`th mob, its
		// `lt_rb` empty or not: a skill with no area of its own only does
		// to mobs all past where they start.
		inline bool shoots(int32_t skill, size_t target, Rectangle<int16_t> lt_rb, int16_t start_x,
						   Rectangle<int16_t> body, bool left)
		{
			if (hides_bullet(skill) || hops(skill))
				return false;

			if (target > 0 && !shoots_all(skill))
				return false;

			bool no_area = lt_rb.left() >= lt_rb.right() || lt_rb.top() >= lt_rb.bottom();

			if (skill && no_area)
				return left ? body.right() < start_x : start_x < body.left();

			return true;
		}

		// How far in front of the user the bullets of a shot that hit nothing
		// fly, further from a bow or crossbow.
		inline int16_t miss_range(bool bow)
		{
			return bow ? MISS_RANGE_BOW : MISS_RANGE;
		}

		// Whether a shot that hit nothing draws no bullets: fewer skills
		// than hide them otherwise.
		inline bool hides_miss(int32_t skill)
		{
			return shot_path::hides_bullet(skill) || hops(skill);
		}

		// Where the bullets of a shot that hit nothing start, in front of
		// the user, Shark Wave's further on.
		inline int16_t miss_start(int32_t skill, int16_t user_x, bool toleft)
		{
			int16_t reach = skill == 15111007 ? 150 : REACH;

			return static_cast<int16_t>(user_x + (toleft ? -reach : reach));
		}

		// How long they fly from `start_x` to `end_x`: 1.5 ms per pixel, 1
		// for the Aran's combo skills.
		inline uint16_t miss_flight(int32_t skill, int16_t start_x, int16_t end_x)
		{
			double length = std::abs(end_x - start_x);

			if (skill == 21100004 || skill == 21110004)
				return static_cast<uint16_t>(length);

			return static_cast<uint16_t>(shot_path::MS_PER_PIXEL * length);
		}

		// How far below the user's shot height the `bullet`th of `count`
		// bullets of a shot that hit nothing flies, so that they spread 8 px
		// apart.
		inline int16_t miss_spread(uint8_t bullet, uint8_t count)
		{
			return static_cast<int16_t>(8 * bullet - 4 * count + 4);
		}
	}
}
