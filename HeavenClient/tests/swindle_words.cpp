#include "../Util/SwindleWords.h"
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
		using ms::SwindleWords::contains;
		using ms::SwindleWords::match;
		using ms::SwindleWords::normalize;

		require(normalize("f.r-a_u d!") == "fraud", "ignored punctuation and spaces are dropped");
		require(normalize("a\tb\r\nc\x01") == "abc", "control characters are dropped");
		require(normalize("a?b") == "a?b", "other punctuation stays");

		require(contains("thisisFRAUD!", "fraud"), "case is ignored");
		require(contains("fraudster", "fraud"), "a word inside a longer one matches");
		require(!contains("frau", "fraud"), "a partial word doesn't match");
		require(!contains("buycash", "buy cash"), "a word with a space never matches");
		require(!contains("ab", "b"), "no match starts at the last character");
		require(contains("ab", "a"), "a match may start before the last character");
		require(!contains("abc", ""), "an empty word never matches");

		// From Etc/Swindle.img: 0 is fraud, 1 is party.
		std::vector<ms::SwindleWords::Group> groups {
			{ { "buy cash", "fraud" }, { "fraud warning" } },
			{ { "party member", "party" }, { "party note" } }
		};
		require(match(groups, "Is this a f r a u d?") == 0, "fraud");
		require(match(groups, "need party") == 1, "party");
		require(match(groups, "party fraud") == 1, "the last group is checked first");
		require(!match(groups, "buy cash please"), "nothing matches");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "swindle words OK" << std::endl;
	return 0;
}
