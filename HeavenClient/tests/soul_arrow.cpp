#include "../Gameplay/Combat/SoulArrow.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using namespace ms::soul_arrow;

	try
	{
		require(ball_skill(310) == 3101004, "a Ranger flies the Hunter's ball");
		require(ball_skill(321) == 3101004, "a Marksman flies the Hunter's ball too");
		require(ball_skill(1311) == 13101003, "a Wind Archer flies their own ball");
		require(is_ball(3101004) && is_ball(13101003), "both balls are Soul Arrow's");
		require(!is_ball(3201004) && !is_ball(2060000), "the Crossbowman's skill and arrows are not");

		std::cout << "Soul Arrow passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
