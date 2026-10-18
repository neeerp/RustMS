#pragma once

#include <algorithm>
#include <cstdint>
#include <vector>

namespace ms
{
	// What a map's fieldLimit stops the v83 client doing with items.
	namespace field_limit
	{
		constexpr int32_t CONSUMABLES = 0x400;
		constexpr int32_t DROPS = 0x400000;

		// v83 FUN_00a092fb: with bit 0x400, only the items the map lists
		// under info/allowedItem can be used. The check covers the kinds
		// that path sends (potions, food, cures, transformations and the
		// like); return scrolls, scrolls and the rest go other ways.
		inline bool may_use(int32_t fieldlimit, const std::vector<int32_t> &allowed, int32_t item_id)
		{
			switch (item_id / 10000)
			{
				case 200:
				case 201:
				case 202:
				case 205:
				case 221:
				case 236:
				case 238:
				case 245:
					break;
				default:
					return true;
			}

			return !(fieldlimit & CONSUMABLES) || std::find(allowed.begin(), allowed.end(), item_id) != allowed.end();
		}

		// v83 FUN_00a0900a and FUN_00a23de5: bit 0x400000 keeps items and
		// mesos off the ground.
		inline bool may_drop(int32_t fieldlimit)
		{
			return !(fieldlimit & DROPS);
		}
	}
}
