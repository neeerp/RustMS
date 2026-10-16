#pragma once

#include "ShotPath.h"

#include <algorithm>
#include <climits>
#include <cstdint>
#include <functional>
#include <optional>
#include <vector>

namespace ms
{
	// Where the Teleport skills take the user, as in the MapleRoyals client
	// (FUN_00957b74).
	namespace teleport
	{
		// How far above the user's feet the area checked for mobs in the way
		// reaches, and how far either side of the user it reaches on a move
		// up or down.
		constexpr int32_t MOB_HEIGHT = 65;
		constexpr int32_t MOB_REACH = 20;

		// How many mobs in the way it looks at.
		constexpr size_t MOB_LOOK = 15;

		// How far up or down a move across looks for a floor to land on.
		constexpr int32_t FLOOR_REACH = 80;

		inline bool teleports(int32_t skill)
		{
			switch (skill)
			{
			case 2101002: // Teleport (Fire/Poison)
			case 2201002: // Teleport (Ice/Lightning)
			case 2301001: // Teleport (Cleric)
			case 8001001:
			case 9001002: // GM Teleport
			case 12101003: // Teleport (Blaze Wizard)
			case 22101001: // Teleport (Evan)
				return true;
			default:
				return false;
			}
		}

		// A platform the user might land on.
		struct Floor
		{
			uint16_t id;
			shot_path::Line line;
		};

		// Where the user would land on a floor.
		struct Landing
		{
			uint16_t id;
			int32_t y;
		};

		// The floor's height across at x, if it runs left to right over x.
		inline std::optional<int32_t> height(shot_path::Line line, int32_t x)
		{
			if (line.x1 >= line.x2 || x < line.x1 || x > line.x2)
				return std::nullopt;

			return (line.y2 - line.y1) * (x - line.x1) / (line.x2 - line.x1) + line.y1;
		}

		// The highest floor at x from y down to just above `bottom`
		// (FUN_00a45585). Walls and ceilings don't count.
		inline std::optional<Landing> floor_below(const std::vector<Floor> &floors, int32_t x, int32_t y, int32_t bottom)
		{
			if (bottom < y)
				return std::nullopt;

			std::optional<Landing> found;
			int32_t best = bottom;

			for (const Floor &floor : floors)
			{
				std::optional<int32_t> fy = height(floor.line, x);

				if (fy && y <= *fy && *fy < best)
				{
					best = *fy;
					found = Landing{ floor.id, *fy };
				}
			}

			return found;
		}

		// The lowest floor at x from y up to just below `top` (FUN_00a4549d).
		inline std::optional<Landing> floor_above(const std::vector<Floor> &floors, int32_t x, int32_t y, int32_t top)
		{
			if (y < top)
				return std::nullopt;

			std::optional<Landing> found;
			int32_t best = top;

			for (const Floor &floor : floors)
			{
				std::optional<int32_t> fy = height(floor.line, x);

				if (fy && *fy <= y && best < *fy)
				{
					best = *fy;
					found = Landing{ floor.id, *fy };
				}
			}

			return found;
		}

		// Where a mob that can't be passed by Teleport stops the move. Across,
		// it is the way between the user and the end of the move, from 65 px
		// above the user down to the end's height; up or down, 20 px either
		// side of the user, from 65 px above the higher end to the lower. With
		// no way pressed, there's no move.
		inline std::optional<Rectangle<int16_t>> mob_area(Point<int16_t> user, int32_t dx, int32_t dy, int32_t range)
		{
			int32_t x = user.x();
			int32_t y = user.y();
			int32_t tx = x + range * dx;
			int32_t ty = y + range * dy;

			if (dx != 0)
				return Rectangle<int16_t>(
					static_cast<int16_t>(std::min(x, tx)),
					static_cast<int16_t>(std::max(x, tx)),
					static_cast<int16_t>(y - MOB_HEIGHT),
					static_cast<int16_t>(ty)
				);

			if (dy != 0)
				return Rectangle<int16_t>(
					static_cast<int16_t>(x - MOB_REACH),
					static_cast<int16_t>(x + MOB_REACH),
					static_cast<int16_t>(std::min(y, ty) - MOB_HEIGHT),
					static_cast<int16_t>(std::max(y, ty))
				);

			return std::nullopt;
		}

		// A mob that might be in the way.
		struct Mob
		{
			Rectangle<int16_t> body;
			// When it entered the map: earlier ones are looked at first.
			uint32_t entry;
			// Whether it has cantPassByTeleport.
			bool blocks;
		};

		// Whether a mob stops the move: one of the first 15 to enter the map
		// whose bodies are in the area can't be passed (FUN_00678476).
		inline bool mob_in_way(Rectangle<int16_t> area, std::vector<Mob> mobs)
		{
			std::stable_sort(mobs.begin(), mobs.end(), [](const Mob &a, const Mob &b) { return a.entry < b.entry; });

			size_t looked = 0;

			for (const Mob &mob : mobs)
			{
				if (looked == MOB_LOOK)
					break;

				if (!shot_path::in_step(area, mob.body))
					continue;

				looked++;

				if (mob.blocks)
					return true;
			}

			return false;
		}

		// Where the user lands: 1 px above a floor, and the floor.
		struct Spot
		{
			Point<int16_t> position;
			uint16_t foothold;
		};

		// Where the user lands, if anywhere: `range` px the way pressed, 1 px
		// above a floor. `dx` and `dy` are -1, 0 or 1; a move across ignores
		// `dy`. `clear` says whether nothing is in the way of a shot from one
		// point to another (FUN_00a457d0).
		//
		// Across, it lands on the nearer of the floors up to 80 px below and
		// above, the one below on a tie. Up, it lands on the floor at or below
		// that height, but only if a platform lies between there and the
		// user; otherwise it stays where it is. Down, it lands on the floor at
		// or above that height unless that is the one the user stands on, and
		// then on the floor up to `range` px below it.
		inline std::optional<Spot> destination(Point<int16_t> user, uint16_t foothold, int32_t dx, int32_t dy,
											   int32_t range, const std::vector<Floor> &floors,
											   const std::function<bool(Point<int16_t>, Point<int16_t>)> &clear)
		{
			int32_t x = user.x();
			int32_t y = user.y();
			int32_t tx = x + range * dx;
			int32_t ty = y + range * dy;
			std::optional<Landing> landing;

			if (dx != 0)
			{
				std::optional<Landing> below = floor_below(floors, tx, y, y + FLOOR_REACH);
				std::optional<Landing> above = floor_above(floors, tx, y, y - FLOOR_REACH);

				landing = below;

				if (!below || (above && below->y - y > y - above->y))
					landing = above;

				x = tx;
			} else if (dy < 0)
			{
				landing = floor_below(floors, x, ty, INT_MAX);

				if (landing && landing->y != y)
				{
					Point<int16_t> from(static_cast<int16_t>(x), static_cast<int16_t>(landing->y - 1));
					Point<int16_t> to(static_cast<int16_t>(x), static_cast<int16_t>(y - 1));

					if (clear(from, to))
						landing->y = y;
				}

				// Staying where it is, the user stays on their own floor.
				if (landing && landing->y == y)
					landing->id = foothold;
			} else if (dy > 0)
			{
				landing = floor_above(floors, x, ty, INT_MIN);

				if (!landing || landing->id == foothold)
					landing = floor_below(floors, x, ty, ty + range);
			}

			if (!landing)
				return std::nullopt;

			return Spot{ { static_cast<int16_t>(x), static_cast<int16_t>(landing->y - 1) }, landing->id };
		}
	}
}
