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
		// Lightning picks its first mob the same way.
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

		// How far above and below its row a shot that hits every mob along it
		// reaches (FUN_0076664d), or 0.
		inline int16_t area_height(int32_t skill)
		{
			switch (skill)
			{
			case 3201005: // Iron Arrow
			case 4121003: // Taunt
			case 4221003:
			case 13111006: // Wind Piercing
				return 10;
			case 3121003: // Dragon's Breath
			case 3221001: // Piercing Arrow
			case 3221003: // Dragon's Breath
				return 20;
			case 4111005: // Avenger
			case 14111002:
				return 36;
			case 11101004: // Soul Blade
			case 15111007: // Shark Wave
			case 21100004: // Combo Smash
			case 21110004: // Combo Penril
				return 60;
			case 15111006: // Spark
				return 150;
			default:
				return 0;
			}
		}

		// Whether a shot hits every mob in the skill's own area around the
		// user, its lt and rb, rather than along its row (FUN_007666cb).
		inline bool skill_area(int32_t skill)
		{
			switch (skill)
			{
			case 3111004: // Arrow Rain
			case 3211004: // Arrow Eruption
			case 5201001: // Invisible Shot
			case 5211004: // Flamethrower
			case 5211005: // Ice Splitter
			case 5221008: // Battleship Torpedo
			case 13101005: // Storm Break
			case 13111000: // Arrow Rain
			case 14101006: // Vampire
			case 21120006: // Combo Tempest
				return true;
			default:
				return false;
			}
		}

		// Whether a shot hits the first mobs in its area, as many as it may
		// and in the order they entered the map, rather than the first mob in
		// its path (FUN_00766722).
		inline bool area_shot(int32_t skill)
		{
			return skill != 15111006 && (area_height(skill) > 0 || skill_area(skill));
		}

		// Whether a shot draws no bullet (FUN_00766867).
		inline bool hides_bullet(int32_t skill)
		{
			return skill_area(skill) && skill != 5221008;
		}

		// Whether a shot jumps on from the first mob it hits, as Chain
		// Lightning does: Energy Orb.
		inline bool chains(int32_t skill)
		{
			return skill == 5121002;
		}

		// Whether a shot also hits the mobs in the skill's area around the
		// first mob it hits: Arrow Bomb, Inferno and Blizzard.
		inline bool splashes(int32_t skill)
		{
			return skill == 3101005 || skill == 3111003 || skill == 3211003;
		}

		// Whether a shot hits only its first mob after all, because it missed
		// it (`first_damage` 0): Arrow Bomb does (FUN_0075bf50).
		inline bool drops_splash(int32_t skill, int32_t first_damage)
		{
			return skill == 3101005 && first_damage == 0;
		}

		// How much of the weapon's damage a skill's hits deal: its `damage`
		// percent, all of it with none, and Arrow Bomb's `x` percent of that
		// (FUN_0078df87, FUN_0075bf50).
		inline float damage_scale(int32_t skill, int32_t damage, int32_t x)
		{
			float scale = damage > 0 ? damage / 100.0f : 1.0f;

			if (skill == 3101005)
				scale *= x / 100.0f;

			return scale;
		}

		// The area a shot covers: its row from `origin` to `end_x`, widened
		// for some skills, or the skill's own `lt_rb` around the `user`,
		// turned the way they face.
		inline Rectangle<int16_t> area(int32_t skill, Point<int16_t> user, Point<int16_t> origin, int16_t end_x,
									   Rectangle<int16_t> lt_rb, bool toleft)
		{
			if (skill_area(skill))
			{
				if (toleft)
					return {
						static_cast<int16_t>(user.x() + lt_rb.left()),
						static_cast<int16_t>(user.x() + lt_rb.right()),
						static_cast<int16_t>(user.y() + lt_rb.top()),
						static_cast<int16_t>(user.y() + lt_rb.bottom())
					};

				return {
					static_cast<int16_t>(user.x() - lt_rb.right()),
					static_cast<int16_t>(user.x() - lt_rb.left()),
					static_cast<int16_t>(user.y() + lt_rb.top()),
					static_cast<int16_t>(user.y() + lt_rb.bottom())
				};
			}

			int16_t height = area_shot(skill) ? area_height(skill) : 0;

			return {
				std::min(origin.x(), end_x),
				std::max(origin.x(), end_x),
				static_cast<int16_t>(origin.y() - height),
				static_cast<int16_t>(origin.y() + 1 + height)
			};
		}

		// The area around the mob a splashing shot hits at `hit`: the skill's
		// lt and rb, whichever way the user faces.
		inline Rectangle<int16_t> splash_area(Point<int16_t> hit, Rectangle<int16_t> lt_rb)
		{
			return {
				static_cast<int16_t>(hit.x() + lt_rb.left()),
				static_cast<int16_t>(hit.x() + lt_rb.right()),
				static_cast<int16_t>(hit.y() + lt_rb.top()),
				static_cast<int16_t>(hit.y() + lt_rb.bottom())
			};
		}

		// Where the bullets of a shot from `origin` start: a few skills fire
		// from further ahead or behind.
		inline Point<int16_t> bullet_start(int32_t skill, Point<int16_t> origin, bool toleft)
		{
			int16_t ahead;

			switch (skill)
			{
			case 5221007: // Battleship Cannon
			case 5221008: // Battleship Torpedo
				ahead = 30;
				break;
			case 11101004: // Soul Blade
				ahead = 70;
				break;
			case 15111007: // Shark Wave
				ahead = 150;
				break;
			default:
				ahead = 0;
				break;
			}

			return { static_cast<int16_t>(origin.x() + (toleft ? -ahead : ahead)), origin.y() };
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

		// Where a shot covering `area` hits a mob with these bounds: the
		// centre of where they cross, moved onto the bounds if they miss
		// each other (FUN_00664603).
		inline Point<int16_t> hit_point(Rectangle<int16_t> area, Rectangle<int16_t> body)
		{
			int32_t left = area.left();
			int32_t right = area.right();
			int32_t top = area.top();
			int32_t bottom = area.bottom();

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

		// Where a shot from `origin` to `end_x` hits a mob with these bounds,
		// its row 1 px tall.
		inline Point<int16_t> hit_point(Point<int16_t> origin, int16_t end_x, Rectangle<int16_t> body)
		{
			return hit_point(area(0, origin, origin, end_x, {}, false), body);
		}

		// How long a bullet flies from `from` to `to`.
		inline uint16_t flight_time(Point<int16_t> from, Point<int16_t> to)
		{
			double dx = to.x() - from.x();
			double dy = to.y() - from.y();

			return static_cast<uint16_t>(MS_PER_PIXEL * std::sqrt(dx * dx + dy * dy));
		}

		// Whether a shot's ball jumps from mob to mob instead, each hit when
		// it gets there: Energy Orb and Spark (FUN_0094289f).
		inline bool hops(int32_t skill)
		{
			return skill == 5121002 || skill == 15111006;
		}

		// Where the ball's first jump starts: 25 px ahead of `origin`.
		inline Point<int16_t> hop_start(Point<int16_t> origin, bool toleft)
		{
			return { static_cast<int16_t>(origin.x() + (toleft ? -25 : 25)), origin.y() };
		}

		// Where the ball jumps to on a mob with these bounds: their middle.
		inline Point<int16_t> hop_point(Rectangle<int16_t> body)
		{
			return {
				static_cast<int16_t>((body.left() + body.right()) / 2),
				static_cast<int16_t>((body.top() + body.bottom()) / 2)
			};
		}

		// How long the ball's jump from `from` to `to` takes: 1.25 ms per
		// pixel, Spark's 2. Each mob is hit when the ball gets there, a jump
		// after the last.
		inline uint16_t hop_time(int32_t skill, Point<int16_t> from, Point<int16_t> to)
		{
			double dx = to.x() - from.x();
			double dy = to.y() - from.y();
			double per_pixel = skill == 15111006 ? 2.0 : 1.25;

			return static_cast<uint16_t>(per_pixel * std::sqrt(dx * dx + dy * dy));
		}

		// How long after the attack's hit frame the damage at `hit` lands.
		// It only counts the horizontal distance.
		inline uint16_t hit_delay(Point<int16_t> origin, Point<int16_t> hit)
		{
			return static_cast<uint16_t>(MS_PER_PIXEL * std::abs(hit.x() - origin.x()));
		}

		// How long after the attack's hit frame the damage to its `target`th
		// mob lands, at `hit`, the first mob's after `first` (0x954867).
		inline uint16_t target_delay(int32_t skill, size_t target, uint16_t first, Point<int16_t> origin,
									 Point<int16_t> hit)
		{
			switch (skill)
			{
			case 3101005: // Arrow Bomb
				if (target > 0)
					return first + 200;
				break;
			case 3111003: // Inferno
			case 3211003: // Blizzard
				if (target > 0)
					return static_cast<uint16_t>(first + 120 + 30 * target);
				break;
			case 3111004: // Arrow Rain
			case 3211004: // Arrow Eruption
			case 13111000: // Arrow Rain
				return 400;
			}

			return hit_delay(origin, hit);
		}

		// How far below its target the `bullet`th of `count` bullets flies, so
		// that they spread 14 px apart around it.
		inline int16_t spread(uint8_t bullet, uint8_t count)
		{
			return static_cast<int16_t>(7 + 14 * bullet - 7 * count);
		}

		// Whether Shadow Partner throws a shot's bullets again: not for
		// Taunt, and never with Vampire (0x953b2a).
		inline bool partner_shoots(int32_t skill)
		{
			return skill != 4121003 && skill != 14101006;
		}

		// A shot's hits with Shadow Partner: twice as many, at most 15.
		inline uint8_t partner_hits(uint8_t hitcount)
		{
			return static_cast<uint8_t>(std::min(hitcount * 2, 15));
		}

		// Where Shadow Partner's bullets start: 30 px behind the user's.
		inline Point<int16_t> partner_start(Point<int16_t> start, bool toleft)
		{
			return start + Point<int16_t>(toleft ? 30 : -30, 0);
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
