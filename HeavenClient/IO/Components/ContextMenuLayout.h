#pragma once

#include "../../Template/Point.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// The rows of v83's menu on someone else (FUN_009967fb) and where it
	// opens (FUN_00996570).
	namespace context_menu
	{
		// In display order. v83's button ids are 0x7d0 on from these; it has
		// a handler for a cash item trade, but no row for it.
		enum Entry : uint16_t
		{
			INFO,
			SAVE_NAME,
			TRADE,
			CASH_TRADE,
			WHISPER,
			PARTY,
			GUILD,
			CLAIM,
			FAMILY
		};

		constexpr int16_t WIDTH = 100;
		constexpr int16_t ROW = 15;
		// The top cap's height, where the first row starts.
		constexpr int16_t TOP = 23;

		// A row is left out when its condition fails, and those below move
		// up; none is ever greyed.
		inline std::vector<Entry> entries(bool party_invite, bool guild_invite, bool add_junior)
		{
			std::vector<Entry> rows = { INFO, SAVE_NAME, TRADE, WHISPER };

			if (party_invite)
				rows.push_back(PARTY);

			if (guild_invite)
				rows.push_back(GUILD);

			rows.push_back(CLAIM);

			if (add_junior)
				rows.push_back(FAMILY);

			return rows;
		}

		// The caps and the rows.
		inline int16_t height(size_t rows)
		{
			return static_cast<int16_t>((rows + 2) * ROW);
		}

		// The top-left corner sits at the cursor, moved back inside the view
		// if the menu would leave it at the right or the bottom.
		inline Point<int16_t> place(Point<int16_t> cursor, size_t rows, int16_t view_width, int16_t view_height)
		{
			int16_t x = cursor.x();
			int16_t y = cursor.y();

			if (x + WIDTH > view_width)
				x = view_width - WIDTH;

			if (y + height(rows) > view_height)
				y = view_height - height(rows);

			return Point<int16_t>(x, y);
		}
	}
}
