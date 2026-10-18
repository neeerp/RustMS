#include "../Util/Chalkboard.h"
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
		using namespace ms::Chalkboard;

		require(trim(" \tselling\r\n") == "selling", "a note is trimmed");
		require(trim(" \n ").empty(), "a blank note is empty");

		auto mask = [](const std::string& text)
		{
			std::string masked = text;
			size_t pos = masked.find("bad");

			if (pos != std::string::npos)
				masked.replace(pos, 3, "***");

			return masked;
		};

		require(remote_text("  bad deal ", mask) == "*** deal", "a remote note is trimmed and masked");
		require(remote_text(std::string(45, 'a'), mask) == std::string(40, 'a'), "a remote note is cut to 40");

		// Ten pixels per character, so a 196 px line holds 19 characters.
		auto width = [](const std::string& text) { return static_cast<int16_t>(text.size() * 10); };

		require(wrap("selling", width) == std::vector<std::string>{ "selling" }, "a short note is one line");
		require(wrap("abcdefghijklmnopqrstuvwxyz", width)
				== std::vector<std::string>{ "abcdefghijklmnopqrs", "tuvwxyz" },
				"a long note wraps by character");
		require(wrap("abcdefghijklmnopq   rstuvwxyz", width)
				== std::vector<std::string>{ "abcdefghijklmnopq", "rstuvwxyz" },
				"a line loses its trailing blanks and the next its leading ones");
		require(wrap("", width).empty(), "an empty note has no lines");
		require(wrap("abc", [](const std::string&) { return int16_t(500); })
				== std::vector<std::string>{ "a", "b", "c" },
				"a line holds at least one character");

		require(tiles(1, 1) == 1, "a tiny note gets one tile");
		require(tiles(1, 14) == 1, "a full tile is one tile");
		require(tiles(1, 15) == 3, "a single line gets an odd tile count");
		require(tiles(1, 42) == 3, "three full tiles stay three");
		require(tiles(1, 196) == 15, "even a full line rounds up to an odd count");
		require(tiles(2, 30) == 14, "a longer note uses fourteen tiles");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
