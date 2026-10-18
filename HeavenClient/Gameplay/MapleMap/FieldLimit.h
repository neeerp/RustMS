#pragma once

#include <algorithm>
#include <cstdint>
#include <map>
#include <vector>

namespace ms
{
	// What a map's fieldLimit stops the v83 client doing.
	namespace field_limit
	{
		constexpr int32_t JUMP = 0x1;
		constexpr int32_t RETURN_SCROLLS = 0x20;
		constexpr int32_t MAP_SCROLLS = 0x100;
		constexpr int32_t CONSUMABLES = 0x400;
		constexpr int32_t DOWN_JUMP = 0x20000;
		constexpr int32_t DROPS = 0x400000;

		// A return scroll's moveTo for "the nearest town": the map's return map.
		constexpr int32_t NEAREST_TOWN = 999999999;

		// The kinds v83 sends as USE_ITEM (FUN_004efd25 and the quick slots
		// at FUN_0094f2ed): potions, food, cures, transformations and the
		// like. Return scrolls, scrolls and the rest go other ways.
		inline bool is_use_item(int32_t item_id)
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
					return true;
				default:
					return false;
			}
		}

		// v83 FUN_00a092fb: with bit 0x400, only the items the map lists
		// under info/allowedItem can be used. The check covers only USE_ITEM.
		inline bool may_use(int32_t fieldlimit, const std::vector<int32_t> &allowed, int32_t item_id)
		{
			return !is_use_item(item_id) || !(fieldlimit & CONSUMABLES)
					|| std::find(allowed.begin(), allowed.end(), item_id) != allowed.end();
		}

		// v83 FUN_00a26227: whether two maps are in the same area, by the
		// first two digits of their ids in Map/AreaCode.img. Event maps
		// (the millions digit 9) and the ships (20009xxxx) are in none.
		inline bool same_area(int32_t from, int32_t to, const std::map<int32_t, int32_t> &codes)
		{
			auto area = [&codes](int32_t map_id)
			{
				if ((map_id / 1000000) % 100 == 9 || map_id / 10000 == 20009)
					return codes.end();

				return codes.find(map_id / 10000000);
			};

			auto area_from = area(from);
			auto area_to = area(to);

			return area_from != codes.end() && area_to != codes.end() && area_from->second == area_to->second;
		}

		enum class ScrollRefusal
		{
			NONE,
			// "You can't use it here in this map."
			HERE,
			// "You cannot go to that place."
			THERE
		};

		// v83 FUN_00a1e1ce: bit 0x20 stops every return scroll, bit 0x100
		// those that name a map. A scroll that names a map stays in the
		// player's area unless it ignores continents.
		inline ScrollRefusal return_scroll_refusal(int32_t fieldlimit, int32_t map_id, int32_t move_to,
				bool ignore_continent, const std::map<int32_t, int32_t> &codes)
		{
			if (fieldlimit & RETURN_SCROLLS)
				return ScrollRefusal::HERE;

			if (move_to == NEAREST_TOWN)
				return ScrollRefusal::NONE;

			if (fieldlimit & MAP_SCROLLS)
				return ScrollRefusal::HERE;

			if (!ignore_continent && !same_area(map_id, move_to, codes))
				return ScrollRefusal::THERE;

			return ScrollRefusal::NONE;
		}

		// v83 FUN_00a0900a and FUN_00a23de5: bit 0x400000 keeps items and
		// mesos off the ground.
		inline bool may_drop(int32_t fieldlimit)
		{
			return !(fieldlimit & DROPS);
		}

		// v83 FUN_0094c383 and FUN_0094c4f8: bit 0x1 stops jumping, and bit
		// 0x20000 the down-jump (Down + Jump), for all but GMs. Down + Jump
		// never falls back to a plain jump.
		inline bool may_jump(int32_t fieldlimit, bool down)
		{
			return !(fieldlimit & (down ? DOWN_JUMP : JUMP));
		}
	}
}
