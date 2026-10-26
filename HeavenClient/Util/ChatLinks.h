#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>

namespace ms
{
	// How v83 links an item to a chat log line (BROADCAST_MSG types 6, 8 and
	// 11): every row of the line carries the item, and a click on one shows
	// it (FUN_008d5f5c).
	namespace chat_link
	{
		// A row is hot left of this, the log's whole width.
		constexpr int16_t HOT_WIDTH = 0x244;
		// The tooltip goes 20 below the click (FUN_008f5b20)...
		constexpr int16_t BELOW = 0x14;
		// ...and stays while the cursor is in the 40x40 square around the
		// click, the right and bottom edges left out (PtInRect, 0x8d804e).
		// `dx` and `dy` are how far the cursor is from the click.
		constexpr int16_t KEEP = 0x14;

		inline bool keeps(int16_t dx, int16_t dy)
		{
			return dx >= -KEEP && dx < KEEP && dy >= -KEEP && dy < KEEP;
		}

		// How much of the part from `from`, `length` long, a row of
		// `row_size` shows; only the first row draws it violet.
		inline size_t shown(size_t row_size, size_t from, size_t length)
		{
			return from >= row_size ? 0 : std::min(length, row_size - from);
		}
	}
}
