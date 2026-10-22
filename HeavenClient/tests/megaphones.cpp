#include "../Util/Megaphones.h"
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
		using namespace ms::Megaphones;

		require(heard_on(100000000), "megaphones are heard in Henesys");
		require(!heard_on(109010000), "but not on an event map");

		require(with_item("Hero : selling", "Ilbi") == "Hero : [Ilbi]selling", "the item goes before the text");
		require(with_item("selling", "Ilbi") == "[Ilbi]selling", "even without a name");

		require(under_name("Hero : one", "Hero : two") == "Hero : two", "a later line keeps the name");
		require(under_name("Hero : one", "Other : two : three") == "Hero : two : three",
				"a later line takes the first line's name");
		require(under_name("Hero : one", "two") == "two", "a line without a name stays as it is");

		require(kind(5071000) == Kind::PLAIN && kind(5078000) == Kind::PLAIN, "kinds 1 and 8 are plain");
		require(kind(5072000) == Kind::SUPER, "kind 2 is the super megaphone");
		require(kind(5076000) == Kind::ITEM && kind(5075006) == Kind::ITEM, "kind 6, and 5 from 6, show an item");
		require(kind(5075000) == Kind::TV && kind(5075003) == Kind::TV && kind(5075005) == Kind::TV, "5075000 to 5075005 are Maple TV");
		require(kind(5075010) == Kind::NONE, "but not 5075010");
		require(kind(5077000) == Kind::TRIPLE, "kind 7 is the triple megaphone");
		require(kind(5073000) == Kind::NONE && kind(5370000) == Kind::NONE, "nothing else is asked for");

		auto keep = [](const std::string& text) { return text; };
		auto star = [](const std::string& text) { return std::string(text.size(), '*'); };
		const std::string long_line(61, 'a');
		auto line_check = [](std::string text, uint16_t level, auto censor) { return check_line(text, level, censor); };
		auto avatar_check = [](AvatarLines lines, auto censor) { return check_avatar_lines(lines, censor); };

		require(line_check("", 5, star) == Refusal::QUIET, "an empty line goes before anything else");
		require(line_check("h\ti", 5, keep) == Refusal::CURSING, "then text the filter won't take");
		std::string starred = "hi";
		require(check_line(starred, 11, star) == Refusal::NONE && starred == "**", "curse words are starred out");
		require(line_check("hi", 10, keep) == Refusal::LEVEL, "then level 10 and below");
		require(line_check(long_line, 11, keep) == Refusal::LONG, "then over 60");
		require(line_check(std::string(81, 'a'), 11, keep) == Refusal::QUIET, "and over 80 says nothing");
		require(line_check(std::string(60, 'a'), 11, keep) == Refusal::NONE, "60 at level 11 goes");

		require(check_lines({ "", "", "" }) == Refusal::QUIET, "a triple needs a line");
		require(check_lines({ "", long_line, "" }) == Refusal::LONG, "of 60 at most");
		require(check_lines({ "", "", "c" }) == Refusal::NONE, "in any field");

		using Lines = std::vector<std::string>;
		require(lines_sent({ "a", "", "" }) == Lines{ "a" }, "blank lines after the last go unsent");
		require(lines_sent({ "", "b", "" }) == Lines{ "", "b" }, "blank lines before it are sent");
		require(lines_sent({ "a", "", "c" }) == Lines{ "a", "", "c" }, "between too");
		require(lines_sent({ "", "", "" }) == Lines{ "" }, "and one at least");

		require(sample_line("[***]  M A P") == "[***]  M A P", "a sample line stays as it is");
		require(sample_line("\xe3\x80\x80[^^^]") == "?[^^^]", "but for what the code page lacks");

		require(kind(5390000) == Kind::AVATAR && kind(5390006) == Kind::AVATAR, "539 is the avatar megaphone");

		using Rows = AvatarLines;
		require(avatar_lines("", {}) == Rows{}, "an empty box sends four empty lines");
		require(avatar_lines("hello", { 0 }) == Rows{ "hello", "", "", "" }, "one row is the first line");
		require(avatar_lines("hello world", { 0, 5 }) == Rows{ "hello ", "world", "", "" },
				"the space a row wraps at stays on the row before");
		require(avatar_lines("abcdefgh", { 0, 3, 6 }) == Rows{ "abc", "def", "gh", "" },
				"a word too long is cut where it wraps");
		require(avatar_lines("a b c d e", { 0, 1, 3, 5, 7 }) == Rows{ "a ", "b ", "c ", "d " },
				"a fifth row is left out");

		require(avatar_check(Rows{}, keep) == Refusal::EMPTY, "an avatar megaphone needs text");
		require(avatar_check(Rows{ "", "", "", "h\ti" }, keep) == Refusal::CURSING, "text the filter takes");
		Rows censored{ "hi", "", "", "yo" };
		require(check_avatar_lines(censored, star) == Refusal::NONE && censored == Rows{ "**", "", "", "**" },
				"each line's curse words starred out");
		auto bad = [](std::string text)
		{
			for (size_t at = text.find("bad"); at != std::string::npos; at = text.find("bad", at))
				text.replace(at, 3, "***");

			return text;
		};
		Rows split{ "a ba", "d day", "", "" };
		require(check_avatar_lines(split, bad) == Refusal::NONE && split == Rows{ "a **", "* day", "", "" },
				"as one text, so a word cut by the wrap is starred too");
		require(avatar_check(Rows{ long_line, long_line, "", "" }, keep) == Refusal::NONE,
				"but any length and level will do");

		require(!avatar_long_name(69) && avatar_long_name(70), "a name of 70 or wider takes the long plate");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
