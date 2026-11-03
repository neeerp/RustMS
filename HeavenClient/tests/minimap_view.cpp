#include "../Gameplay/MapleMap/MiniMapView.h"
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
		using namespace ms::minimap_view;

		require(width(279) == 279 && width(280) == 210 && width(77) == 77, "the width's cap");
		require(height(149) == 149 && height(150) == 112 && height(112) == 112, "the height's cap");

		// Eos Tower 76th~90th Floor (221023400): a 77 x 683 canvas of a
		// 1234 x 10930 map, centre (594, 2662), mag 4.
		require(scroll(-207, 77, 77, 594, 1234, 4) == 0, "a narrow canvas doesn't scroll across");
		require(scroll(-1125, 112, 683, 2662, 10930, 4) == 40, "the view centres on the player");
		require(scroll(-2600, 112, 683, 2662, 10930, 4) == 0, "and stops at the top");
		require(scroll(8000, 112, 683, 2662, 10930, 4) == 571, "and at the bottom");
		require(scroll(0, 50, 50, 25, 49, 0) == 0, "a map smaller than its canvas stays on it");

		std::cout << "minimap view ok\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << "\n";
		return 1;
	}
}
