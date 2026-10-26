#pragma once

#include "Guild.h"

#include <cstdint>
#include <string>

// The guild ranking window that the Honorable Rock and the Guild Rank Board open (v83 FUN_005958c0, report b181).
namespace ms::guild_rank
{
	// The rows after the first, shown six to a page under the leader, who is on every page.
	constexpr int32_t PAGE_ROWS = 6;

	// The rank at the top of page `page` (from 1): 2, 8, 14 ...
	inline int32_t first_rank(int32_t page)
	{
		return PAGE_ROWS * page - 4;
	}

	// The last page with a guild on it; page 1 for one guild or none.
	inline int32_t last_page(int32_t count)
	{
		int32_t page = 1;

		while (page * PAGE_ROWS + 2 <= count)
			page++;

		return page;
	}

	// BtLeft is enabled away from the first page, and BtRight while a later rank exists (FUN_00596183).
	inline bool can_go_left(int32_t page)
	{
		return page != 1;
	}

	inline bool can_go_right(int32_t page, int32_t count)
	{
		return page * PAGE_ROWS + 2 <= count;
	}

	// "0%d" below 10, else "%d": 01 to 09, then 10, 100.
	inline std::string rank_text(int32_t rank)
	{
		return (rank < 10 ? "0" : "") + std::to_string(rank);
	}

	// The leader's row is opaque, the next two 180 and the rest 120, out of 255.
	inline uint8_t alpha(int32_t rank)
	{
		if (rank == 1)
			return 255;

		return rank > 3 ? 120 : 180;
	}

	// A row's emblem is drawn only if one of its four ids is set, and then a zero id takes the viewer's own guild's
	// value (FUN_00a3956e).
	inline bool has_emblem(const GuildEmblem &emblem)
	{
		return emblem.background || emblem.background_color || emblem.mark || emblem.mark_color;
	}

	inline GuildEmblem shown_emblem(GuildEmblem emblem, const GuildEmblem &own)
	{
		if (!emblem.background)
			emblem.background = own.background;

		if (!emblem.background_color)
			emblem.background_color = own.background_color;

		if (!emblem.mark)
			emblem.mark = own.mark;

		if (!emblem.mark_color)
			emblem.mark_color = own.mark_color;

		return emblem;
	}
}
