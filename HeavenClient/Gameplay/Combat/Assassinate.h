#pragma once

#include <algorithm>
#include <cstdint>

namespace ms
{
	// Assassinate: from Dark Sight, which it cancels, a strike of three lines, then, an attack delay later, a
	// second of one line on the first strike's mob, and a dash forward (FUN_00950921, 0x957977, 0x96ce79).
	namespace assassinate
	{
		constexpr int32_t ID = 4221001;
		constexpr int32_t DARK_SIGHT = 4001003;

		// The second strike's mark in the attack's display byte (0x952956).
		constexpr uint8_t SECOND_STRIKE = 0x20;

		// How far forward the second strike dashes, its `lt.x` (0x950da5).
		constexpr int32_t DASH = 100;

		// Whether Dark Sight refuses a skill: in it only Assassinate goes, and Assassinate only from it
		// (0x96705f, 0x9694ef).
		inline bool refused(int32_t skill, bool hidden)
		{
			if (skill == 0)
				return false;

			return hidden ? skill != ID : skill == ID;
		}

		// What both strikes multiply their damage by: one more for every 3 whole seconds since Dark Sight
		// was given, counting at most the skill's `time`, or 0 if it never was (0x79028f).
		inline int32_t multiplier(int64_t hidden_ms, int32_t time)
		{
			if (hidden_ms < 0)
				return 0;

			int64_t seconds = std::min<int64_t>(hidden_ms / 1000, time);

			return static_cast<int32_t>(seconds / 3) + 1;
		}
	}
}
