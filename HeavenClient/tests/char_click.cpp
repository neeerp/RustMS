#include "../Gameplay/MapleMap/CharClick.h"
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
		using char_click::hits;

		Point<int16_t> feet(400, 300);

		require(hits(feet, Point<int16_t>(400, 270)), "the middle of the body is a hit");
		require(hits(feet, Point<int16_t>(382, 300)), "the left edge at the feet is a hit");
		require(hits(feet, Point<int16_t>(418, 230)), "the right edge at the head is a hit");
		require(!hits(feet, Point<int16_t>(419, 270)), "beside the body is a miss");
		require(!hits(feet, Point<int16_t>(400, 301)), "below the feet is a miss");
		require(!hits(feet, Point<int16_t>(400, 229)), "above the head is a miss");

		using char_click::menu_hits;

		require(menu_hits(feet, false, false, Point<int16_t>(378, 235)), "the menu box's top-left corner is in");
		require(!menu_hits(feet, false, false, Point<int16_t>(422, 270)), "the menu box's right edge is out");
		require(!menu_hits(feet, false, false, Point<int16_t>(400, 300)), "the menu box's bottom edge is out");
		require(!menu_hits(feet, false, false, Point<int16_t>(400, 234)), "above the menu box is a miss");
		require(menu_hits(feet, false, true, Point<int16_t>(421, 299)), "facing right, standing, the box is the same");
		require(menu_hits(feet, true, false, Point<int16_t>(354, 269)), "prone facing left, the box reaches left");
		require(!menu_hits(feet, true, false, Point<int16_t>(400, 280)), "prone facing left, the box stops at the feet");
		require(!menu_hits(feet, true, false, Point<int16_t>(380, 268)), "prone, above the body is a miss");
		require(menu_hits(feet, true, true, Point<int16_t>(400, 280)), "prone facing right, the box starts at the feet");
		require(menu_hits(feet, true, true, Point<int16_t>(445, 280)), "prone facing right, the box reaches right");
		require(!menu_hits(feet, true, true, Point<int16_t>(399, 280)), "prone facing right, behind the feet is a miss");

		std::cout << "Character click passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
