#pragma once

#include <cstdint>

namespace ms
{
	// Combo Attack: the orbs a Crusader's (or a Dawn Warrior's) swings gather, which make every swing and
	// shot stronger until a finisher spends them. The Combo buff's value is one more than the orbs.
	namespace combo
	{
		// Panic and Coma, a Crusader's and a Dawn Warrior's.
		inline bool finisher(int32_t skill)
		{
			switch (skill)
			{
				case 1111003:
				case 1111004:
				case 1111005:
				case 1111006:
				case 11111002:
				case 11111003:
					return true;
				default:
					return false;
			}
		}

		// A Hero's Enrage, which spends the orbs.
		constexpr int32_t ENRAGE = 1121010;

		// Enrage wants a Combo value of 10, nine orbs: v83 compares the value, not the orbs
		// (0x764379).
		inline bool enrage_refused(int32_t skill, int32_t value)
		{
			return skill == ENRAGE && value < 10;
		}

		// Combo Attack and Advanced Combo for a job's line, and Combo Attack's last level.
		struct Line
		{
			int32_t combo;
			int32_t advanced;
			int32_t last;
		};

		inline Line line(int32_t job)
		{
			if (job / 1000 == 1)
				return { 11111001, 11110005, 20 };

			return { 1111002, 1120003, 30 };
		}

		// The skill whose `damage` the table starts from: Advanced Combo once Combo Attack is at its last
		// level.
		inline int32_t base_skill(const Line &line, int32_t level, int32_t advanced)
		{
			return level == line.last && advanced > 0 ? line.advanced : line.combo;
		}

		// The percent the orbs make an attack's damage, after its skill's, or 0 when they leave it alone
		// (FUN_007653fa), with Combo Attack at `level` (not 0) and `base` its table's `damage`.
		inline int32_t percent(bool finisher, int32_t level, int32_t base, int32_t orbs)
		{
			static constexpr int32_t STEPS[4] = { 6, 16, 30, 46 };
			static constexpr int32_t MORE[5] = { 24, 28, 32, 36, 40 };
			static constexpr int32_t FINISHED[5] = { 250, 270, 290, 305, 320 };

			if (!finisher && orbs < 0)
				return 0;

			if (orbs == 1)
				return base;

			if (orbs >= 2 && orbs <= 5)
			{
				if (!finisher)
					return base + 5 * (orbs - 1) * (level - 1) / 29;

				int32_t step = STEPS[orbs - 2];

				return base + step + 4 * step * (level - 1) / 29;
			}

			if (orbs >= 6 && orbs <= 10)
				return base + (finisher ? FINISHED : MORE)[orbs - 6];

			return 100;
		}
	}
}
