#include "../IO/Components/MenuCover.h"
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
		using menu_cover::covered;

		// The game menu as it opens over the icon row (#135), with QUIT GAME
		// and the Stat icon overlapping at its bottom.
		Rectangle<int16_t> menu(Point<int16_t>(666, 425), Point<int16_t>(759, 565));
		Rectangle<int16_t> quit(Point<int16_t>(672, 525), Point<int16_t>(753, 549));
		Rectangle<int16_t> stat(Point<int16_t>(686, 537), Point<int16_t>(710, 557));

		auto takes = [&](Rectangle<int16_t> bounds, bool menu_button, bool open, Point<int16_t> cursor)
		{
			return bounds.contains(cursor) && !covered(open, menu, menu_button, cursor);
		};

		Point<int16_t> over_both(700, 546);
		require(takes(quit, true, true, over_both), "QUIT GAME takes a click over the Stat icon");
		require(!takes(stat, false, true, over_both), "the Stat icon under QUIT GAME is covered");

		Point<int16_t> frame(688, 556);
		require(!takes(quit, true, true, frame), "the menu frame is not QUIT GAME");
		require(!takes(stat, false, true, frame), "the Stat icon under the menu frame is covered");

		require(takes(stat, false, false, frame), "with the menu closed the Stat icon takes the click");
		require(!covered(true, menu, false, Point<int16_t>(700, 570)), "below the menu nothing is covered");

		std::cout << "Menu cover passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
