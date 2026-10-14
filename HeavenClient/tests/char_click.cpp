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

		std::cout << "Character click passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
