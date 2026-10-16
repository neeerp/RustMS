#include "../Util/CurseWords.h"
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
		using ms::CurseWords::filter;
		using ms::CurseWords::find;
		using ms::CurseWords::printable;

		require(find("xxFOOxx", "foo") == 2, "case is ignored");
		require(find("fo", "foo") == std::string::npos, "a partial word doesn't match");
		require(find("ab", "b") == std::string::npos, "no match starts at the last character");
		require(find("abc", "") == std::string::npos, "an empty word never matches");

		std::vector<std::string> words { "foo", "bar" };

		require(filter("hello", words) == "hello", "a clean message stays");
		require(filter("Foo you", words) == "*** you", "a word is starred out");
		require(filter("foofoo", words) == "******", "every occurrence is starred out");
		require(filter("f.o-o!", words) == "*.*-*!", "skipped characters stay where they were");
		require(filter("ba r", words) == "** *", "a word may span a space");
		require(filter("seafood", words) == "sea***d", "a word inside a longer one is starred out");
		require(filter("foo\x01", words) == "foo\x01", "a message with control characters stays");
		require(filter(std::string(0x3ff, 'a') + "foo", words) == std::string(0x3ff, 'a') + "foo",
				"a long message stays");

		// Earlier words go first, so a later, longer one may no longer match.
		require(filter("foobar", { "foo", "foobar" }) == "***bar", "words are applied in order");
		require(filter("a*b", { "*b" }) == "a*b", "a word with a star is skipped");

		require(printable("caf\xc3\xa9\t!") == "caf   !", "characters outside space to tilde become spaces");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "curse words OK" << std::endl;
	return 0;
}
