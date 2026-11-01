#pragma once

#include <array>
#include <cstdint>
#include <string>
#include <vector>

// The Stat window's AP auto-assign, as v83 works it out (report 181-stat-auto-assign).
namespace ms::auto_assign
{
	// STAT_CHANGED's mask bits for the stats it spreads AP over.
	constexpr int32_t STR = 0x40, DEX = 0x80, INT = 0x100, LUK = 0x200;

	struct Pair
	{
		int32_t flag;
		int32_t amount;
	};

	// The stats as the window counts them, equipment and all.
	struct Totals
	{
		int32_t str, dex, intl, luk;
	};

	// Whether a beginner of level 10 or less, who has no AP buttons at all (0x8c79f5).
	inline bool no_buttons(int32_t job, int32_t level)
	{
		bool beginner = job == 0 || job == 1000 || job == 2000 || job == 2001;

		return beginner && level <= 10;
	}

	// Whether the auto button `which` shows (0x8c5845): 0 is BtAuto, for every job but the first-job
	// Pirate; 1 and 2 are its Brawler and Gunslinger halves. GMs get none.
	inline bool shown(int32_t job, int which)
	{
		if (job % 1000 / 100 == 9)
			return false;

		return which == 0 ? job != 500 : job == 500;
	}

	// What each of the two stats the job builds still needs to reach its level's "ideal", in the order
	// v83 sends them, or nothing for a job it can't build (0x7939ba). `brawler` picks the first-job
	// Pirate's half.
	inline std::vector<Pair> distribute(int32_t job, int32_t level, const Totals &have, bool brawler)
	{
		auto need = [](int32_t target, int32_t stat) { return target > stat ? target - stat : 0; };
		int32_t jm = job % 1000;
		int32_t l = level;

		switch (jm / 100)
		{
			case 0:
				if (jm != 0 && job != 2001)
					return {};

				return { { DEX, need(l, have.dex) }, { STR, need(4 * l + 12, have.str) } };
			case 1:
			{
				int32_t dex = l > 30 ? l + 30 : 2 * l;

				return { { DEX, need(dex, have.dex) }, { STR, need(5 * l - dex + 12, have.str) } };
			}
			case 2:
			{
				int32_t luk = l + 3;

				return { { LUK, need(luk, have.luk) }, { INT, need(5 * l - luk + 12, have.intl) } };
			}
			case 3:
			{
				int32_t str;

				if (jm == 300 || (jm >= 310 && jm <= 312))
					str = l + 5;
				else if (jm >= 320 && jm <= 322)
					str = l;
				else
					return {};

				return { { STR, need(str, have.str) }, { DEX, need(5 * l - str + 12, have.dex) } };
			}
			case 4:
			{
				int32_t dex = l > 40 ? l + 40 : 2 * l;

				return { { DEX, need(dex, have.dex) }, { LUK, need(5 * l - dex + 12, have.luk) } };
			}
			case 5:
			{
				bool first = jm == 500;
				bool knuckle = jm / 10 == 51 || job == 1500 || (first && brawler);
				bool gun = !knuckle && (jm / 10 == 52 || (first && !brawler));

				if (knuckle)
				{
					int32_t dex = l > 20 ? l + 20 : 2 * l;

					return { { DEX, need(dex, have.dex) }, { STR, need(5 * l - dex + 12, have.str) } };
				}

				if (gun)
					return { { STR, need(l, have.str) }, { DEX, need(4 * l + 12, have.dex) } };

				return {};
			}
			default:
				return {};
		}
	}

	// Each pair takes what it needs of the AP in turn, and the last takes whatever is left (0x8c8692).
	inline void spread(std::vector<Pair> &pairs, int32_t ap)
	{
		for (Pair &pair : pairs)
		{
			if (pair.amount > ap)
				pair.amount = ap;

			ap -= pair.amount;
		}

		if (ap > 0 && !pairs.empty())
			pairs.back().amount += ap;
	}

	// The confirmation's text: every stat, in order, with what it gets (SP 0x79f, 0x7a0).
	inline std::string message(const std::vector<Pair> &pairs)
	{
		static constexpr std::array<const char *, 4> NAMES = { "STR", "DEX", "INT", "LUK" };
		std::array<int32_t, 4> gains{};

		for (const Pair &pair : pairs)
		{
			size_t i = pair.flag == DEX ? 1 : pair.flag == INT ? 2 : pair.flag == LUK ? 3 : 0;
			gains[i] = pair.amount;
		}

		std::string text = "    Your AP will be distributed as follows:\\n\\n";

		for (size_t i = 0; i < NAMES.size(); i++)
			text += "     " + std::string(NAMES[i]) + " : +" + std::to_string(gains[i]) + "\\n";

		return text;
	}
}
