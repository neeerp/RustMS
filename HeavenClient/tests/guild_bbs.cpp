#include "../Character/GuildBbs.h"
#include "../Net/Handlers/Helpers/GuildBbsParser.h"

#include <cstdint>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

struct Writer
{
	std::vector<int8_t> bytes;

	template <typename T>
	Writer& put(T value)
	{
		int8_t raw[sizeof(T)];
		std::memcpy(raw, &value, sizeof(T));
		bytes.insert(bytes.end(), raw, raw + sizeof(T));
		return *this;
	}

	Writer& string(const std::string& text)
	{
		put<int16_t>(static_cast<int16_t>(text.size()));
		bytes.insert(bytes.end(), text.begin(), text.end());
		return *this;
	}

	// A list record: id, poster, title, time, icon, replies.
	Writer& entry(int32_t id, int32_t poster, const std::string& title, int64_t time, int32_t icon, int32_t replies)
	{
		return put(id).put(poster).string(title).put(time).put(icon).put(replies);
	}
};

int main()
{
	try
	{
		using namespace ms;

		// 2026-10-21 13:05 and 2026-01-02 09:07, UTC.
		constexpr int64_t OCT_21 = 134370615000000000;
		constexpr int64_t JAN_2 = 134118184200000000;

		require(GuildBbs::pages(0) == 1, "no threads still make a page");
		require(GuildBbs::pages(1) == 1 && GuildBbs::pages(10) == 1, "ten threads to a page");
		require(GuildBbs::pages(11) == 2 && GuildBbs::pages(21) == 3, "the eleventh starts a page");
		require(GuildBbs::served_page(1, 11) == 1 && GuildBbs::served_page(0, 0) == 0, "a page that exists");
		require(GuildBbs::served_page(2, 20) == 0 && GuildBbs::served_page(1, 10) == 0,
				"past the last, the first page");
		require(GuildBbs::served_page(-1, 30) == 0, "no negative page");
		require(GuildBbs::pages(-25) == 1 && GuildBbs::pages(INT32_MIN) == 1, "a negative count makes a page");

		require(GuildBbs::row_at(30, 110, true, 3) == GuildBbs::NOTICE_ROW, "the notice's row");
		require(GuildBbs::row_at(30, 110, false, 3) == GuildBbs::NO_ROW, "no notice, no notice row");
		require(GuildBbs::row_at(30, 102, true, 3) == GuildBbs::NO_ROW
				&& GuildBbs::row_at(30, 103, true, 3) == GuildBbs::NOTICE_ROW
				&& GuildBbs::row_at(30, 133, true, 3) == GuildBbs::NOTICE_ROW
				&& GuildBbs::row_at(30, 134, true, 3) == 0, "the notice's row runs from y 103 to 133");
		require(GuildBbs::row_at(30, 140, true, 0) == GuildBbs::NO_ROW, "no rows on an empty page");
		require(GuildBbs::row_at(22, 134, false, 3) == 0 && GuildBbs::row_at(366, 164, false, 3) == 0,
				"the first row's corners");
		require(GuildBbs::row_at(30, 165, false, 3) == 1, "31 px a row");
		require(GuildBbs::row_at(30, 134 + 31 * 3, false, 3) == GuildBbs::NO_ROW, "nothing past the last row");
		require(GuildBbs::row_at(21, 140, false, 3) == GuildBbs::NO_ROW
				&& GuildBbs::row_at(367, 140, false, 3) == GuildBbs::NO_ROW, "rows run from x 22 to 366");

		require(GuildBbs::may_moderate(1) && GuildBbs::may_moderate(2), "masters and Jr. Masters moderate");
		require(!GuildBbs::may_moderate(3) && !GuildBbs::may_moderate(0), "members and outsiders don't");
		require(GuildBbs::may_edit(5, 7, 7), "posters edit their own");
		require(!GuildBbs::may_edit(3, 7, 8) && GuildBbs::may_edit(2, 7, 8), "others need a moderator's rank");

		require(GuildBbs::reply_start(0) == 0 && GuildBbs::reply_start(4) == 0, "four replies fit");
		require(GuildBbs::reply_start(6) == 2, "the view starts at the last four");

		require(GuildBbs::row_text(12, "Hi") == "12 Hi", "a row is its id and title");
		require(GuildBbs::row_text(0, "Rules") == " Rules", "the notice shows no id");
		require(GuildBbs::count_text(3) == "[3]" && GuildBbs::count_text(0).empty(), "the reply count");

		require(GuildBbs::full_date(OCT_21) == "2026/10/21 13:05", "the thread's date");
		require(GuildBbs::full_date(JAN_2) == "2026/01/02 09:07", "the month, day and hour are padded");
		require(GuildBbs::short_date(OCT_21, { 2026, 10, 21, 23, 0 }) == "13:05", "today's time");
		require(GuildBbs::short_date(JAN_2, { 2026, 1, 3, 0, 0 }) == "01/02", "another day's month and day");
		require(GuildBbs::short_date(JAN_2, { 2027, 1, 2, 0, 0 }) == "01/02", "the same day of another year");
		require(GuildBbs::short_date(134117858400000000, { 2026, 1, 2, 0, 0 }) == "0:04", "the hour is not padded");

		auto width = [](const std::string& text) { return static_cast<int16_t>(text.size() * 6); };
		std::vector<std::string> lines = GuildBbs::body_lines("one\r\ntwo\nthree\rfour", 240, width);
		require(lines == std::vector<std::string>({ "one", "two", "three", "four" }), "every kind of line break");
		lines = GuildBbs::body_lines("aaaa bbbb", 30, width);
		require(lines == std::vector<std::string>({ "aaaa ", "bbbb" }), "long lines wrap after a space");
		require(GuildBbs::body_lines("a\n\nb", 240, width).size() == 3, "blank lines stay");
		require(GuildBbs::body_lines("a\r\n", 240, width) == std::vector<std::string>({ "a", "" }),
				"a closing line break leaves a blank line");

		{
			using text_lines::Line;

			auto width = [](const std::string& text) { return static_cast<int16_t>(text.size() * 6); };
			std::string text = "ab\r\ncccc dddd\n";
			std::vector<Line> lines = text_lines::layout(text, 30, width);

			require(lines.size() == 4, "a break, a wrap and a closing break make four lines");
			require(lines[0].start == 0 && lines[0].length == 2, "a line stops before its break");
			require(lines[1].start == 4 && lines[1].length == 5 && lines[2].start == 9 && lines[2].length == 4,
					"a wrapped line ends where the next starts");
			require(lines[3].start == 14 && lines[3].length == 0, "after a closing break, an empty line");
			require(text_lines::layout("", 30, width).size() == 1, "no text is one line");

			require(text_lines::line_of(lines, 0) == 0 && text_lines::line_of(lines, 2) == 0, "the first line");
			require(text_lines::line_of(lines, 9) == 2, "a wrap's end is the next line's start");
			require(text_lines::line_of(lines, 14) == 3, "the end of the text");

			require(text_lines::step_left(text, 4) == 2 && text_lines::step_right(text, 2) == 4,
					"\\r\\n is one step");
			require(text_lines::step_left(text, 3) == 2 && text_lines::step_right(text, 13) == 14, "a lone character");
			require(text_lines::step_left(text, 0) == 0 && text_lines::step_right(text, 14) == 14, "the ends stay");

			std::vector<Line> lone = text_lines::layout("a\rb\r", 30, width);
			require(lone.size() == 3 && lone[1].start == 2 && lone[2].start == 4, "a lone \\r breaks the line");
			require(text_lines::step_left("a\rb", 2) == 1, "a lone \\r is one step");

			require(text_lines::scroll_to(0, 15, 16, 15) == 1, "a 16th line scrolls one down");
			require(text_lines::scroll_to(5, 15, 30, 2) == 2, "a line above scrolls up to it");
			require(text_lines::scroll_to(5, 15, 30, 10) == 5, "a line shown scrolls nothing");
			require(text_lines::scroll_to(5, 15, 17, 16) == 2, "no empty rows after deleting lines");
			require(text_lines::scroll_to(3, 15, 4, 0) == 0, "fewer lines than rows show from the first");

			require(text_lines::scroll_x(0, 100, 80, 120, 12) == 0, "a caret shown scrolls nothing");
			require(text_lines::scroll_x(0, 100, 99, 120, 12) == 0, "a caret on the right edge is shown");
			require(text_lines::scroll_x(0, 100, 110, 110, 12) == 11, "typing at the end puts the caret on the edge");
			require(text_lines::scroll_x(0, 100, 110, 150, 12) == 23, "text after the caret shows a margin");
			require(text_lines::scroll_x(40, 100, 30, 150, 12) == 18, "going left scrolls the margin back in");
			require(text_lines::scroll_x(40, 100, 5, 150, 12) == 0, "but not before the start");
			require(text_lines::scroll_x(40, 100, 40, 150, 12) == 40, "a caret on the left edge is shown");
			require(text_lines::scroll_x(60, 100, 120, 120, 12) == 60, "deleting at the end leaves the scroll");
			require(text_lines::scroll_x(0, 10, 30, 100, 48) == 21, "a narrow view keeps the caret on its edge");
		}

		{
			auto stars = [](const std::string& text) { return text == "darn" ? std::string("****") : text; };

			auto thread = [&](std::string title, std::string body) { return GuildBbs::check_thread(title, body, stars); };
			auto reply = [&](std::string text) { return GuildBbs::check_reply(text, stars); };

			require(thread("", "") == GuildBbs::Refusal::NO_TITLE, "the title first");
			require(thread("Hi", "") == GuildBbs::Refusal::NO_TEXT, "then the text");
			require(thread("a\tb", "x") == GuildBbs::Refusal::CURSING
					&& thread("x", "a\x01") == GuildBbs::Refusal::CURSING, "text the filter won't take, in either");
			require(thread("Hi", "line\rnext") == GuildBbs::Refusal::NONE, "a thread to send, line breaks and all");
			require(reply("") == GuildBbs::Refusal::NO_TEXT && reply("a\tb") == GuildBbs::Refusal::CURSING
					&& reply("ok") == GuildBbs::Refusal::NONE, "a reply's checks");

			std::string title = "darn", body = "darn";
			require(GuildBbs::check_thread(title, body, stars) == GuildBbs::Refusal::NONE && title == "****"
					&& body == "****", "curse words are starred out in both, not refused (0x8166f6)");
			std::string text = "darn";
			require(GuildBbs::check_reply(text, stars) == GuildBbs::Refusal::NONE && text == "****",
					"and in a reply (0x816994)");
		}

		{
			require(GuildBbs::icon_of_item(5290000) == 100 && GuildBbs::icon_of_item(5290006) == 106,
					"the cash icons' items");

			require(GuildBbs::slot_at(426, 246, 0) == 0 && GuildBbs::slot_at(445, 265, 0) == 0, "the first slot");
			require(GuildBbs::slot_at(446, 250, 0) == GuildBbs::NO_ROW, "a gap between slots");
			require(GuildBbs::slot_at(449, 250, 0) == 1 && GuildBbs::slot_at(472, 250, 0) == 2, "23 px a slot");
			require(GuildBbs::slot_at(495, 250, 0) == GuildBbs::NO_ROW && GuildBbs::slot_at(495, 250, 1) == 3,
					"a cash slot only for an icon owned");
			require(GuildBbs::slot_at(430, 245, 2) == GuildBbs::NO_ROW
					&& GuildBbs::slot_at(430, 266, 2) == GuildBbs::NO_ROW
					&& GuildBbs::slot_at(425, 250, 2) == GuildBbs::NO_ROW, "slots run from y 246 to 265");

			std::vector<int32_t> owned = { 102, 105 };
			require(GuildBbs::slot_icon(1, owned) == 1 && GuildBbs::slot_icon(4, owned) == 105,
					"a slot's icon: free, then owned");
			require(GuildBbs::slot_of(2, owned) == 2 && GuildBbs::slot_of(105, owned) == 4,
					"an edit starts on a free or owned icon");
			require(GuildBbs::slot_of(103, owned) == GuildBbs::NO_ROW && GuildBbs::slot_of(-1, owned) == GuildBbs::NO_ROW,
					"but not on one the palette lacks");
		}

		{
			Writer w;
			w.put<uint8_t>(0).put<int32_t>(12).put<int32_t>(2);
			w.entry(12, 7, "Hello", OCT_21, 1, 0).entry(11, 8, "Older", JAN_2, 101, 3);

			InPacket recv(w.bytes.data(), w.bytes.size());
			GuildBbs::List list = GuildBbsParser::parse_list(recv);

			require(!recv.available(), "a list is read to its end");
			require(!list.has_notice && list.total == 12 && list.entries.size() == 2, "no notice and a page");
			require(list.entries[1].id == 11 && list.entries[1].poster == 8 && list.entries[1].title == "Older",
					"a record's id, poster and title");
			require(list.entries[1].time == JAN_2 && list.entries[1].icon == 101 && list.entries[1].replies == 3,
					"a record's time, icon and replies");
		}

		{
			Writer w;
			w.put<uint8_t>(1).entry(0, 7, "Rules", OCT_21, 2, 1).put<int32_t>(0);

			InPacket recv(w.bytes.data(), w.bytes.size());
			GuildBbs::List list = GuildBbsParser::parse_list(recv);

			require(!recv.available(), "no page count follows no threads");
			require(list.has_notice && list.notice.title == "Rules" && list.notice.replies == 1, "the notice");
			require(list.total == 0 && list.entries.empty(), "no other threads");
		}

		{
			// The usual case: a notice and a page.
			Writer w;
			w.put<uint8_t>(1).entry(0, 7, "Rules", OCT_21, 2, 1).put<int32_t>(1).put<int32_t>(1);
			w.entry(1, 8, "First", JAN_2, 0, 0);

			InPacket recv(w.bytes.data(), w.bytes.size());
			GuildBbs::List list = GuildBbsParser::parse_list(recv);

			require(!recv.available(), "a notice and a page are read to their end");
			require(list.has_notice && list.notice.id == 0 && list.total == 1 && list.entries.size() == 1
					&& list.entries[0].title == "First", "the notice and the page");
		}

		{
			// A page past the last.
			Writer w;
			w.put<uint8_t>(0).put<int32_t>(3).put<int32_t>(0);

			InPacket recv(w.bytes.data(), w.bytes.size());
			GuildBbs::List list = GuildBbsParser::parse_list(recv);

			require(!recv.available() && list.total == 3 && list.entries.empty(), "an empty page");
		}

		{
			Writer w;
			w.put<int32_t>(5).put<int32_t>(7).put(OCT_21).string("Title").string("Body\r\ntext").put<int32_t>(0);
			w.put<int32_t>(2);
			w.put<int32_t>(1).put<int32_t>(8).put(JAN_2).string("first");
			w.put<int32_t>(4).put<int32_t>(7).put(OCT_21).string("second");

			InPacket recv(w.bytes.data(), w.bytes.size());
			GuildBbs::Thread thread = GuildBbsParser::parse_thread(recv);

			require(!recv.available(), "a thread is read to its end");
			require(thread.id == 5 && thread.poster == 7 && thread.time == OCT_21, "the thread's id, poster and time");
			require(thread.title == "Title" && thread.body == "Body\r\ntext" && thread.icon == 0, "its text and icon");
			require(thread.replies.size() == 2, "two replies");
			require(thread.replies[0].id == 1 && thread.replies[0].poster == 8 && thread.replies[0].time == JAN_2
					&& thread.replies[0].text == "first", "replies keep their order");
			require(thread.replies[1].id == 4 && thread.replies[1].text == "second", "the second reply");
		}

		{
			Writer w;
			w.put<int32_t>(5).put<int32_t>(7).put(OCT_21).string("Title").string("").put<int32_t>(100)
					.put<int32_t>(0);

			InPacket recv(w.bytes.data(), w.bytes.size());
			GuildBbs::Thread thread = GuildBbsParser::parse_thread(recv);

			require(!recv.available() && thread.replies.empty() && thread.icon == 100, "no replies");
		}

		{
			// A page claiming more records than it holds.
			Writer w;
			w.put<uint8_t>(0).put<int32_t>(12).put<int32_t>(5).entry(12, 7, "Hello", OCT_21, 1, 0);

			InPacket recv(w.bytes.data(), w.bytes.size());
			bool threw = false;

			try
			{
				GuildBbsParser::parse_list(recv);
			}
			catch (const PacketError&)
			{
				threw = true;
			}

			require(threw, "a short list stops at its end");
		}

		std::cout << "guild bbs tests passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << "\n";
		return 1;
	}
}
