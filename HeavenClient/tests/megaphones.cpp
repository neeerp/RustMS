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
		require(kind(5075005) == Kind::NONE, "kind 5 below 6 is Maple TV");
		require(kind(5077000) == Kind::TRIPLE, "kind 7 is the triple megaphone");
		require(kind(5073000) == Kind::NONE && kind(5370000) == Kind::NONE, "nothing else is asked for");

		auto keep = [](const std::string& text) { return text; };
		auto star = [](const std::string& text) { return std::string(text.size(), '*'); };
		const std::string long_line(61, 'a');

		require(check_line("", 5, star) == Refusal::QUIET, "an empty line goes before anything else");
		require(check_line("hi", 5, star) == Refusal::CURSING, "then cursing");
		require(check_line("hi", 10, keep) == Refusal::LEVEL, "then level 10 and below");
		require(check_line(long_line, 11, keep) == Refusal::LONG, "then over 60");
		require(check_line(std::string(81, 'a'), 11, keep) == Refusal::QUIET, "and over 80 says nothing");
		require(check_line(std::string(60, 'a'), 11, keep) == Refusal::NONE, "60 at level 11 goes");

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
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
