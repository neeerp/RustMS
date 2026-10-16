#pragma once

#include "../../Character/Inventory/Weapon.h"
#include "../../Template/Rectangle.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <optional>
#include <vector>

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

		// How far a shot's search for its target widens per pixel from the user.
		constexpr int16_t SPREAD_DIVISOR = 4;

		// How wide each step of that search is.
		constexpr int16_t STEP = 20;

		// How far in front of the user an aimed spell starts, and how far it
		// reaches.
		constexpr int16_t SPELL_REACH = 50;
		constexpr int16_t SPELL_RANGE = 300;

		// Whether a spell is aimed at the first mob in its path like a shot
		// (FUN_0095571f), rather than at all the mobs in its area. Chain
		// Lightning picks its first mob the same way, but its chain is not
		// copied yet.
		inline bool aimed_spell(int32_t skill)
		{
			switch (skill)
			{
			case 2111002: // Explosion
			case 2121001: // Big Bang
			case 2121003: // Fire Demon
			case 2121007: // Meteor Shower
			case 2201005: // Thunder Bolt
			case 2211002: // Ice Strike
			case 2221001: // Big Bang
			case 2221003: // Ice Demon
			case 2221006: // Chain Lightning
			case 2221007: // Blizzard
			case 2301002: // Heal
			case 2311004: // Shining Ray
			case 2321001: // Big Bang
			case 2321008: // Genesis
			case 4121004: // Ninja Ambush
			case 4221004:
			case 12101006: // Fire Pillar
			case 12111003: // Meteor Shower
			case 12111006: // Fire Strike
				return false;
			default:
				return true;
			}
		}

		// The skill that lets this job's shots with this weapon reach further:
		// Eye of Amazon for bows and crossbows, Keen Eyes for claws, or their
		// Cygnus versions.
		inline int32_t range_skill(Weapon::Type weapon, uint16_t job)
		{
			bool cygnus = job / 1000 == 1;

			switch (weapon)
			{
			case Weapon::Type::BOW:
			case Weapon::Type::CROSSBOW:
				return cygnus ? 13000001 : 3000002;
			case Weapon::Type::CLAW:
				return cygnus ? 14000001 : 4000001;
			default:
				return 0;
			}
		}

		// How far in front of the user a shot reaches (FUN_00761845): the
		// skill's own range if it has one, else the weapon's plus that of its
		// range skill.
		inline int16_t range(int32_t skill, int16_t skill_range, Weapon::Type weapon, int16_t skill_bonus)
		{
			if (skill_range > 0)
				return skill_range;

			if (skill == 4121003 || skill == 4221003)
				return 300;

			switch (weapon)
			{
			case Weapon::Type::BOW:
			case Weapon::Type::CROSSBOW:
				return static_cast<int16_t>(300 + skill_bonus);
			case Weapon::Type::CLAW:
				return static_cast<int16_t>(200 + skill_bonus);
			case Weapon::Type::GUN:
				return 200;
			default:
				return 0;
			}
		}

		// Where a shot from `origin` toward `end_x` looks for its target, step
		// by step from the nearest. Each step is 20 px wide and reaches 1 px
		// above and below the shot per 4 px it starts from the user
		// (FUN_006789ed), so the first steps of a skill's shot are empty.
		inline std::vector<Rectangle<int16_t>> search_steps(int16_t user_x, Point<int16_t> origin, int16_t end_x)
		{
			std::vector<Rectangle<int16_t>> steps;
			bool forward = origin.x() <= end_x;

			for (int32_t x = origin.x(); forward ? x < end_x : end_x < x; x += forward ? STEP : -STEP)
			{
				int32_t left = forward ? x : std::max<int32_t>(x - STEP, end_x);
				int32_t right = forward ? std::min<int32_t>(x + STEP, end_x) : x;
				int32_t spread = (forward ? x - user_x : user_x - x) / SPREAD_DIVISOR;

				steps.emplace_back(
					static_cast<int16_t>(left),
					static_cast<int16_t>(right),
					static_cast<int16_t>(origin.y() - spread),
					static_cast<int16_t>(origin.y() + spread)
				);
			}

			return steps;
		}

		// Whether a mob with these bounds is in a search step, as
		// IntersectRect sees it: nothing is in an empty step.
		inline bool in_step(Rectangle<int16_t> step, Rectangle<int16_t> body)
		{
			return std::max(step.left(), body.left()) < std::min(step.right(), body.right())
				&& std::max(step.top(), body.top()) < std::min(step.bottom(), body.bottom());
		}

		// A platform's line, from where it starts to where it ends.
		struct Line
		{
			int32_t x1, y1, x2, y2;
		};

		// A platform as a shot sees it, with those joined to either end.
		struct Platform
		{
			Line line;
			uint16_t group;
			std::optional<Line> prev;
			std::optional<Line> next;
		};

		// Which side of a line a point is on: negative left of it, which for
		// a platform is the open side above a floor, positive right of it.
		inline int32_t side(Line line, int32_t x, int32_t y)
		{
			return (line.x2 - line.x1) * (y - line.y1) - (line.y2 - line.y1) * (x - line.x1);
		}

		// Whether a point is behind the corner where `first` runs into
		// `second` (FUN_009b3f1b): behind either of them where they turn
		// toward their open side, like a step up, and behind both where they
		// turn away from it, like a ledge.
		inline bool behind_corner(Line first, Line second, int32_t x, int32_t y)
		{
			bool behind_first = side(first, x, y) > 0;
			bool behind_second = side(second, x, y) > 0;

			if (side(first, second.x2, second.y2) <= 0)
				return behind_first || behind_second;
			else
				return behind_first && behind_second;
		}

		// Whether a shot from `from` to `to` runs into a platform first
		// (FUN_00a457d0). It only stops going into one from its open side, at
		// a floor or at a wall or ceiling with no group or the `group` the
		// user last stood on. If it does, `to` moves to where it stops.
		inline bool blocked(Point<int16_t> from, Point<int16_t> &to, const std::vector<Platform> &platforms, uint16_t group)
		{
			Line shot = { from.x(), from.y(), to.x(), to.y() };

			if (shot.x1 == shot.x2 && shot.y1 == shot.y2)
				return false;

			// How far along the shot the nearest platform is, as a fraction.
			int64_t near = 2;
			int64_t far = 1;
			bool found = false;

			for (const Platform &platform : platforms)
			{
				Line line = platform.line;

				if (line.x2 <= line.x1 && platform.group != 0 && platform.group != group)
					continue;

				int32_t start = side(line, shot.x1, shot.y1);
				int32_t end = side(line, shot.x2, shot.y2);

				if (start > 0 || end < 0 || (start == 0 && end == 0))
					continue;

				int32_t first = side(shot, line.x1, line.y1);
				int32_t second = side(shot, line.x2, line.y2);

				if ((first < 0 && second < 0) || (first > 0 && second > 0))
					continue;

				// Through an end, it has to go behind the platform there too.
				if (first == 0)
				{
					if (!platform.prev || !behind_corner(*platform.prev, line, shot.x2, shot.y2))
						continue;
				} else if (second == 0)
				{
					if (!platform.next || !behind_corner(line, *platform.next, shot.x2, shot.y2))
						continue;
				}

				int64_t across = end - start;
				int64_t along = -start;

				if (across * near > along * far)
				{
					near = along;
					far = across;
					found = true;
				}
			}

			if (!found)
				return false;

			double t = static_cast<double>(near) / far;
			to = {
				static_cast<int16_t>((shot.x2 - shot.x1) * t + shot.x1),
				static_cast<int16_t>((shot.y2 - shot.y1) * t + shot.y1)
			};

			return true;
		}

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
