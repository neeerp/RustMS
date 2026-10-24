#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The clocks of timed quests (QuestInfo timeLimit), which UPDATE_QUEST_INFO modes 6, 7, 15 and 16 put up, take
	// down and reset (v83 FUN_00959a73; report b149).
	namespace timed_quests
	{
		struct Clock
		{
			int16_t quest;
			// timeLimit in ms, which the gauge and the clock's frames measure against.
			int32_t total;
			int32_t remaining;
			int16_t x, y;
			// Tells clocks of the same quest apart.
			uint32_t serial;
		};

		// The first clock's place on the screen; each one put up later goes under the last (FUN_00a24d5d).
		constexpr int16_t FIRST_X = 700;
		constexpr int16_t FIRST_Y = 50;
		constexpr int16_t SPACING = 50;

		// The gauge has this many 1 px steps (FUN_0088ea75).
		constexpr int32_t SEGMENTS = 48;

		// Put up a clock with `ms` left of `total`. v83 doesn't look for one already up for the quest.
		inline void add(std::vector<Clock>& clocks, int16_t quest, int32_t total, int32_t ms)
		{
			int16_t x = FIRST_X;
			int16_t y = FIRST_Y;

			if (!clocks.empty())
			{
				x = clocks.back().x;
				y = clocks.back().y + SPACING;
			}

			static uint32_t next = 0;
			clocks.push_back({ quest, total, ms, x, y, ++next });
		}

		// The clocks up, which stay through map and channel changes; a new login takes them down.
		inline std::vector<Clock>& shown()
		{
			static std::vector<Clock> clocks;

			return clocks;
		}

		// Take down the quest's first clock; the others stay where they are.
		inline bool remove(std::vector<Clock>& clocks, int16_t quest)
		{
			for (auto it = clocks.begin(); it != clocks.end(); ++it)
			{
				if (it->quest == quest)
				{
					clocks.erase(it);

					return true;
				}
			}

			return false;
		}

		// Mode 16: the quest's clock has its whole time again (FUN_00a24ed3).
		inline void reset(std::vector<Clock>& clocks, int16_t quest)
		{
			for (Clock& clock : clocks)
			{
				if (clock.quest == quest)
				{
					clock.remaining = clock.total;

					return;
				}
			}
		}

		// The time goes on; at 0 a clock stays until the server takes it down.
		inline void tick(std::vector<Clock>& clocks, int32_t elapsed)
		{
			for (Clock& clock : clocks)
				clock.remaining = clock.remaining > elapsed ? clock.remaining - elapsed : 0;
		}

		// How many of the gauge's steps are lit: 1 to 49, fewer as the time runs out.
		inline int32_t segments(const Clock& clock)
		{
			int32_t step = clock.total / SEGMENTS;

			if (step <= 0)
				return SEGMENTS + 1;

			int32_t lit = clock.remaining / step + 1;

			return lit > SEGMENTS + 1 ? SEGMENTS + 1 : lit;
		}

		// The frame of `frames` the clock shows: they spread over the whole time, and the last is left for when
		// it is taken down (FUN_0088dcc6).
		inline int32_t frame(const Clock& clock, int32_t frames)
		{
			if (frames < 2)
				return 0;

			int32_t step = clock.total / (frames - 1);

			if (step <= 0)
				return frames - 2;

			int32_t elapsed = clock.total > clock.remaining ? clock.total - clock.remaining : 0;
			int32_t shown = elapsed / step;

			return shown >= frames - 1 ? frames - 2 : shown;
		}

		// The tooltip, unpadded (pool 0xfeb, FUN_0088eeb4).
		inline std::string time_left(int32_t ms)
		{
			int32_t seconds = ms / 1000;

			return "Time Left " + std::to_string(seconds / 3600) + ":" + std::to_string(seconds % 3600 / 60) + ":"
				+ std::to_string(seconds % 60);
		}
	}
}
