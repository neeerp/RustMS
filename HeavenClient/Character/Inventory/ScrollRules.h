#pragma once

#include <cstdint>

namespace ms
{
	// Equipment scrolls (USE 204xxxx): what the v83 client checks before it
	// sends one dropped on the Equip window (FUN_004f4bb7), and the chat line
	// the scroller reads after SHOW_SCROLL_EFFECT (FUN_0093354d).
	namespace scroll_rules
	{
		constexpr int32_t WHITE_SCROLL = 2340000;

		inline bool is_scroll(int32_t item_id)
		{
			return item_id / 10000 == 204;
		}

		// FUN_004f5497. Clean Slate (20490xx) and Chaos (20491xx) fit any
		// equipment but pet equipment; 20492xx fits rings, pendants and belts;
		// any other scroll fits the category its id names (2043001: 130xxxx).
		// The [6th Anniversary] scrolls 2049105-2049110 fit nothing.
		inline bool fits(int32_t scroll_id, int32_t equip_id)
		{
			if (!is_scroll(scroll_id) || equip_id / 1000000 != 1)
				return false;

			int32_t group = scroll_id / 100;
			bool any_equip = group == 20490 || (group == 20491 && (scroll_id < 2049105 || scroll_id > 2049110));

			if (any_equip && equip_id / 100000 != 18)
				return true;

			int32_t category = equip_id / 10000 % 100;

			if (group == 20492)
				return category >= 11 && category <= 13;

			return scroll_id % 10000 / 100 == category;
		}

		// Whether to offer a White Scroll: never for Clean Slate (2049000-
		// 2049003) or the spikes and cold scrolls, which use no slot.
		inline bool takes_white_scroll(int32_t scroll_id)
		{
			return scroll_id != 2040727 && scroll_id != 2041058 && (scroll_id < 2049000 || scroll_id > 2049003);
		}

		// The scroller's chat line (StringPool 0x183-0x185, 0xf7d, 0xf7e).
		inline const char* result_text(bool success, bool cursed, bool white)
		{
			if (success)
				return white ?
					" The item was successfully upgraded, and the white scroll was used in the process." :
					"The scroll lights up, and then its mysterious power has been transferred to the item.";

			if (cursed)
				return "The item is destroyed due to the overwhelming power of the scroll.";

			return white ?
				"The item upgrade failed, but since the White Scroll was used, the number of item upgrade slots remained in tact." :
				"The scroll lights up, but the item winds up as if nothing happened";
		}
	}
}
