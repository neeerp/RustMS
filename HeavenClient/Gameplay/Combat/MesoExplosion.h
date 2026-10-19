#pragma once

#include "../../Template/Point.h"
#include "../../Template/Rectangle.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <utility>
#include <vector>

namespace ms
{
	// Meso Explosion, as in the MapleRoyals client (FUN_0096b3fb): the mesos
	// lying in front of a Chief Bandit go off, and each hurts the mobs around
	// it by what it is worth.
	namespace meso_explosion
	{
		constexpr int32_t ID = 4211006;
		// It sets off no more drops than this, of which only the first 15
		// hurt anything (FUN_00791fbc), and hurts no more than 15 mobs.
		constexpr int32_t MOST_DROPS = 20;
		constexpr size_t HURTING_DROPS = 15;
		constexpr int32_t MOST_MOBS = 15;
		// A drop hurts the mobs within 50 px of it.
		constexpr int16_t BLAST = 50;
		// A mob shows its hits as at most five numbers.
		constexpr size_t NUMBERS = 5;
		// The explosions' sound plays no more often than this, in ms
		// (FUN_00504bff).
		constexpr int32_t SOUND_GAP = 90;

		// Where it looks for mesos: the skill's box, as facing left, around
		// the player's feet, mirrored when they face right.
		inline Rectangle<int16_t> area(Rectangle<int16_t> range, Point<int16_t> feet, bool toleft)
		{
			int16_t left = toleft ? range.left() : static_cast<int16_t>(-range.right());
			int16_t right = toleft ? range.right() : static_cast<int16_t>(-range.left());

			return {
				static_cast<int16_t>(feet.x() + left),
				static_cast<int16_t>(feet.x() + right),
				static_cast<int16_t>(feet.y() + range.top()),
				static_cast<int16_t>(feet.y() + range.bottom())
			};
		}

		// Whether a drop at `spot` is in it: as PtInRect, without its right
		// and bottom edges.
		inline bool in_area(Rectangle<int16_t> area, Point<int16_t> spot)
		{
			return spot.x() >= area.left() && spot.x() < area.right()
				   && spot.y() >= area.top() && spot.y() < area.bottom();
		}

		// Whether the player may set off mesos of this type (0 a
		// character's, 1 a party's, 2 anyone's, 3 explosive): unlike
		// picking them up, however long they have lain (FUN_0050579e).
		inline bool may_set_off(int8_t type, int32_t owner, int32_t self, int32_t party)
		{
			return (type != 0 || owner == self) && (type != 1 || owner == party);
		}

		struct Drop
		{
			int32_t oid;
			int32_t mesos;
			Point<int16_t> spot;
		};

		// The drops it sets off, of those `found` in its box in the order
		// they entered the map: the first `attackcount`, up to 20, cheapest
		// first.
		inline std::vector<Drop> choose(std::vector<Drop> found, int32_t attackcount)
		{
			size_t most = static_cast<size_t>(std::clamp(attackcount, 1, MOST_DROPS));

			if (found.size() > most)
				found.resize(most);

			std::stable_sort(found.begin(), found.end(), [](const Drop &a, const Drop &b) { return a.mesos < b.mesos; });

			return found;
		}

		// A mob it may hurt.
		struct Mob
		{
			int32_t oid;
			Rectangle<int16_t> body;
		};

		// Whether two boxes share any area, as IntersectRect.
		inline bool crosses(Rectangle<int16_t> a, Rectangle<int16_t> b)
		{
			return std::max(a.left(), b.left()) < std::min(a.right(), b.right())
				   && std::max(a.top(), b.top()) < std::min(a.bottom(), b.bottom());
		}

		// The mobs it hurts, each with the drops that hit it (bit i for the
		// i-th of `drops`), and each drop with the mobs it hit (bit j for the
		// j-th of them).
		struct Targets
		{
			std::vector<std::pair<int32_t, uint32_t>> mobs;
			std::vector<uint8_t> drops;
		};

		// Which of `mobs`, in the order they entered the map, each drop hits:
		// up to `mobcount` mobs with a drop on them, and 15 drops on each
		// (FUN_00678606).
		inline Targets aim(const std::vector<Drop> &drops, const std::vector<Mob> &mobs, int32_t mobcount)
		{
			size_t most = static_cast<size_t>(std::clamp(mobcount, 1, MOST_MOBS));
			Targets targets;
			targets.drops.resize(drops.size());

			for (const Mob &mob : mobs)
			{
				if (targets.mobs.size() >= most)
					break;

				uint32_t mask = 0;
				size_t hits = 0;

				for (size_t i = 0; i < drops.size() && hits < HURTING_DROPS; i++)
				{
					Point<int16_t> spot = drops[i].spot;
					Rectangle<int16_t> blast(
						static_cast<int16_t>(spot.x() - BLAST),
						static_cast<int16_t>(spot.x() + BLAST),
						static_cast<int16_t>(spot.y() - BLAST),
						static_cast<int16_t>(spot.y() + BLAST)
					);

					if (crosses(blast, mob.body))
					{
						mask |= 1u << i;
						hits++;
					}
				}

				if (hits == 0)
					continue;

				for (size_t i = 0; i < drops.size(); i++)
					if (mask & (1u << i))
						targets.drops[i] |= static_cast<uint8_t>(1u << targets.mobs.size());

				targets.mobs.emplace_back(mob.oid, mask);
			}

			return targets;
		}

		// The mesos of the drops on a mob with this `mask` that hurt it.
		inline std::vector<int32_t> hurting(const std::vector<Drop> &drops, uint32_t mask)
		{
			std::vector<int32_t> mesos;

			for (size_t i = 0; i < std::min(drops.size(), HURTING_DROPS); i++)
				if (mask & (1u << i))
					mesos.push_back(drops[i].mesos);

			return mesos;
		}

		// What share of 50·x one line from a drop of `mesos` deals at most
		// (0x7920a3).
		inline double ratio(int32_t mesos)
		{
			if (mesos <= 1000)
				return (mesos * 0.82 + 28.0) / 5300.0;

			return mesos / (mesos + 5250.0);
		}

		// One line from a drop of `mesos`, with the skill's `x` and `roll`
		// from 0.5 up to 1.
		inline int32_t line(int32_t x, int32_t mesos, double roll)
		{
			return static_cast<int32_t>(x * 50 * roll * ratio(mesos));
		}

		// Its lines on one mob as they are shown: the i-th added to the
		// (i % 5)-th number.
		inline std::vector<int32_t> fold(const std::vector<int32_t> &lines)
		{
			std::vector<int32_t> numbers(std::min(lines.size(), NUMBERS), 0);

			for (size_t i = 0; i < lines.size(); i++)
				numbers[i % NUMBERS] += lines[i];

			return numbers;
		}

		// What a number shows on a mob with its `fixed` damage, and that it
		// may `hurt` or not.
		inline int32_t shown(int32_t number, int32_t fixed, bool hurt)
		{
			if (!hurt)
				return 0;

			return fixed > 0 && number > 0 ? fixed : number;
		}
	}
}
