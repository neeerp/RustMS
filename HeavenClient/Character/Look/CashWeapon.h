#pragma once

#include <cstdint>

namespace ms
{
	// A cash weapon (1702xxx, worn at -111) has art for each weapon type it
	// covers, under Weapon/<id>.img/<type>. v83 draws the regular weapon's
	// type from it, and no weapon when it has none (0x41272c, 0x412d63);
	// the regular weapon keeps its stances, sounds and speed.
	namespace cash_weapon
	{
		// v83 0x460aa0: id/10000 % 100 for the weapon types, else 0.
		inline int32_t type(int32_t weapon)
		{
			int32_t type = weapon / 10000 % 100;

			if (weapon / 1000000 != 1 || type < 30 || type > 49 || type > 33 && type < 37)
				return 0;

			return type;
		}

		// What v83's equip gate (0x4f1c2d) says to a weapon or a shield that
		// can't go on, or nullptr. A cash weapon needs a weapon under it that
		// it has art for (0x46d39c), and a weapon going under a cash weapon
		// needs the same; a cash shield needs a shield (0x5d4a2b reads
		// info/cash). `covers(cash, weapon)` says whether the cash weapon has
		// art for the weapon's type; `weapon`, `cash_weapon` and `shield` are
		// what's worn, 0 for nothing.
		template <typename Covers>
		const char* refusal(int16_t part, int32_t item_id, bool cash_item, int32_t weapon, int32_t cash_weapon,
							int32_t shield, Covers covers)
		{
			if (part == 11)
			{
				int32_t kind = item_id / 100000;

				if (kind == 17)
				{
					if (weapon == 0)
						return "You need to equip yourself with an item\\nto use it.";

					if (!covers(item_id, weapon))
						return "This item is not available with the weapon\\nyou're currenly using.";
				}
				else if (kind != 16 && cash_weapon / 100000 == 17 && !covers(cash_weapon, item_id))
				{
					return "You may not use the item with the\\ncash item you're currently equipped with.\\n"
						   "Please unequip that item and try again.";
				}
			}
			else if (part == 10 && cash_item && shield == 0)
			{
				return "You are able to use this only\\nwhen you are wearing a shield.";
			}

			return nullptr;
		}
	}
}
