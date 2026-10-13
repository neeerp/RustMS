#pragma once

#include "../../Template/Point.h"

#include <cstdint>
#include <cstdlib>

namespace ms
{
	// Which NPC the Interact key talks to: the nearest one whose feet are
	// within a short walk of the player's, on about the same level.
	namespace npc_reach
	{
		constexpr int16_t RANGE_X = 150;
		constexpr int16_t RANGE_Y = 50;

		inline bool in_reach(Point<int16_t> player, Point<int16_t> npc)
		{
			return std::abs(npc.x() - player.x()) <= RANGE_X && std::abs(npc.y() - player.y()) <= RANGE_Y;
		}

		// Whether `npc` is nearer to the player than `other`; horizontal
		// distance first, then vertical.
		inline bool nearer(Point<int16_t> player, Point<int16_t> npc, Point<int16_t> other)
		{
			int32_t dx = std::abs(npc.x() - player.x());
			int32_t other_dx = std::abs(other.x() - player.x());

			if (dx != other_dx)
				return dx < other_dx;

			return std::abs(npc.y() - player.y()) < std::abs(other.y() - player.y());
		}
	}
}
