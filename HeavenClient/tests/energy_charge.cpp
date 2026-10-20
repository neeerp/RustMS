#include "../Gameplay/Combat/EnergyCharge.h"

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
		using namespace ms::energy;

		require(skill(511) == 5110001 && skill(1511) == 15100004, "each line has its own Energy Charge");

		require(needs_full(5111002) && needs_full(5111004) && needs_full(5121002), "the Energy skills wait");
		require(needs_full(15101005) && needs_full(15111001), "and the Thunder Breaker's");
		require(!needs_full(5110001) && !needs_full(5101002) && !needs_full(0), "the rest don't");

		// Level 40 lends 20 of each.
		require(bonus(0, 20) == 20, "with no buff the skill's stat counts");
		require(bonus(12, 20) == 8, "a smaller buff is topped up");
		require(bonus(25, 20) == 0, "a larger buff stays");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
