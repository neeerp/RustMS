#pragma once

#include "../../Template/Rectangle.h"

#include <cmath>
#include <cstdint>
#include <cstdlib>

namespace ms
{
	// Where a shot starts, what it hits and when, as in the MapleRoyals
	// client (FUN_009537d5).
	namespace shot_path
	{
		// How far above the user's feet a shot flies.
		constexpr int16_t HEIGHT = 28;

		// How long a bullet or the hit takes per pixel, in ms.
		constexpr double MS_PER_PIXEL = 1.5;

		// Where a shot leaves the user. A regular attack's reach is the
		// point-blank whack range's; a skill's is 0.
		inline Point<int16_t> origin(Point<int16_t> user, int16_t reach, bool toleft)
		{
			int16_t x = user.x() + (toleft ? -reach : reach);

			return { x, static_cast<int16_t>(user.y() - HEIGHT) };
		}

		// Where a shot from `origin` to `end_x` hits a mob with these bounds:
		// the centre of where the shot's 1 px tall row crosses them, moved
		// onto the bounds if they miss each other (FUN_00664603).
		inline Point<int16_t> hit_point(Point<int16_t> origin, int16_t end_x, Rectangle<int16_t> body)
		{
			int32_t left = std::min(origin.x(), end_x);
			int32_t right = std::max(origin.x(), end_x);
			int32_t top = origin.y();
			int32_t bottom = origin.y() + 1;

			if (left >= body.right())
				left = body.right() - 1;
			else if (right <= body.left())
				right = body.left() + 1;

			if (top >= body.bottom())
				top = body.bottom() - 1;
			else if (bottom <= body.top())
				bottom = body.top() + 1;

			left = std::max<int32_t>(left, body.left());
			right = std::min<int32_t>(right, body.right());
			top = std::max<int32_t>(top, body.top());
			bottom = std::min<int32_t>(bottom, body.bottom());

			return { static_cast<int16_t>((left + right) / 2), static_cast<int16_t>((top + bottom) / 2) };
		}

		// How long a bullet flies from `from` to `to`.
		inline uint16_t flight_time(Point<int16_t> from, Point<int16_t> to)
		{
			double dx = to.x() - from.x();
			double dy = to.y() - from.y();

			return static_cast<uint16_t>(MS_PER_PIXEL * std::sqrt(dx * dx + dy * dy));
		}

		// How long after the attack's hit frame the damage at `hit` lands.
		// It only counts the horizontal distance.
		inline uint16_t hit_delay(Point<int16_t> origin, Point<int16_t> hit)
		{
			return static_cast<uint16_t>(MS_PER_PIXEL * std::abs(hit.x() - origin.x()));
		}

		// How far below its target the `bullet`th of `count` bullets flies, so
		// that they spread 14 px apart around it.
		inline int16_t spread(uint8_t bullet, uint8_t count)
		{
			return static_cast<int16_t>(7 + 14 * bullet - 7 * count);
		}

		// How long after the first bullet the `bullet`th one is thrown or
		// fired (FUN_00942831).
		inline uint16_t bullet_delay(int32_t bullet_id, int32_t skill, uint8_t bullet)
		{
			uint16_t interval;

			if (bullet_id / 10000 == 207 || bullet_id / 1000 == 5021)
				interval = 120; // Throwing stars
			else
				switch (skill)
				{
				case 4111004: // Shadow Meso
				case 5221007: // Battleship Cannon
					interval = 120;
					break;
				case 3111006: // Strafe
				case 3211006:
				case 13111001:
					interval = 60;
					break;
				case 5001003: // Double Shot
				case 5210000: // Burst Fire
					interval = 240;
					break;
				default:
					interval = 0;
					break;
				}

			return static_cast<uint16_t>(interval * bullet);
		}
	}
}
