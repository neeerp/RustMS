#include "../Gameplay/MapleMap/ObjFlow.h"
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
		using namespace ms::obj_flow;
		using Offset = std::pair<int64_t, int64_t>;

		// Aqua Road's fish/13: rx 10, so 100 px every 2000 ms, from 100 px to the left.
		require(offset(1, 10, 0, 0) == Offset(-100, 0), "starts behind");
		require(offset(1, 10, 0, 2000) == Offset(0, 0) && offset(1, 10, 0, 4000) == Offset(100, 0), "moves right");
		require(offset(2, 0, -15, 0) == Offset(0, 100) && offset(2, 0, -15, 1333) == Offset(0, 0), "moves up");
		require(offset(3, 20, 1, 3000) == Offset(200, 200), "flow 3 moves both ways at rx");
		require(offset(1, 350, 0, 57) == Offset(0, 0), "the period is whole ms");
		require(offset(1, 0, 0, 500) == Offset(0, 0), "no speed, no move");

		require(extent(800, 2600) == 5200, "two more periods");
		require(extent(800, -1800) == 0 && extent(800, -100) == 0 && extent(800, 0) == 0, "none past a negative period");

		auto fish = copies(1, 2600, 1000, 800, 600, 259, -261, 0, 0);
		require(fish == std::vector<std::pair<int32_t, int32_t>>({ { 259, -261 }, { -4941, -261 }, { -2341, -261 } }),
			"three copies wrapped around the view");
		require(copies(1, -1800, 1000, 800, 600, 259, -261, 0, 0).empty(), "a negative cx makes no copies");

		auto rising = copies(2, 1000, 5000, 800, 600, 10, 100, 0, 0);
		require(rising == std::vector<std::pair<int32_t, int32_t>>({ { 10, 100 }, { 10, -9900 }, { 10, -4900 } }),
			"copies along y");

		std::cout << "obj flow passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
