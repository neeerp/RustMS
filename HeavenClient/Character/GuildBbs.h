#pragma once

#include "Inventory/ItemExpiry.h"

#include "../Util/Memos.h"

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

// The guild bulletin board's rules (v83 CUIGuildBBS, ctor 0x812fb4; spec
// v83ref/scratch/251/bbs.md).
namespace ms::GuildBbs
{
	// A thread as the list shows it: the notice, or a row.
	struct Entry
	{
		int32_t id = 0;
		int32_t poster = 0;
		std::string title;
		// A FILETIME.
		int64_t time = 0;
		int32_t icon = 0;
		int32_t replies = 0;
	};

	// GUILD_BBS mode 6: the notice, if any, and a page of other threads.
	struct List
	{
		bool has_notice = false;
		Entry notice;
		// The threads but the notice, on every page.
		int32_t total = 0;
		std::vector<Entry> entries;
	};

	struct Reply
	{
		int32_t id = 0;
		int32_t poster = 0;
		int64_t time = 0;
		std::string text;
	};

	// GUILD_BBS mode 7: a thread and its replies, oldest first.
	struct Thread
	{
		int32_t id = 0;
		int32_t poster = 0;
		int64_t time = 0;
		std::string title;
		std::string body;
		int32_t icon = 0;
		std::vector<Reply> replies;
	};

	constexpr int32_t PAGE_SIZE = 10;
	// The thread view shows four replies at a time.
	constexpr size_t REPLY_ROWS = 4;

	// The pages the selector offers: at least one, even for no threads
	// (the original's signed division gives (-1)/10 + 1). A negative count
	// also gives one.
	inline int32_t pages(int32_t total)
	{
		return total > 0 ? (total - 1) / PAGE_SIZE + 1 : 1;
	}

	// The page a list shows: the one asked for, or the first when it is past
	// the last, as the server answers then.
	inline int32_t served_page(int32_t page, int32_t total)
	{
		return page >= 0 && static_cast<int64_t>(page) * PAGE_SIZE < total ? page : 0;
	}

	// The list's rows, in window coordinates: the notice's at y 103, the
	// others' from 134, 31 px each, all from x 22 to 366 (0x81621c).
	constexpr int16_t ROW_LEFT = 22, ROW_RIGHT = 366, NOTICE_TOP = 103, ROWS_TOP = 134, ROW_HEIGHT = 31;

	constexpr int32_t NO_ROW = -1;
	constexpr int32_t NOTICE_ROW = -2;

	// The row under a point: NOTICE_ROW, a row's index, or NO_ROW. The
	// notice's row counts only when there is a notice.
	inline int32_t row_at(int16_t x, int16_t y, bool has_notice, size_t rows)
	{
		if (x < ROW_LEFT || x > ROW_RIGHT)
			return NO_ROW;

		if (has_notice && y >= NOTICE_TOP && y < ROWS_TOP)
			return NOTICE_ROW;

		if (y < ROWS_TOP)
			return NO_ROW;

		size_t row = static_cast<size_t>((y - ROWS_TOP) / ROW_HEIGHT);

		return row < rows ? static_cast<int32_t>(row) : NO_ROW;
	}

	// Masters and Jr. Masters write the notice and moderate (0x813a70).
	inline bool may_moderate(int32_t rank)
	{
		return rank >= 1 && rank <= 2;
	}

	// Who may edit or delete a thread or a reply: its poster and the
	// moderators.
	inline bool may_edit(int32_t rank, int32_t poster, int32_t self)
	{
		return may_moderate(rank) || poster == self;
	}

	// The first reply shown when a thread arrives: the view starts at the
	// last four.
	inline size_t reply_start(size_t replies)
	{
		return replies > REPLY_ROWS ? replies - REPLY_ROWS : 0;
	}

	// A row's text: the id, a space, and the title. The notice has id 0 and
	// shows no number (0x8159d8).
	inline std::string row_text(int32_t id, const std::string& title)
	{
		return (id != 0 ? std::to_string(id) : "") + " " + title;
	}

	// The reply count after a row's title, or nothing for none.
	inline std::string count_text(int32_t replies)
	{
		return replies != 0 ? "[" + std::to_string(replies) + "]" : "";
	}

	// The thread view's date (0x817329): the FILETIME as it is, without
	// converting it to local time.
	inline std::string full_date(int64_t filetime)
	{
		item_expiry::Time t = item_expiry::utc(filetime);
		char text[32];
		std::snprintf(text, sizeof(text), "%d/%02d/%02d %02d:%02d", t.year, t.month, t.day, t.hour, t.minute);

		return text;
	}

	// The list's and replies' dates (0x817369): the time on the day it is
	// `today`, else the month and day.
	inline std::string short_date(int64_t filetime, const item_expiry::Time& today)
	{
		item_expiry::Time t = item_expiry::utc(filetime);
		char text[16];

		if (t.year == today.year && t.month == today.month && t.day == today.day)
			std::snprintf(text, sizeof(text), "%d:%02d", t.hour, t.minute);
		else
			std::snprintf(text, sizeof(text), "%02d/%02d", t.month, t.day);

		return text;
	}

	// A body's lines of at most `max` px by `width`: its own line breaks,
	// then wrapped.
	template <typename Width>
	std::vector<std::string> body_lines(const std::string& body, int16_t max, Width width)
	{
		std::vector<std::string> lines;
		size_t start = 0;

		while (true)
		{
			size_t end = body.find_first_of("\r\n", start);
			std::string paragraph = body.substr(start, end == std::string::npos ? std::string::npos : end - start);

			for (std::string& line : Memos::wrap(paragraph, max, width))
				lines.push_back(std::move(line));

			if (end == std::string::npos)
				break;

			start = end + (body.compare(end, 2, "\r\n") == 0 ? 2 : 1);
		}

		return lines;
	}
}
