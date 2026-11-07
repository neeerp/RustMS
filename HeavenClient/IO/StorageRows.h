#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace ms
{
	// The row arithmetic of v83's storage dialog (CTrunkDlg).
	namespace storage_rows
	{
		struct Entry
		{
			int32_t itemid;
			int16_t count;
			int16_t slot;
		};

		// The row a take-out changed: the first that differs from the list
		// asked for it (FUN_007c9078). A list that only got shorter gives its
		// last row, and an unchanged one its first.
		inline int16_t changed_row(const std::vector<Entry> &before, const std::vector<Entry> &after)
		{
			size_t shared = std::min(before.size(), after.size());

			for (size_t i = 0; i < shared; i++)
				if (before[i].itemid != after[i].itemid || before[i].slot != after[i].slot
					|| before[i].count != after[i].count)
					return static_cast<int16_t>(i);

			if (after.size() == before.size())
				return 0;

			return static_cast<int16_t>(after.size() < before.size() ? shared - 1 : shared);
		}

		// The top of the player's list once it shows a row. v83 allows four
		// rows below the top, one more than the list shows, so a row just
		// under the last stays out of view (0x7c8b5d).
		inline int16_t scrolled_to(int16_t row, int16_t top)
		{
			if (row < top)
				return row;

			return row > top + 4 ? row - 4 : top;
		}

		// A take-out shows the item's bag. v83 compares the tab shown, counted
		// from 0, to the item's type, counted from 1, so the tab just right of
		// the item's stays (0x7c8b41).
		inline bool switches_tab(int16_t shown, int8_t type)
		{
			return shown != type;
		}

		// How many rows the stored list scrolls over: every slot, filled or
		// not (FUN_007c7073).
		inline int16_t stored_positions(uint8_t slots, int16_t rows)
		{
			return std::max<int16_t>(slots, rows);
		}
	}
}
