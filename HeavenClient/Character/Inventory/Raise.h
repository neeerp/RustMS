#pragma once

#include <algorithm>
#include <cstdint>
#include <string>
#include <utility>

namespace ms
{
	// The raise window (v83 CUIRaise, report b233): an Etc 4220xxx item whose info names a quest record
	// (`questId`), art (`uiData`), `exp` per stage and `grade` stages, as the Timer's Egg of "A Cute Baby Bird".
	// Double-clicking it opens the window, which asks for the record (0xEC); the items its `consumeItem` lists are
	// dropped on it to feed it (0xEE), and the record, sent back as a quest record message, grows it.
	namespace raise
	{
		// The items whose double-click opens the window (0x4f0024).
		inline bool opens(int32_t item_id)
		{
			return item_id / 1000 == 4220;
		}

		// StringPool 0x110a, for an item it doesn't take.
		constexpr const char *NOT_AVAILABLE = "This item is not available.";

		// The growth of an item that grows by `exp` (v83's exp mode, ctor 0x8926d8).
		struct Growth
		{
			int32_t exp;
			int32_t grade;

			// Whether the window opens (0x8932b8): an item with no exp grows by collection (ctor 0x892ac1),
			// which no quest here uses and this client doesn't do; one with exp but no grade doesn't open.
			bool valid() const
			{
				return exp > 0 && grade > 0;
			}

			// The value before the first record comes (0x8926d8), stage -1.
			int32_t unknown() const
			{
				return -exp;
			}

			// grade * exp (0x892878).
			int32_t max() const
			{
				return grade * exp;
			}

			// The art and message shown for `value`, -1 for none (0x892a61).
			int32_t stage(int32_t value) const
			{
				return std::min(value / exp, grade);
			}

			// Which of a fed item's messages shows when it is dropped on the window (0x892878): 1 while it may
			// still grow, in the chat log, and the item is sent; 0 when grown, as a notice, and nothing is sent.
			int32_t feed_message(int32_t value) const
			{
				return value < max() ? 1 : 0;
			}
		};

		// The file and path of `uiData`, as "UI/UIWindow/raise/3": the UI file's image and the rest. Empty if it
		// isn't a UI path.
		inline std::pair<std::string, std::string> ui_path(const std::string &data)
		{
			const std::string prefix = "UI/";

			if (data.compare(0, prefix.size(), prefix) != 0)
				return {};

			size_t slash = data.find('/', prefix.size());

			if (slash == std::string::npos || slash + 1 == data.size())
				return {};

			return {data.substr(prefix.size(), slash - prefix.size()) + ".img", data.substr(slash + 1)};
		}
	}
}
