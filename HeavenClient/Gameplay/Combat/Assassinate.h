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
		constexpr int32_t NIGHT_SIGHT = 14001003;
		constexpr int32_t VANISH = 14100005;
		constexpr int32_t WIND_WALK = 13101006;

		// The second strike's mark in the attack's display byte (0x952956).
		constexpr uint8_t SECOND_STRIKE = 0x20;

		// How far forward the second strike dashes, its `lt.x` (0x950da5).
		constexpr int32_t DASH = 100;

		// The dash's element in the movement path (0x952dff).
		constexpr uint8_t MOVE = 8;

		// Whether Dark Sight refuses a skill: in it only Assassinate goes, and Assassinate only from it
		// (0x96705f, 0x9694ef). The owner of Vanish may use anything in it but a Night Walker's Dark Sight.
		inline bool refused(int32_t skill, bool hidden, bool vanish = false)
		{
			if (skill == 0)
				return false;

			if (!hidden)
				return skill == ID;

			return vanish ? skill == NIGHT_SIGHT : skill != ID;
		}

		// The Dark Sight of the job, which the attack key cancels (0x94e366).
		constexpr int32_t dark_sight(int32_t job)
		{
			return job / 1000 == 1 ? NIGHT_SIGHT : DARK_SIGHT;
		}

		// Whether the attack key only ends Dark Sight, which it does without Vanish (0x94e32c, 0x957279).
		constexpr bool attack_ends(bool hidden, bool vanish)
		{
			return hidden && !vanish;
		}

		// Wind Walk on already, casting it does nothing (0x96a0e1).
		constexpr bool recast(int32_t skill, bool walking)
		{
			return skill == WIND_WALK && walking;
		}

		// What a line of a physical attack is made, in percent, by the `damage` of Wind Walk while it is on and
		// of Vanish while in Dark Sight; 0 for a skill that doesn't apply (0x790315, 0x790376).
		constexpr int32_t unseen(int32_t wind_walk, int32_t vanish)
		{
			int32_t percent = 100;

			if (wind_walk)
				percent = percent * wind_walk / 100;

			if (vanish)
				percent = percent * vanish / 100;

			return percent;
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
