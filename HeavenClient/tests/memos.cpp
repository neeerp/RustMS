#include "../Util/Memos.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::Memos;

		auto clean = [](const std::string& text) { return text; };
		auto mask = [](const std::string& text)
		{
			std::string masked = text;
			size_t pos = masked.find("bad");

			if (pos != std::string::npos)
				masked.replace(pos, 3, "***");

			return masked;
		};

		require(check("Bob", "hi", "Me", clean) == Refusal::NAME, "a name under 4 is refused");
		require(check("Abcdefghijklm", "hi", "Me", clean) == Refusal::NAME, "a name over 12 is refused");
		require(check("mYsElF", "hi", "Myself", clean) == Refusal::SELF, "not to oneself, ignoring case");
		require(check("Alice", "a bad word", "Me", mask) == Refusal::CURSE, "curses are refused");
		require(check("Alice", "", "Me", clean) == Refusal::EMPTY, "an empty note is refused");
		require(check("Alice", std::string(201, 'a'), "Me", clean) == Refusal::LONG, "200 bytes at most");
		require(check("Alice", std::string(200, 'a'), "Me", clean) == Refusal::NONE, "200 bytes go");
		require(check("Alice", "", "Alice", clean) == Refusal::SELF, "the name is checked before the text");

		// 2026-10-20 14:05:09 UTC.
		int64_t when = ms::item_expiry::UNIX_EPOCH + 1792505109LL * ms::item_expiry::TICKS_PER_SECOND;
		require(date(when) == "10.20.2026 14:05", "the date is month, day, year, then the time");

		require(shown_text({ 1, "A", "Join us_4031377", 0, INVITATION }) == "Join us", "an invitation hides its item");
		require(shown_text({ 1, "A", "a_b", 0, PLAIN }) == "a_b", "a plain note shows all");

		require(row_height(1, PLAIN) == 57, "a one-line row");
		require(row_height(2, FAME) == 129, "a thank-you has three lines more");
		require(row_height(1, GIFT) == 57, "a gift's lines take no room of their own");

		require(scroll_steps(57) == 0, "a short list doesn't scroll");
		require(scroll_steps(300) == 18, "steps of 10 px past the first 120");

		auto width = [](const std::string& text) { return static_cast<int16_t>(text.size() * 10); };

		require(wrap("hello", 244, width) == std::vector<std::string>{ "hello" }, "a short text is one line");
		require(wrap("", 244, width) == std::vector<std::string>{ "" }, "an empty text is one line");
		require(wrap("aaaa bbbb", 60, width) == std::vector<std::string>{ "aaaa ", "bbbb" }, "a line breaks after a space");
		require(wrap("aaaaaaaa", 50, width) == std::vector<std::string>{ "aaaaa", "aaa" }, "a word too long is split");

		Inbox inbox;
		require(inbox.arrived(), "the first arrival asks");
		require(!inbox.arrived(), "another waits for the answer");
		require(inbox.listed({ { 1, "A", "hi", 0, PLAIN } }), "and asks again once it comes");
		inbox.asked();
		require(!inbox.listed({ { 1, "A", "hi", 0, PLAIN }, { 2, "B", "yo", 0, FAME } }), "then stops asking");
		require(inbox.size() == 2, "a list replaces the last");
		require(inbox.take().size() == 2 && inbox.empty(), "the inbox takes the notes");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
