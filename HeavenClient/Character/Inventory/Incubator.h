#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// The Incubator (5060002, v83 cash type 0x1b, report c035), which hatches a Pigmy Egg. Double-clicking an egg
	// opens its window with the egg in, double-clicking the incubator opens it empty; OK asks, then sends the
	// incubator with the egg's tab and slot. INCUBATOR_RESULT says what the egg held.
	namespace incubator
	{
		constexpr int32_t INCUBATOR = 5060002;

		// The items whose double-click opens the window (0x4f009e): the Pigmy Eggs and the DS Egg.
		inline bool opens(int32_t item_id)
		{
			return item_id / 10000 == 417;
		}

		// The eggs the window takes when dropped in (0x4f46d2): not the DS Egg 4170012.
		inline bool is_egg(int32_t item_id)
		{
			return item_id >= 4170000 && item_id <= 4170009;
		}

		// StringPool 0x111a, for an egg without an incubator.
		constexpr const char *NO_INCUBATOR =
				"To hatch Pigmy Egg, you need to\\npurchase the incubator from the Cash Shop.";
		// StringPool 0xd61, for anything else dropped in.
		constexpr const char *NOT_AN_EGG = "This item cannot be incubated.";
		// StringPool 0xd62, for OK with nothing in.
		constexpr const char *SELECT = "Please select the item you'd like to incubate";
		// StringPool 0xd63, the question OK asks.
		constexpr const char *ASK = "You'll need at least 1 slot of Equip. inv., 3 slots of Use inv., and 1 slot of "
				"Etc. inv. available.\\nWill you incubate the Pigmy Egg?";
		// StringPool 0xd67, for a result with no item.
		constexpr const char *FULL = "Your inventory is full.\\nPlease try again later.";

		// The popup's text for `count` of `item_id` found (0xa28298): SP 0xd68, then "#w", from which the rest shows
		// at once, the reward header (SP 0xc85), and the item's icon, name and count (SP 0xc86).
		inline std::string found(int32_t item_id, int16_t count)
		{
			std::string id = std::to_string(item_id);

			return "An item is found inside the Pigmy Egg.#w\r\n\r\n#fUI/UIWindow.img/QuestIcon/4/0#\r\n#v" + id + ":# #t"
				   + id + ":# " + std::to_string(count) + "\r\n\r\n";
		}
	}
}
