#pragma once

#include <algorithm>
#include <cstdint>

// The rules behind v83's AP Reset (5050000): which stat an AP may leave and which it may go to, and by how much
// Max HP and MP move (FUN_008cbddb, report 195).
namespace ms::ap_reset
{
	constexpr int32_t ITEM = 5050000;

	// The dialogs' rows, top to bottom; the order of `stat0` and `stat1` too.
	enum Row
	{
		HP,
		MP,
		STR,
		DEX,
		INT,
		LUK,
		NUM_ROWS
	};

	// What 0x4F sends for each row: its STAT_CHANGED mask bit.
	constexpr int32_t FLAGS[NUM_ROWS] = { 0x800, 0x2000, 0x40, 0x80, 0x100, 0x200 };

	struct Stats
	{
		int32_t job;
		int32_t level;
		int32_t str, dex, intl, luk;
		int32_t ap;
		int32_t max_hp, max_mp;
		// INT with what is worn and the buffs on, for a magician's Max MP.
		int32_t total_int;
	};

	// 0 beginner, 1 warrior, 2 magician, 3 bowman, 4 thief, 5 pirate; the same for Cygnus.
	inline int32_t branch(int32_t job)
	{
		return (job % 1000) / 100;
	}

	// GMs (900) can't use it.
	inline bool may_use(int32_t job)
	{
		return branch(job) <= 8;
	}

	inline int32_t hp_loss(int32_t job)
	{
		switch (branch(job))
		{
			case 1: return 54;
			case 2: return 10;
			case 3: case 4: return 20;
			case 5: return 42;
			default: return 12;
		}
	}

	// FUN_008ce5bd.
	inline int32_t mp_loss(int32_t job, int32_t total_int)
	{
		switch (branch(job))
		{
			case 1: return 4;
			case 2: return total_int * 3 / 40 + 30;
			case 3: case 4: return 12;
			case 5: return 16;
			default: return 8;
		}
	}

	inline int32_t hp_gain(int32_t job)
	{
		switch (branch(job))
		{
			case 1: return 20;
			case 2: return 6;
			case 3: case 4: return 16;
			case 5: return 18;
			default: return 8;
		}
	}

	inline int32_t mp_gain(int32_t job)
	{
		switch (branch(job))
		{
			case 1: return 2;
			case 2: return 18;
			case 3: case 4: return 10;
			case 5: return 14;
			default: return 6;
		}
	}

	// The least Max HP a job keeps (FUN_008cd10a).
	inline int32_t min_hp(int32_t job, int32_t level)
	{
		int32_t j = job % 1000;

		if (j == 0)
			return 12 * level + 38;
		if (j == 100 || (j >= 120 && j <= 122) || (j >= 130 && j <= 132))
			return 24 * level + 118;
		if (j >= 110 && j <= 112)
			return 24 * level + 418;
		if (j >= 200 && j <= 299)
			return 10 * level + 54;
		if (j == 300 || j == 400)
			return 20 * level + 58;
		if ((j >= 310 && j <= 312) || (j >= 320 && j <= 322) || (j >= 410 && j <= 412) || (j >= 420 && j <= 422))
			return 20 * level + 358;
		if (j == 500)
			return 22 * level + 38;
		if ((j >= 510 && j <= 512) || (j >= 520 && j <= 522))
			return 22 * level + 338;

		return 0;
	}

	// The least Max MP a job keeps (FUN_008cd369).
	inline int32_t min_mp(int32_t job, int32_t level)
	{
		int32_t j = job % 1000;

		if (j == 0)
			return 10 * level - 5;
		if (j == 100 || (j >= 110 && j <= 112))
			return 4 * level + 55;
		if ((j >= 120 && j <= 122) || (j >= 130 && j <= 132))
			return 4 * level + 155;
		if (j == 200)
			return 22 * level - 1;
		if ((j >= 210 && j <= 212) || (j >= 220 && j <= 222) || (j >= 230 && j <= 232))
			return 22 * level + 449;
		if (j == 300 || j == 400)
			return 14 * level - 15;
		if ((j >= 310 && j <= 312) || (j >= 320 && j <= 322) || (j >= 410 && j <= 412) || (j >= 420 && j <= 422))
			return 14 * level + 135;
		if (j == 500)
			return 18 * level - 55;
		if ((j >= 510 && j <= 512) || (j >= 520 && j <= 522))
			return 18 * level + 95;

		return 0;
	}

	// Whether any AP went into Max HP or MP: the main stats and AP left hold less than every AP earned.
	inline bool has_hp_mp_ap(const Stats &s)
	{
		int32_t earned = 20 + 5 * s.level;

		if (s.job / 1000 == 1)
			earned += std::min(s.level - 1, 69);

		earned += s.job % 10 == 1 ? 5 : s.job % 10 == 2 ? 10 : 0;

		return s.str + s.dex + s.intl + s.luk + s.ap < earned;
	}

	inline int32_t value(const Stats &s, Row row)
	{
		switch (row)
		{
			case HP: return s.max_hp;
			case MP: return s.max_mp;
			case STR: return s.str;
			case DEX: return s.dex;
			case INT: return s.intl;
			case LUK: return s.luk;
			default: return 0;
		}
	}

	// What an AP taken out of `row` takes with it, and what one put there adds.
	inline int32_t loss(const Stats &s, Row row)
	{
		return row == HP ? hp_loss(s.job) : row == MP ? mp_loss(s.job, s.total_int) : 1;
	}

	inline int32_t gain(const Stats &s, Row row)
	{
		return row == HP ? hp_gain(s.job) : row == MP ? mp_gain(s.job) : 1;
	}

	// A main stat keeps 4, and what the job's advancement asked for.
	inline int32_t stat_floor(int32_t job, Row row)
	{
		int32_t b = branch(job);
		int32_t required = 0;

		if (b == 1 && row == STR)
			required = 35;
		else if (b == 2 && row == INT)
			required = 20;
		else if ((b == 3 || b == 4) && row == DEX)
			required = 25;
		else if (b == 5 && row == DEX)
			required = 20;

		return std::max(required, 4);
	}

	// Whether the first dialog lets an AP leave `row`.
	inline bool may_take(const Stats &s, Row row)
	{
		switch (row)
		{
			case HP:
				return has_hp_mp_ap(s) && s.max_hp - hp_loss(s.job) >= min_hp(s.job, s.level);
			case MP:
				return has_hp_mp_ap(s) && s.max_mp - mp_loss(s.job, s.total_int) >= min_mp(s.job, s.level);
			default:
				return value(s, row) > stat_floor(s.job, row);
		}
	}

	constexpr int32_t MAX_HP_MP = 30000;
	constexpr int32_t MAX_STAT = 999;

	// Whether the second lets it go to `row`, having left `from`. v83 adds Max HP and MP's gain twice before
	// comparing it with 30000 (8cc216 and 8cd64f).
	inline bool may_give(const Stats &s, Row row, Row from)
	{
		if (row == from)
			return false;

		switch (row)
		{
			case HP:
				return s.max_hp + 2 * hp_gain(s.job) <= MAX_HP_MP;
			case MP:
				return s.max_mp + 2 * mp_gain(s.job) <= MAX_HP_MP;
			default:
				return value(s, row) < MAX_STAT;
		}
	}
}
