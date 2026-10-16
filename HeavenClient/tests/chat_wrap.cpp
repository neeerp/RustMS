#include "../Util/ChatWrap.h"
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
		using ms::ChatWrap::wrap;

		// Every character is as wide, and ten fit on a row.
		auto ten = [](const std::string& text) { return std::min<size_t>(10, text.size()); };

		require(wrap("short", true, ten) == std::vector<std::string>{ "short" }, "a short line is one row");
		require(wrap("", true, ten).empty(), "an empty line has no rows");
		require(wrap("   ", true, ten).empty(), "a blank line has no rows");

		require(wrap("0123456789abcdefgh", true, ten) == std::vector<std::string>{ "0123456789", "     abcde", "     fgh" },
				"rows after the first are indented");
		require(wrap("0123456789abcdefgh", false, ten) == std::vector<std::string>{ "0123456789", "abcdefgh" },
				"some lines aren't indented");
		require(wrap("0123456789   ", true, ten) == std::vector<std::string>{ "0123456789" },
				"trailing whitespace doesn't make a row");

		auto none = [](const std::string&) { return size_t(0); };

		require(wrap("abc", false, none) == std::vector<std::string>{ "a", "b", "c" },
				"a row holds at least one character");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "chat wrap OK" << std::endl;
	return 0;
}
