#pragma once

#include <cstdint>

namespace ms
{
	// What a character's job, stats and passive skills add to its accuracy,
	// avoid, attack, defence and speed, as the MapleRoyals client totals
	// them (SecondaryStat::SetFrom, FUN_0077f4c9).
	namespace secondary_stat
	{
		// Whether `job` is `line` or a later job of it, the way the client
		// matches a skill's job (FUN_007669dd): 311 takes in 312, and 300
		// every bowman.
		inline bool in_line(int32_t job, int32_t line)
		{
			if (line % 100 == 0)
				return line / 100 == job / 100;

			return line / 10 == job / 10 && line % 10 <= job % 10;
		}

		// A Brawler's, a pirate's, an archer's and a thief's DEX counts for
		// less (0x77f663).
		inline int32_t base_accuracy(int32_t job, int32_t dex, int32_t luk)
		{
			if (job / 10 == 51)
				return static_cast<int32_t>(dex * 0.9 + luk * 0.3);

			if (job / 100 == 5 || job % 1000 / 100 == 3 || job % 1000 / 100 == 4)
				return static_cast<int32_t>(dex * 0.6 + luk * 0.3);

			return static_cast<int32_t>(dex * 0.8 + luk * 0.5);
		}

		// A Brawler dodges with one and a half times their DEX, a
		// Gunslinger with an eighth of it, and everyone else with a quarter;
		// half of LUK, rounded down, adds to each (0x77f54f).
		inline int32_t base_avoid(int32_t job, int32_t dex, int32_t luk)
		{
			if (job / 10 == 51)
				return static_cast<int32_t>(dex * 1.5 + static_cast<double>(luk / 2));

			if (job / 10 == 52)
				return dex / 8 + luk / 2;

			return dex / 4 + luk / 2;
		}

		// The skills that add their x to accuracy and their y to avoid:
		// Blessing of Amazon, Nimble Body (both), Bullet Time and Quick
		// Motion (setfrom.c:313-386).
		constexpr int32_t NIMBLE_SKILLS[] = { 3000000, 4000000, 14000000, 5000000, 15000000 };

		// The Shield Mastery a job may have learned, or 0 (FUN_00764b74).
		inline int32_t shield_mastery(int32_t job)
		{
			if (in_line(job, 111))
				return 1110001;

			if (in_line(job, 121))
				return 1210001;

			if (in_line(job, 421))
				return 4210000;

			return 0;
		}

		// A shield's defence under Shield Mastery's x percent; with none
		// learned, x is 100.
		inline int32_t shield_defence(int32_t defence, int32_t percent)
		{
			return defence * percent / 100;
		}

		// The Thrusts, which add their speed (FUN_00765c5e): the Ranger's,
		// the Sniper's and the Wind Archer's.
		constexpr int32_t THRUSTS[] = { 3110000, 3210000, 13100004 };

		// A pirate with no weapon still hits with 10 attack and 0.7 more a
		// level, at most 31 (0x77f89e).
		inline int32_t bare_hands_attack(int32_t job, int32_t level)
		{
			if (job % 1000 / 100 != 5)
				return 0;

			if (level >= 31)
				return 31;

			return static_cast<int32_t>(level * 0.7 + 10.0);
		}
	}
}
