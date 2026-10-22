#include "../Character/Look/Stance.h"
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

		require(Stance::by_state(2) == Stance::WALK1 && Stance::by_state(3) == Stance::WALK1, "2 and 3 walk");
		require(Stance::by_state(4) == Stance::STAND1 && Stance::by_state(5) == Stance::STAND1, "4 and 5 stand");
		require(Stance::by_state(20) == Stance::SIT && Stance::by_state(21) == Stance::SIT, "20 and 21 sit");
		require(Stance::by_state(22) == Stance::WALK1 && Stance::by_state(23) == Stance::WALK1, "past sitting walks");
		require(Stance::by_state(0) == Stance::WALK1 && Stance::by_state(-4) == Stance::WALK1, "below walking walks");
		require(Stance::by_state(127) == Stance::WALK1, "far past sitting walks");
	} catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	return 0;
}
