#pragma once

#include "../Character/Inventory/ItemExpiry.h"
#include "../Data/CurseData.h"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <cstdio>
#include <string>
#include <utility>
#include <vector>

// v83's notes (memos, report 204m): the list MEMO_RESULT (41) brings, the send
// dialog's checks, and how the inbox lays out its rows.
namespace ms::Memos
{
	// The note item (5090000, any 509xxxx sends one).
	constexpr int32_t ITEM = 5090000;
	// The longest receiver name and message the send dialog takes.
	constexpr size_t SHORTEST_NAME = 4, LONGEST_NAME = 12, LONGEST_TEXT = 200;

	// What a note is: a plain one, a gift's thank-you raising its reader's
	// fame, a gift, or a wedding invitation ("<text>_<item>").
	enum Flag : uint8_t { PLAIN, FAME, GIFT, INVITATION };

	struct Memo
	{
		int32_t id;
		std::string sender;
		std::string text;
		// A FILETIME, shown in UTC.
		int64_t sent;
		uint8_t flag;
	};

	// Why the send dialog won't send (FUN_007dc659), in the order it checks.
	enum class Refusal { NONE, NAME, SELF, CURSE, EMPTY, LONG };

	inline const char* refusal_text(Refusal refusal)
	{
		switch (refusal)
		{
			case Refusal::NAME:
				return "Please enter the name\\n of the receiving character.";
			case Refusal::SELF:
				return "You can't send the note to yourself.";
			case Refusal::CURSE:
				return "Cursing is not allowed.";
			case Refusal::EMPTY:
				return "Please enter the content.";
			case Refusal::LONG:
				// Though the dialog takes 200 bytes.
				return "The notes can be sent in 100 characters.";
			default:
				return "";
		}
	}

	inline bool same_name(const std::string& a, const std::string& b)
	{
		return a.size() == b.size() && std::equal(a.begin(), a.end(), b.begin(), [](char x, char y)
		{
			return std::tolower(static_cast<unsigned char>(x)) == std::tolower(static_cast<unsigned char>(y));
		});
	}

	// Whether a note with the trimmed `to` and `text` may go, from `own`.
	// `text` becomes what is sent, its curse words starred out by `filter`.
	template <typename Filter>
	Refusal check(const std::string& to, std::string& text, const std::string& own, Filter filter)
	{
		if (to.size() < SHORTEST_NAME || to.size() > LONGEST_NAME)
			return Refusal::NAME;

		if (same_name(to, own))
			return Refusal::SELF;

		if (!CurseData::valid(text))
			return Refusal::CURSE;

		text = filter(text);

		if (text.empty())
			return Refusal::EMPTY;

		if (text.size() > LONGEST_TEXT)
			return Refusal::LONG;

		return Refusal::NONE;
	}

	// When it was sent, as "%d.%02d.%02d %02d:%02d" filled with the month,
	// day, year, hour and minute.
	inline std::string date(int64_t filetime)
	{
		item_expiry::Time time = item_expiry::utc(filetime);
		char text[32];
		std::snprintf(text, sizeof text, "%d.%02d.%02d %02d:%02d", time.month, time.day, time.year, time.hour,
				time.minute);

		return text;
	}

	// What a row shows of the note's text: an invitation's before its "_".
	inline std::string shown_text(const Memo& memo)
	{
		if (memo.flag == INVITATION)
			return memo.text.substr(0, memo.text.find('_'));

		return memo.text;
	}

	// A row: its header, a line per line of text, its bottom and the gap
	// to the next; a thank-you has three lines more.
	constexpr int16_t TOP = 24, LINE = 18, BOTTOM = 8, GAP = 7;

	inline int16_t row_height(size_t lines, uint8_t flag)
	{
		int16_t extra = flag == FAME ? 3 * LINE : 0;

		return static_cast<int16_t>(TOP + LINE * lines + BOTTOM + GAP + extra);
	}

	// The list's 138 px view scrolls 10 px a step, over rows `total` high.
	constexpr int16_t VIEW = 138, STEP = 10;

	inline int16_t scroll_steps(int16_t total)
	{
		return std::max<int16_t>(0, total / STEP - 12);
	}

	// Lines of at most `max` px by `width`, broken after a space where one
	// fits, else between characters.
	template <typename Width>
	std::vector<std::string> wrap(const std::string& text, int16_t max, Width width)
	{
		std::vector<std::string> lines;
		size_t start = 0;

		while (start < text.size())
		{
			size_t end = start + 1;

			while (end < text.size() && width(text.substr(start, end + 1 - start)) <= max)
				end++;

			if (end < text.size())
			{
				size_t space = text.rfind(' ', end);

				if (space != std::string::npos && space >= start)
					end = space + 1;
			}

			lines.push_back(text.substr(start, end - start));
			start = end;
		}

		if (lines.empty())
			lines.emplace_back();

		return lines;
	}

	// The notes waiting, until the inbox takes them, and whether a list
	// asked for is on its way (CWvsContext+0x368c, +0x36a0, +0x36a4).
	class Inbox
	{
	public:
		static Inbox& get()
		{
			static Inbox inbox;
			return inbox;
		}

		// A list came, replacing the last. Whether to ask again for one
		// that arrived since asking.
		bool listed(std::vector<Memo> memos)
		{
			notes = std::move(memos);
			asking = false;

			return std::exchange(again, false);
		}

		// A note arrived. Whether to ask for the list now; if one is
		// already asked for, ask once more when it comes.
		bool arrived()
		{
			if (asking)
			{
				again = true;

				return false;
			}

			asking = true;

			return true;
		}

		// Asking again now.
		void asked()
		{
			asking = true;
		}

		bool empty() const
		{
			return notes.empty();
		}

		size_t size() const
		{
			return notes.size();
		}

		std::vector<Memo> take()
		{
			return std::exchange(notes, {});
		}

	private:
		std::vector<Memo> notes;
		bool asking = false, again = false;
	};
}
