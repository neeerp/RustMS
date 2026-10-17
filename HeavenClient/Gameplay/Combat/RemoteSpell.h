#pragma once

#include "ShotPath.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>

namespace ms
{
	// How another user's spell is shown, as in the MapleRoyals client
	// (FUN_0098250d): a ball to each mob it hits, from in front of the user
	// on that mob's side, and the hits as the ball gets there.
	namespace remote_spell
	{
		// How far in front of the user the balls start.
		constexpr int16_t REACH = 50;
		// How far inside the near side of a mob a ball ends.
		constexpr int16_t INSET = 10;
		// How far a ball flies at most.
		constexpr double MAX_FLIGHT = 600.0;
		// How far the ball of a spell that hit nothing flies.
		constexpr int16_t MISS_RANGE = 200;
		// How long after the last mob the next one's hit lands, for the
		// spells that hit one mob after another.
		constexpr uint16_t STAGGER = 50;

		// Whether the spell shows nothing more from the first mob that isn't
		// past where its ball starts: all but the spells that hit the area
		// around the user, and Chain Lightning.
		inline bool needs_ahead(int32_t skill)
		{
			switch (skill)
			{
			case 2201005: // Thunder Bolt
			case 2301002: // Heal
			case 2111002: // Explosion
			case 2311004: // Shining Ray
			case 2211002: // Ice Strike
			case 2121001: // Big Bang
			case 2221001:
			case 2321001:
			case 2121007: // Meteor Shower
			case 2221007: // Blizzard
			case 2321008: // Genesis
			case 2221006: // Chain Lightning
			case 12111003: // Meteor Shower
			case 12101006: // Fire Pillar
			case 22121000: // Evan's
			case 22151001:
			case 22171003:
				return false;
			default:
				return true;
			}
		}

		// Whether the spell flies one ball, to the furthest mob it hits,
		// rather than one to each: Fire Strike and the Demons.
		inline bool one_ball(int32_t skill)
		{
			return skill == 12111006 || skill == 2121003 || skill == 2221003;
		}

		// Whether the spell draws a bolt instead of balls.
		inline bool bolts(int32_t skill)
		{
			return skill == 2221006 || skill == 22171003;
		}

		// Whether its hits land one mob after another, rather than as each
		// ball gets there.
		inline bool staggers(int32_t skill)
		{
			return skill == 2201005 || skill == 2301002;
		}

		// How long after the spell starts its hits land: Heal's at once.
		inline uint16_t attack_delay(int32_t skill, uint16_t action_delay)
		{
			return skill == 2301002 ? 0 : action_delay;
		}

		// How many of a mob's hits show: Heal's first.
		inline size_t shown_hits(int32_t skill, size_t hits)
		{
			return skill == 2301002 ? std::min<size_t>(hits, 1) : hits;
		}

		// Where the ball toward a mob on the left of the user (`left`), or
		// on its right, starts.
		inline Point<int16_t> start(Point<int16_t> user, bool left)
		{
			return {
				static_cast<int16_t>(user.x() + (left ? -REACH : REACH)),
				static_cast<int16_t>(user.y() - shot_path::HEIGHT)
			};
		}

		// Whether a mob with these bounds is all past `start_x`, on the side
		// the ball flies to.
		inline bool ahead(int16_t start_x, Rectangle<int16_t> body, bool left)
		{
			if (body.left() >= body.right() || body.top() >= body.bottom())
				return false;

			return left ? body.right() < start_x : start_x < body.left();
		}

		// Where on a mob the ball aims: 10 px inside its near side, or its
		// middle if it is narrower than that, at the height it starts from
		// if the mob reaches it.
		inline Point<int16_t> aim(Point<int16_t> start, Rectangle<int16_t> body, bool left)
		{
			int32_t middle = (body.left() + body.right()) / 2;
			int32_t x = left ? std::max<int32_t>(middle, body.right() - INSET)
				: std::min<int32_t>(middle, body.left() + INSET);
			int32_t y = start.y();

			if (y < body.top())
				y = body.top();
			else if (body.bottom() < y)
				y = body.bottom();

			return { static_cast<int16_t>(x), static_cast<int16_t>(y) };
		}

		inline double distance(Point<int16_t> from, Point<int16_t> to)
		{
			double dx = to.x() - from.x();
			double dy = to.y() - from.y();

			return std::sqrt(dx * dx + dy * dy);
		}

		// Where the ball from `start` to `aim` stops: there, or 600 px along
		// the way.
		inline Point<int16_t> end(Point<int16_t> start, Point<int16_t> aim)
		{
			double length = distance(start, aim);

			if (length <= MAX_FLIGHT)
				return aim;

			double scale = MAX_FLIGHT / length;

			return {
				static_cast<int16_t>((aim.x() - start.x()) * scale + start.x()),
				static_cast<int16_t>((aim.y() - start.y()) * scale + start.y())
			};
		}

		// How long after the spell starts the `target`th mob's hits land,
		// and the ball to it flies, if it has one: 1.5 ms per pixel all the
		// way to where it aims, however far the ball goes.
		inline uint16_t hit_delay(int32_t skill, size_t target, bool ball, Point<int16_t> start, Point<int16_t> aim)
		{
			if (staggers(skill))
				return static_cast<uint16_t>(std::min<size_t>(target * STAGGER, UINT16_MAX));

			if (!ball)
				return 0;

			return static_cast<uint16_t>(shot_path::MS_PER_PIXEL * distance(start, aim));
		}

		// How long a ball flies straight across from `start_x` to `end_x`.
		inline uint16_t across_time(int16_t start_x, int16_t end_x)
		{
			return static_cast<uint16_t>(shot_path::MS_PER_PIXEL * std::abs(end_x - start_x));
		}
	}
}
