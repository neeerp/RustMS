#include "../IO/Components/MapTooltipLayout.h"
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
		using namespace ms::map_tooltip;

		// Southperry right after Amherst is the same as Southperry alone.
		Size alone = size(120, 18, 150, 36);
		size(140, 18, 200, 54);
		Size after = size(120, 18, 150, 36);
		require(alone.width == after.width && alone.height == after.height, "showing another map first changes nothing");

		require(alone.width == 150 + 2 * PADDING_X, "a wider description widens the box");
		require(alone.height == 18 + DESC_GAP + 36 + 2 * PADDING_Y, "the name, then the description");
		require(size(211, 18, 0, 0).width == 211 + 2 * PADDING_X, "a name alone sets the width");
		require(size(211, 18, 0, 0).height == 18 + 2 * PADDING_Y, "a name alone is one line tall");
		require(wrap_width(80) == DESC_WIDTH && wrap_width(260) == 260, "descriptions wrap at least as wide as the name");

		std::cout << "Map tooltip layout passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
