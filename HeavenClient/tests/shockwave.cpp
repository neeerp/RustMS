#include "../Gameplay/Combat/Shockwave.h"
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
		using shockwave::far_side;
		using shockwave::steps;

		// A flat floor from x 0 to `end`, at y 100.
		auto floor_to = [](int32_t end)
		{
			return [end](int32_t x, int32_t y, int32_t reach)
			{
				return x >= 0 && x <= end && 100 >= y && 100 < y + reach;
			};
		};
		auto over_to = [](int32_t end)
		{
			return [end](int32_t x, int32_t y, int32_t reach)
			{
				return x >= 0 && x <= end && 100 <= y && 100 > y - reach;
			};
		};

		require(steps(0, 100, false, floor_to(1000), over_to(1000)) == 5, "a long floor, all five steps");
		require(steps(0, 100, false, floor_to(60), over_to(60)) == 2, "a step past the floor's end");
		require(steps(0, 100, false, floor_to(10), over_to(10)) == 1, "the last spot still has its floor");
		require(steps(0, 100, true, floor_to(1000), over_to(1000)) == 1, "the floor ends behind the user");
		require(steps(0, 40, false, floor_to(1000), over_to(1000)) == 0, "too high above it, no wave");
		require(steps(0, 71, false, floor_to(1000), over_to(1000)) == 5, "just above it, a wave");
		require(steps(-10, 71, false, floor_to(1000), over_to(1000)) == 5, "the floor just ahead counts");
		require(steps(-200, 71, false, floor_to(1000), over_to(1000)) == 0, "out over nothing");

		require(far_side(1000, 5, true) == 695 && far_side(1000, 5, false) == 1305, "305 of the area's 330");
		require(far_side(1000, 1, false) == 1085, "30 past the wave");
		require(shockwave::is(5111006) && shockwave::is(15111003) && !shockwave::is(5111005), "both classes' Shockwave");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
