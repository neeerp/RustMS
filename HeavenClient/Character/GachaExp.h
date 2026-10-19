#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// A Writ of Solomon (237xxxx) gives gacha EXP, which the player takes
	// in by double-clicking the EXP numbers on the status bar.
	namespace gacha_exp
	{
		// The highest level that may take it in (FUN_008d7611 asks below 51).
		constexpr int16_t MAX_LEVEL = 50;

		enum class WritRefusal
		{
			NONE,
			// Above the writ's info/maxLevel: chat 0xc69
			LEVEL,
			// Some gacha EXP is left: chat 0x12fc
			EXP_LEFT
		};

		// Why v83 won't use a writ, in the order it checks (FUN_00a12685).
		// `max_level` is 0 for none.
		inline WritRefusal writ_refusal(int16_t level, int32_t max_level, int32_t gacha)
		{
			if (max_level > 0 && level > max_level)
				return WritRefusal::LEVEL;

			if (gacha != 0)
				return WritRefusal::EXP_LEFT;

			return WritRefusal::NONE;
		}

		// Whether double-clicking the EXP numbers asks to take it in.
		inline bool may_take(int16_t level, int32_t gacha)
		{
			return level <= MAX_LEVEL && gacha > 0;
		}

		// What hovering the EXP numbers shows (FUN_008d76d7, StringPool
		// 0x15a1 without gacha EXP).
		inline std::string tooltip(int64_t exp, int64_t need, int32_t gacha)
		{
			std::string text = "EXP : " + std::to_string(exp) + " / " + std::to_string(need);

			if (gacha > 0)
				text += "  GachaponEXP : " + std::to_string(gacha);

			return text;
		}
	}
}
