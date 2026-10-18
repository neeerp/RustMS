#include "../IO/Components/ContextMenuLayout.h"
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
		using namespace ms;
		using namespace context_menu;

		std::vector<Entry> base = { INFO, SAVE_NAME, TRADE, WHISPER, CLAIM };
		require(entries(false, false, false) == base, "five rows with no invites");

		std::vector<Entry> all = { INFO, SAVE_NAME, TRADE, WHISPER, PARTY, GUILD, CLAIM, FAMILY };
		require(entries(true, true, true) == all, "invites sit between whisper and alert GM");

		std::vector<Entry> guild_only = { INFO, SAVE_NAME, TRADE, WHISPER, GUILD, CLAIM };
		require(entries(false, true, false) == guild_only, "a missing row lets the rest move up");

		require(height(5) == 105, "five rows and the caps are 105 tall");

		require(place(Point<int16_t>(300, 200), 5, 800, 600) == Point<int16_t>(300, 200), "the menu opens at the cursor");
		require(place(Point<int16_t>(700, 495), 5, 800, 600) == Point<int16_t>(700, 495), "a menu that just fits stays");
		require(place(Point<int16_t>(750, 200), 5, 800, 600) == Point<int16_t>(700, 200), "past the right it moves left");
		require(place(Point<int16_t>(300, 550), 5, 800, 600) == Point<int16_t>(300, 495), "past the bottom it moves up");
		require(place(Point<int16_t>(1000, 700), 6, 1024, 768) == Point<int16_t>(924, 648), "a larger view keeps it inside");

		std::cout << "Context menu passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
