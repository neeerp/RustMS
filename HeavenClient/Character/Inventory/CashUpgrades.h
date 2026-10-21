#pragma once

#include "ScrollRules.h"

#include <cstdint>
#include <string>

namespace ms
{
	// Vicious' Hammer (557xxxx): what the v83 client checks of an equip
	// dragged onto its window (FUN_004f44cf), and its texts.
	namespace vicious_hammer
	{
		constexpr int32_t ITEM = 5570000;
		constexpr int32_t MAX = 2;
		constexpr int32_t HORNTAIL_NECKLACE = 1122000;

		// The reasons, in the order of 354's 0x3e.
		enum class Refusal
		{
			NONE,
			NOT_UPGRADABLE,
			USED_UP,
			HORNTAIL
		};

		// Whether an equip with `slots` upgrade slots in its template could
		// ever take a hammer, which also puts the count in its tooltip
		// (0x8e93aa): not a pet's equip, nor Horntail's Necklace.
		inline bool takes(int32_t equip_id, int32_t slots)
		{
			return slots > 0 && equip_id / 100000 != 18 && equip_id != HORNTAIL_NECKLACE;
		}

		// The equip in the Equip tab or not, its template's slots, whether it
		// came from the Cash Shop, and the hammers it has had.
		inline Refusal check(bool equip_tab, int32_t equip_id, int32_t slots, bool cash, int32_t applied)
		{
			if (!equip_tab || equip_id / 1000000 != 1)
				return Refusal::NOT_UPGRADABLE;

			if (equip_id == HORNTAIL_NECKLACE)
				return Refusal::HORNTAIL;

			if (!takes(equip_id, slots) || cash)
				return Refusal::NOT_UPGRADABLE;

			if (applied >= MAX)
				return Refusal::USED_UP;

			return Refusal::NONE;
		}

		// StringPool 0x13bc, 0x13bd and 0x13bf.
		inline const char* refusal_text(Refusal refusal)
		{
			switch (refusal)
			{
				case Refusal::USED_UP:
					return "2 upgrade increases\\nhave been used already.";
				case Refusal::HORNTAIL:
					return "You can't use Vicious' Hammer\\non Horntail Necklace.";
				default:
					return "The item is not upgradable.";
			}
		}

		// 0x13be, once the equip has had `applied` hammers.
		inline std::string done_text(int32_t applied)
		{
			return "Increased available upgrade by 1. \\n" + std::to_string(MAX - applied) + " upgrades are left.";
		}

		// 0x15db.
		inline std::string error_text(int32_t code)
		{
			return "Unknown Error (" + std::to_string(code) + ").";
		}

		// The tooltip's line (0x13c0 and 0x13c1).
		inline std::string tooltip_line(int32_t applied)
		{
			return applied >= MAX ? "2(MAX)" : std::to_string(applied);
		}
	}

	// Vega's Spell (561xxxx): which scrolls its window takes (FUN_0082ce58)
	// and how it raises their rate.
	namespace vega_spell
	{
		constexpr int32_t TEN = 5610000;
		constexpr int32_t SIXTY = 5610001;

		inline bool is_vega(int32_t item_id)
		{
			return item_id / 10000 == 561;
		}

		// An equipment scroll, but neither of the two 2041211/2041212 nor
		// Chaos. It must be on Etc's VegaSpell list too.
		inline bool takes_scroll(int32_t scroll_id)
		{
			return scroll_rules::is_scroll(scroll_id) && scroll_id != 2041211 && scroll_id != 2041212 &&
				   scroll_id != 2049100;
		}

		// The rate a scroll of `rate` percent has under this spell, or 0 if
		// the spell isn't for it: 10 goes to 30, and 60 to 90.
		inline int32_t raised(int32_t item_id, int32_t rate)
		{
			if (item_id == TEN && rate == 10)
				return 30;

			if (item_id == SIXTY && rate == 60)
				return 90;

			return 0;
		}
	}
}
