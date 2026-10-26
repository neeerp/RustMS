#include "../Gameplay/SightRules.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool is(const ms::sight::Strip& strip, int16_t left, int16_t top, int16_t width, int16_t height)
{
	return strip.left == left && strip.top == top && strip.width == width && strip.height == height;
}

int main()
{
	try
	{
		using namespace ms::sight;

		require(limits_view(9), "fieldType 9 is a LimitedView");
		require(!limits_view(0) && !limits_view(2) && !limits_view(10), "no other type is");

		require(LIMITED.x == -163 && LIMITED.y == -190, "a LimitedView's sight sits up and left of the feet");
		require(FEAR.x == -80 && FEAR.y == -120, "Fear's too");

		// Feet at the middle of an 800x600 screen: the sight's square is
		// at (237, 110), and black fills the rest.
		auto around = strips(237, 110, 316, 316, 800, 600);
		require(is(around[0], 0, 0, 800, 110), "above");
		require(is(around[1], 0, 426, 800, 174), "below");
		require(is(around[2], 0, 110, 237, 316), "left");
		require(is(around[3], 553, 110, 247, 316), "right");

		// Near the screen's top left corner the sight hangs off it, and
		// the strips it pushes off are empty.
		auto corner = strips(-100, -50, 316, 316, 1024, 768);
		require(is(corner[0], 0, 0, 1024, 0), "nothing above");
		require(is(corner[1], 0, 266, 1024, 502), "below on a larger screen");
		require(is(corner[2], 0, 0, 0, 266), "nothing left");
		require(is(corner[3], 216, 0, 808, 266), "right");

		// Wholly off the screen it leaves it all black.
		auto gone = strips(900, 700, 316, 316, 800, 600);
		require(is(gone[0], 0, 0, 800, 600), "all black");
		require(gone[1].height == 0 && gone[2].height == 0 && gone[3].height == 0, "and nothing else");

		std::cout << "sight_rules: all passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << "sight_rules: " << error.what() << "\n";
		return 1;
	}
}
