#include "../Gameplay/MapleMap/Portal.h"
#include <iostream>
#include <stdexcept>

// Portal::draw is the only Animation use in Portal.cpp; these tests never draw.
void ms::Animation::draw(const DrawArgument&, float) const {}

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

ms::Portal touch_portal(int16_t x, int16_t y, int16_t hrange = ms::Portal::DEFAULT_RANGE,
	int16_t vrange = ms::Portal::DEFAULT_RANGE)
{
	return ms::Portal(nullptr, ms::Portal::SCRIPTED_TOUCH, "tuto01", false, {x, y}, 999999999, "", hrange, vrange);
}

int main()
{
	try
	{
		// Map 10000's infoMinimap trigger sits 30 px above the floor at y = 485.
		ms::Portal minimap = touch_portal(280, 455);
		require(!minimap.touch({200, 485}), "touched the portal from 80 px away");
		require(minimap.touch({240, 485}), "walking along the floor did not touch the portal");
		require(!minimap.touch({280, 485}), "touched again without leaving the portal");
		require(!minimap.touch({340, 485}), "touched the portal after walking past it");
		require(minimap.touch({280, 485}), "re-entering did not touch the portal again");

		// The box is hRange by vRange around the portal, without its right and bottom edges.
		require(touch_portal(280, 455).touch({230, 405}), "the top-left corner is outside the box");
		require(touch_portal(280, 455).touch({329, 504}), "the bottom-right corner is outside the box");
		require(!touch_portal(280, 455).touch({330, 455}), "the right edge is inside the box");
		require(!touch_portal(280, 455).touch({280, 505}), "the bottom edge is inside the box");

		// Map 10000's exit warnings are 55 px above the floor and 65 px below the
		// platform over them, so neither reaches them.
		require(!touch_portal(987, 430).touch({987, 485}), "the exit warning fired from the floor");
		require(!touch_portal(987, 430).touch({987, 365}), "the exit warning fired from the platform");

		// A map can set its own ranges.
		require(touch_portal(0, 0, 40, 20).touch({19, 9}), "a custom range missed a point inside it");
		require(!touch_portal(0, 0, 40, 20).touch({20, 0}), "a custom range reached past hRange / 2");
		require(!touch_portal(0, 0, 40, 20).touch({0, 10}), "a custom range reached past vRange / 2");

		std::cout << "Portal touch ranges passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
