#pragma once

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <optional>
#include <utility>

namespace ms
{
	// Where a monster's attacks reach, as the v83 client works it out
	// (FUN_0066a517 picks an attack, FUN_0066d9c0 lands it). Attack data is
	// drawn for a monster facing left.
	namespace mob_attack
	{
		// A rectangle that holds its left and top edges but not its right
		// and bottom ones, like a Win32 RECT.
		struct Box
		{
			int32_t left;
			int32_t top;
			int32_t right;
			int32_t bottom;

			bool empty() const
			{
				return left >= right || top >= bottom;
			}

			Box shift(int32_t x, int32_t y) const
			{
				return { left + x, top + y, right + x, bottom + y };
			}

			// The same box for a monster facing the other way.
			Box mirror() const
			{
				return { -right, top, -left, bottom };
			}

			int32_t center_x() const
			{
				return (left + right) / 2;
			}

			int32_t center_y() const
			{
				return (top + bottom) / 2;
			}
		};

		// IntersectRect: empty boxes meet nothing.
		inline bool intersects(const Box &a, const Box &b)
		{
			if (a.empty() || b.empty())
				return false;

			return std::max(a.left, b.left) < std::min(a.right, b.right) &&
				   std::max(a.top, b.top) < std::min(a.bottom, b.bottom);
		}

		inline bool contains(const Box &box, int32_t x, int32_t y)
		{
			return x >= box.left && x < box.right && y >= box.top && y < box.bottom;
		}

		// The player's body at (x, y) (FUN_0045183b).
		inline Box user_body(int32_t x, int32_t y, bool prone, bool facing_left)
		{
			Box body = prone ? Box{ -46, -31, 0, 0 } : Box{ -22, -65, 22, 0 };

			if (!facing_left)
				body = body.mirror();

			return body.shift(x, y);
		}

		struct Spot
		{
			int32_t x;
			int32_t y;
		};

		// Where on a player's body an attack's hit shows, from two random
		// numbers: somewhere inside it, or the bare number along a side it
		// has no length on (0x983398-0x9833eb).
		inline Spot hit_spot(const Box &body, uint32_t rx, uint32_t ry)
		{
			uint32_t width = static_cast<uint32_t>(body.right - body.left);
			uint32_t height = static_cast<uint32_t>(body.bottom - body.top);

			return { width ? body.left + static_cast<int32_t>(rx % width) : static_cast<int32_t>(rx),
					 height ? body.top + static_cast<int32_t>(ry % height) : static_cast<int32_t>(ry) };
		}

		// The Sound.nx Mob.img/<id> sound attack `index` (attack1 is 0)
		// plays on the player it hits: slots 11 on of FUN_00989ba1's table
		// (0x98a238), where only the first two and the ninth have one. The
		// table goes on to the Skill sounds past the ninth; those aren't
		// played here.
		inline const char *char_dam_sound(int32_t index)
		{
			switch (index)
			{
				case 0:
					return "CharDam1";
				case 1:
					return "CharDam2";
				case 8:
					return "CharDamF";
				default:
					return nullptr;
			}
		}

		// Whether a cone from (x0, y) towards `end` meets `target`: 20 px
		// strips that widen by a quarter of their distance up and down
		// (FUN_0066afb7).
		inline bool cone_reaches(int32_t x0, int32_t end, int32_t y, const Box &target)
		{
			constexpr int32_t STRIP = 20;

			if (end >= x0)
			{
				for (int32_t x = x0; x < end; x += STRIP)
				{
					int32_t spread = (x - x0) / 4;
					Box strip{ x, y - spread, std::min(x + STRIP, end), y + spread + 1 };

					if (intersects(strip, target))
						return true;
				}
			} else
			{
				for (int32_t x = x0; x > end; x -= STRIP)
				{
					int32_t spread = (x0 - x) / 4;
					Box strip{ std::max(x - STRIP, end), y - spread, x, y + spread + 1 };

					if (intersects(strip, target))
						return true;
				}
			}

			return false;
		}

		// Which way a monster turns to attack, if either way reaches: the
		// way it faces first. A monster that may not turn only tries its own.
		inline std::optional<bool> choose_facing(bool facing_left, bool may_turn, bool reaches_left,
												 bool reaches_right)
		{
			if (facing_left)
			{
				if (reaches_left)
					return true;

				if (may_turn && reaches_right)
					return false;
			} else
			{
				if (reaches_right)
					return false;

				if (reaches_left)
					return true;
			}

			return std::nullopt;
		}

		// Where a shot or ball starts: the attack's `sp`, drawn for a monster
		// facing left.
		inline int32_t origin_x(int32_t x, int32_t sp_x, bool facing_left)
		{
			return facing_left ? x + sp_x : x - sp_x;
		}

		// Where a ball aims at a target: its near edge, 10 px in, and at the
		// shooter's height when that's well inside it.
		inline std::pair<int32_t, int32_t> aim(const Box &target, int32_t origin_y, bool facing_left)
		{
			int32_t x = facing_left ? std::max(target.right - 10, target.center_x())
									: std::min(target.left + 10, target.center_x());
			int32_t y = (target.top + 10 <= origin_y && origin_y <= target.bottom - 10) ? origin_y
																						 : target.center_y();

			return { x, y };
		}

		// Where a shot meets a target, for the range check of a direct
		// attack (type 1).
		inline std::pair<int32_t, int32_t> shot_point(const Box &target, int32_t origin_y, bool facing_left)
		{
			int32_t x = facing_left ? std::max(target.right - 10, target.center_x())
									: std::min(target.left + 10, target.center_x());

			return { x, std::clamp(origin_y, target.top, target.bottom) };
		}

		// Whether a direct attack from (ox, oy) reaches the target.
		inline bool shot_reaches(const Box &target, int32_t ox, int32_t oy, int32_t range, bool facing_left)
		{
			auto [x, y] = shot_point(target, oy, facing_left);
			double distance = std::hypot(static_cast<double>(x - ox), static_cast<double>(y - oy));

			return distance <= range + 10;
		}

		// Where a ball from (ox, oy) aimed at (tx, ty) ends: the full range
		// along its flight, no steeper than 0.6.
		inline std::pair<int32_t, int32_t> ball_end(int32_t ox, int32_t oy, int32_t tx, int32_t ty, int32_t range,
													 bool facing_left)
		{
			double ahead = facing_left ? ox - tx : tx - ox;
			double dy = ty - oy;

			if (ahead < 1.0)
			{
				ahead = 1.0;
				dy = 0.0;
			}

			if (std::abs(dy / ahead) > 0.6)
				dy = dy < 0 ? -0.6 * ahead : 0.6 * ahead;

			double dx = facing_left ? -ahead : ahead;
			double length = std::hypot(dx, dy);

			return { ox + static_cast<int32_t>(dx * range / length), oy + static_cast<int32_t>(dy * range / length) };
		}

		// What a mob's attack does to a Hypnotized mob it reaches: its
		// attack times [0.8, 0.85], plus 100, times its attack / 100, less
		// the other's defense times [0.5, 0.6]; a magic one's attack times
		// [0.75, 0.8] (FUN_00792ae1, FUN_00792c01). Rolls are in [0, 1);
		// it may come out below 0, which v83 sends as it is.
		inline int32_t mob_hit(int32_t attack, int32_t defense, bool magic, double attackroll, double defenseroll)
		{
			double low = magic ? 0.75 : 0.8;
			double reach = low + 0.05 * attackroll;
			double guard = defense < 1 ? 0.0 : defense * (0.5 + 0.1 * defenseroll);

			return static_cast<int32_t>((attack * reach + 100.0) * attack * 0.01 - guard);
		}

		// How long a ball flies over its range, in ms.
		inline int32_t ball_time(int32_t range, int32_t speed)
		{
			if (speed <= 0)
				speed = 100;

			return range * 600 / speed;
		}

		// An area attack (types 3 and 4): cells as wide as its range box,
		// from `start` to `start + count - 1` in front of the monster.
		struct Area
		{
			Box cell;
			int32_t start;
			int32_t count;

			int32_t width() const
			{
				return cell.right - cell.left;
			}
		};

		// The direction the cells run in; data is drawn for a monster facing
		// left, which sees them running right.
		inline int32_t area_sign(bool facing_left)
		{
			return facing_left ? 1 : -1;
		}

		// The whole area an attack covers, which must meet the target to
		// pick it. For a monster facing right it is inside out and meets
		// nothing, so only monsters facing left use these attacks.
		inline Box area_box(const Area &area, int32_t x, int32_t y, bool facing_left)
		{
			int32_t sign = area_sign(facing_left);
			int32_t w = area.width();

			return { x + sign * w * area.start, area.cell.top + y, x + sign * w * (area.start + area.count),
					 area.cell.bottom + y };
		}

		// The middle of cell `cell` (counted from 0 at `start`).
		inline int32_t cell_x(const Area &area, int32_t x, int32_t cell, bool facing_left)
		{
			int32_t w = area.width();

			return x + area_sign(facing_left) * ((area.start + cell) * w + w / 2);
		}

		// Where a cell's hit lands, with the cell's middle at (x, y).
		inline Box cell_box(const Area &area, int32_t x, int32_t y, bool facing_left)
		{
			Box box = facing_left ? area.cell : area.cell.mirror();

			return box.shift(x, y);
		}

		// The action a skill's animation skill<n> is sent as: the skills'
		// actions follow the nine attacks' (FUN_0066e7af).
		inline int8_t skill_action(uint8_t n)
		{
			return static_cast<int8_t>(n - 1 + 21);
		}

		// What a cast sends: the skill, its level, and how long its
		// animation plays.
		inline int32_t skill_data(uint8_t skill, uint8_t level, int32_t duration)
		{
			return skill | level << 8 | duration << 16;
		}

		// The skill and level a cast sent.
		inline std::pair<uint8_t, uint8_t> cast_skill(int32_t skilldata)
		{
			return { static_cast<uint8_t>(skilldata & 0xFF), static_cast<uint8_t>((skilldata >> 8) & 0xFF) };
		}

		// An ack keeps the skill the mob was let cast unless it lets it
		// cast another (FUN_0066c23b).
		inline std::pair<uint8_t, uint8_t> granted_skill(std::pair<uint8_t, uint8_t> pending,
														 std::pair<uint8_t, uint8_t> granted)
		{
			return pending.first == 0 || granted.first != 0 ? granted : pending;
		}

		// How far the local player is from a firstAttack mob: a tenth of
		// the x distance and a third of the y one, each rounded down
		// (FUN_0066e32d, FUN_0066e146).
		inline int32_t first_attack_distance(int32_t dx, int32_t dy)
		{
			return std::abs(dx) / 10 + std::abs(dy) / 3;
		}
	}
}
