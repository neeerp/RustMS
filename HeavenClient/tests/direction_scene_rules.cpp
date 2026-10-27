#include "../Gameplay/DirectionSceneRules.h"
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
		using namespace ms::direction;

		// The Mushroom Kingdom's opening: back1 slides from (0, 0) to (232, 0) over 1500 ms.
		Event slide;
		slide.x1 = 232;
		slide.duration = 1500;
		require(position(slide, 0).x == 0 && position(slide, 0).y == 0, "starts where it is");
		require(position(slide, 750).x == 116 && position(slide, 750).y == 0, "halfway at half the time");
		require(position(slide, 1500).x == 232 && position(slide, 9000).x == 232, "then stays");

		// Only an x1 or y1 moves it; a duration alone doesn't.
		Event still;
		still.x = -40;
		still.y = 25;
		still.duration = 1500;
		require(position(still, 1000).x == -40 && position(still, 1000).y == 25, "no x1 or y1, no move");

		// A move without a duration is there at once.
		Event jump;
		jump.x1 = 10;
		jump.y1 = -20;
		require(position(jump, 0).x == 10 && position(jump, 0).y == -20, "no duration, no slide");

		Event up;
		up.y = 100;
		up.y1 = -100;
		up.duration = 400;
		require(position(up, 100).y == 50 && position(up, 100).x == 0, "y moves too");

		std::cout << "Direction scene rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
