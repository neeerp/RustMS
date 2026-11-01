#pragma once

#include <algorithm>
#include <array>
#include <cstdint>

// The stats a map pins (FORCED_STAT_SET), kept until FORCED_STAT_RESET lets them go, as v83 keeps them
// in CWvsContext's ForcedStat (report forced-stats).
namespace ms::forced_stat
{
	// The slots in the packet's mask order (FUN_0077bf9a).
	enum Slot
	{
		STR,
		DEX,
		INT,
		LUK,
		PAD,
		PDD,
		MAD,
		MDD,
		ACC,
		EVA,
		SPEED,
		JUMP,
		SPEED_MAX,
		LENGTH
	};

	// 0 leaves a stat alone.
	using Values = std::array<int32_t, LENGTH>;

	// SP 0x36e and 0x373, while any stat is pinned (FUN_0077c129).
	constexpr const char *EQUIP_LOCKED = "You cannot change your equipment on this map.";
	constexpr const char *STATS_LOCKED = "You cannot change your stats on this map.";

	// What a speed may reach with no SpeedMax pinned (0x780720).
	constexpr int32_t SPEED_CAP = 140;

	inline bool any(const Values &values)
	{
		return std::any_of(values.begin(), values.end(), [](int32_t value) { return value != 0; });
	}

	// The mask's slots in order: bits 0 to 9 two bytes each, 10 to 12 one unsigned byte.
	template <typename ReadShort, typename ReadByte>
	Values decode(int32_t mask, ReadShort read_short, ReadByte read_byte)
	{
		Values values{};

		for (int32_t slot = 0; slot < LENGTH; slot++)
			if (mask & (1 << slot))
				values[slot] = slot < SPEED ? read_short() : static_cast<uint8_t>(read_byte());

		return values;
	}

	// STR, DEX, INT or LUK: a pinned one takes the place of the total, and a percent then counts the
	// player's own stat only (FUN_0077ec9f). Unpinned, the percent counts the whole total as before.
	inline int32_t basic(int32_t total, int32_t own, float percent, int32_t pinned)
	{
		if (pinned > 0)
			return pinned + static_cast<int32_t>(own * percent);

		return total + static_cast<int32_t>(total * percent);
	}

	// One of PAD to Jump: a pinned one takes the place of the total, then each is held to v83's bounds
	// (FUN_0077f4c9).
	inline int32_t secondary(Slot slot, int32_t total, const Values &values)
	{
		if (values[slot] > 0)
			total = values[slot];

		switch (slot)
		{
		case ACC:
		case EVA:
			return std::clamp(total, 0, 999);
		case SPEED:
			return std::clamp(total, 100, values[SPEED_MAX] > 0 ? values[SPEED_MAX] : SPEED_CAP);
		case JUMP:
			return std::clamp(total, 100, 123);
		default:
			return std::clamp(total, 0, 1999);
		}
	}
}
