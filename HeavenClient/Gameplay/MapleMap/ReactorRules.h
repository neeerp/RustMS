#pragma once

#include <algorithm>
#include <cstdint>
#include <set>
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

		// The event type a skill used by a reactor sets off.
		constexpr int32_t SKILL_USED = 5;

		// A reactor's event, as a skill used by it sees it: its type, the
		// skills it takes, and where the user must stand, from the reactor.
		struct SkillEvent
		{
			int32_t type = 0;
			std::vector<int32_t> skills;
			Box area;
		};

		// The event `skill`, used by a user `x` across and `y` down from the
		// reactor, sets off: the first of type 5 that lists the skill and
		// whose area holds the user, or -1 (FUN_00735ab7).
		inline int32_t pick_skill_event(const std::vector<SkillEvent> &events, int32_t skill, int32_t x, int32_t y)
		{
			for (size_t i = 0; i < events.size(); i++)
			{
				const SkillEvent &event = events[i];
				const Box &area = event.area;

				if (event.type != SKILL_USED
					|| std::find(event.skills.begin(), event.skills.end(), skill) == event.skills.end())
					continue;

				if (x >= area.left && x < area.right && y >= area.top && y < area.bottom)
					return static_cast<int32_t>(i);
			}

			return -1;
		}

		inline bool empty(const Box &box)
		{
			return box.right <= box.left || box.bottom <= box.top;
		}

		inline bool overlap(const Box &a, const Box &b)
		{
			return std::max(a.left, b.left) < std::min(a.right, b.right) &&
				   std::max(a.top, b.top) < std::min(a.bottom, b.bottom);
		}

		// Whether the point is in the box, as PtInRect has it.
		inline bool contains(const Box &box, int32_t x, int32_t y)
		{
			return x >= box.left && x < box.right && y >= box.top && y < box.bottom;
		}

		// A reactor set off by touch (`info/activateByTouch`), where it is
		// drawn.
		struct Touchable
		{
			int32_t oid = 0;
			Box box;
		};

		// The user stepped onto a reactor (`inside`) or off it.
		struct TouchChange
		{
			int32_t oid = 0;
			bool inside = false;

			bool operator==(const TouchChange &other) const
			{
				return oid == other.oid && inside == other.inside;
			}
		};

		// The reactors the user at (x, y) stepped onto or off since the last
		// poll, which `touched` holds those they stood on at (FUN_00735d90):
		// each whose box holds the user and isn't in it goes in, and each in
		// it whose box doesn't comes out. One with an empty box is passed
		// over, and one gone stays in until the field changes.
		inline std::vector<TouchChange> touch_changes(const std::vector<Touchable> &reactors, int32_t x, int32_t y,
													  std::set<int32_t> &touched)
		{
			std::vector<TouchChange> changes;

			for (const Touchable &reactor : reactors)
			{
				if (empty(reactor.box))
					continue;

				bool inside = contains(reactor.box, x, y);

				if (inside && touched.insert(reactor.oid).second)
					changes.push_back({ reactor.oid, true });
				else if (!inside && touched.erase(reactor.oid))
					changes.push_back({ reactor.oid, false });
			}

			return changes;
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
