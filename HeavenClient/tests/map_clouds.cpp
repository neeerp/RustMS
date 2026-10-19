#include "../Gameplay/MapleMap/CloudLayout.h"
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
		using map_clouds::Area;

		// The physical space grown by 200 px on every side.
		Area area = map_clouds::area(-100, -500, 1000, 300);
		require(area.left == -300 && area.top == -700 && area.width == 1500 && area.height == 1200,
				"200 px around the physical space");

		// 1500 x 1200 is a fifth of 9,000,000 px², so 6 more than the 10 to
		// 29. Then per cloud: tile, drift, opacity, x, y.
		std::vector<int32_t> rolls = { 9, 123, 81, 1600, 50 };
		size_t calls = 0;
		auto scripted = [&]() { return calls++ == 0 ? 5 : rolls[(calls - 2) % rolls.size()]; };
		std::vector<MapCloud> clouds = map_clouds::scatter(area, 7, scripted);
		require(clouds.size() == 21, "5 + 6 + 10 clouds");
		require(calls == 1 + 21 * 5, "five rolls a cloud");
		MapCloud first{ 2, 141, -200, -650, 730, -9 };
		require(clouds.front() == first && clouds.back() == first,
				"tile 9 % 7, opacity 81 % 80 + 140, x and y from the area's corner, drift 23");

		// Every roll its highest, over an area of at least 9,000,000 px².
		auto highest = []() { return 32767; };
		clouds = map_clouds::scatter({ 0, 0, 4000, 3000 }, 7, highest);
		require(clouds.size() == 7 + 30 + 10, "30 more at most");
		MapCloud top{ 0, 187, 32767 % 4000, 32767 % 3000, 1170, 1 };
		require(clouds.front() == top, "the highest rolls");

		// The smallest field still has 10; none without canvases.
		auto lowest = []() { return 0; };
		require(map_clouds::scatter(map_clouds::area(0, 0, 0, 0), 7, lowest).size() == 10, "at least 10");
		require(map_clouds::scatter(area, 0, lowest).empty(), "no canvases, no clouds");

		// A cloud drifts by (dx, dy) every 30 s, truncating in between, and
		// wraps around the area.
		MapCloud cloud{ 0, 200, 100, 50, 730, -9 };
		auto at = [&](int64_t elapsed) { return map_clouds::position(cloud, elapsed, area); };
		require(at(0) == std::make_pair(100, 50), "where it starts");
		require(at(15000) == std::make_pair(465, 46), "half way: -4.5 is -4");
		require(at(30000) == std::make_pair(830, 41), "a whole drift");
		require(at(45000) == std::make_pair(1195, 37), "short of the right edge");
		require(at(60000) == std::make_pair(60, 32), "past it, back in from the left");
		require(at(86400000) == std::make_pair(1000, -670), "a day on");
		require(at(3000000000LL) == std::make_pair(1100, 50), "past 2^31 ms");
		cloud = { 0, 200, 100, -690, 730, -15 };
		require(at(60000) == std::make_pair(60, 480), "above the top, back in from the bottom");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
