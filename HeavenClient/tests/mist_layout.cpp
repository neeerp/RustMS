#include "../Gameplay/MapleMap/MistLayout.h"
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

		// With every roll 0, columns are 35 px apart, and a 50 px tile piles
		// 40 px below the last. A column's top and bottom are inset by half the
		// area's height times its distance from the centre over the width.
		std::vector<int32_t> asked;
		auto lowest = [&](int32_t n) { asked.push_back(n); return 0; };
		std::vector<MistCloud> clouds = layout_mist(0, 0, 100, 100, { 50 }, lowest);
		std::vector<MistCloud> want = {
			{ 0, 0, 25, 0, false }, { 0, 0, 65, 0, false },
			{ 0, 35, 7, 0, false }, { 0, 35, 47, 0, false }, { 0, 35, 87, 0, false },
			{ 0, 70, 10, 0, false }, { 0, 70, 50, 0, false }
		};
		require(clouds == want, "a diamond of columns");
		require(std::vector<int32_t>(asked.begin(), asked.begin() + 7) == std::vector<int32_t>{ 1, 10, 2, 1, 10, 2, 20 },
				"tile, order and flip per cloud, then the column's step");

		// With every roll its highest, the tall tile, the top order, flipped,
		// and columns 54 px apart.
		auto highest = [](int32_t n) { return n - 1; };
		clouds = layout_mist(0, 0, 100, 100, { 50, 100 }, highest);
		want = { { 1, 0, 25, 9, true }, { 1, 54, 2, 9, true }, { 1, 54, 82, 9, true } };
		require(clouds == want, "the highest rolls");

		// Coordinates truncate toward zero, as the client's ftol does.
		clouds = layout_mist(-200, -150, 200, 150, { 95 }, lowest);
		require(clouds.front().x == -200 && clouds.front().y == -75, "the first cloud at the left edge's top");
		require(clouds[1].y == 1, "-75 + 76 is the next");
		require(layout_mist(0, 0, 0, 100, { 50 }, lowest).empty(), "an empty area has no clouds");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
